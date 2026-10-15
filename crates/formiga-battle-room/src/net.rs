//! The service on sockets, and the client end, with plain threads: no async runtime, as in
//! Desktop. One JSON message a line. Every line is bounded before it is parsed, and a connection
//! that sends one too long, or nonsense, is closed.
//!
//! The connection itself is plain TCP, for a room on this computer or the local network. A room
//! on the internet goes behind a TLS proxy; nothing here changes for that.

use crate::protocol::*;
use crate::service::{ConnId, Limits, Millis, Service};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Reads one line of at most `max` bytes. `None` at the end of the stream.
fn read_line_bounded(reader: &mut impl BufRead, max: usize) -> io::Result<Option<String>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if line.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&line).into_owned())
            });
        }
        if let Some(end) = available.iter().position(|b| *b == b'\n') {
            line.extend_from_slice(&available[..end]);
            reader.consume(end + 1);
            if line.len() > max {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "a line too long",
                ));
            }
            return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
        }
        let n = available.len();
        line.extend_from_slice(available);
        reader.consume(n);
        if line.len() > max {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "a line too long",
            ));
        }
    }
}

fn write_line<T: serde::Serialize>(stream: &mut impl Write, msg: &T) -> io::Result<()> {
    let mut line = serde_json::to_vec(msg).map_err(io::Error::other)?;
    line.push(b'\n');
    stream.write_all(&line)?;
    stream.flush()
}

enum Event {
    Opened(ConnId, Sender<ServerMsg>),
    Said(ConnId, ClientMsg),
    Closed(ConnId),
}

/// A running room service.
pub struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Server {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stops taking connections and closes every room.
    pub fn stop(self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop.
        let _ = TcpStream::connect(self.addr);
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

/// Starts a room service on `bind`. Port 0 picks a free one; `Server::addr` says which.
pub fn serve(bind: impl ToSocketAddrs, limits: Limits) -> io::Result<Server> {
    let listener = TcpListener::bind(bind)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let (events, inbox) = mpsc::channel::<Event>();

    let service_stop = stop.clone();
    let service = std::thread::spawn(move || run_service(inbox, limits, service_stop));

    let accept_stop = stop.clone();
    let accept = std::thread::spawn(move || {
        let mut next: ConnId = 1;
        for stream in listener.incoming() {
            if accept_stop.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = stream else {
                continue;
            };
            let conn = next;
            next += 1;
            let _ = stream.set_nodelay(true);
            let Ok(write_half) = stream.try_clone() else {
                continue;
            };
            let (to_client, outbox) = mpsc::channel::<ServerMsg>();
            if events.send(Event::Opened(conn, to_client)).is_err() {
                break;
            }
            std::thread::spawn(move || {
                let mut stream = write_half;
                for msg in outbox {
                    if write_line(&mut stream, &msg).is_err() {
                        break;
                    }
                }
                let _ = stream.shutdown(std::net::Shutdown::Both);
            });
            let events = events.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream);
                loop {
                    match read_line_bounded(&mut reader, MAX_LINE_BYTES) {
                        Ok(Some(line)) if line.trim().is_empty() => continue,
                        Ok(Some(line)) => match serde_json::from_str::<ClientMsg>(&line) {
                            Ok(msg) => {
                                if events.send(Event::Said(conn, msg)).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        },
                        _ => break,
                    }
                }
                let _ = events.send(Event::Closed(conn));
            });
        }
    });
    Ok(Server {
        addr,
        stop,
        threads: vec![accept, service],
    })
}

fn run_service(inbox: Receiver<Event>, limits: Limits, stop: Arc<AtomicBool>) {
    let mut service = Service::new(limits);
    let mut outboxes: std::collections::BTreeMap<ConnId, Sender<ServerMsg>> = Default::default();
    let started = Instant::now();
    let now = || started.elapsed().as_millis() as Millis;
    let mut last_rooms = 0;
    while !stop.load(Ordering::SeqCst) {
        let out = match inbox.recv_timeout(Duration::from_millis(200)) {
            Ok(Event::Opened(conn, outbox)) => {
                service.connected(conn);
                outboxes.insert(conn, outbox);
                Vec::new()
            }
            Ok(Event::Said(conn, msg)) => service.message(conn, msg, now()),
            Ok(Event::Closed(conn)) => {
                outboxes.remove(&conn);
                service.disconnected(conn, now())
            }
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let out = out.into_iter().chain(service.tick(now()));
        for (conn, msg) in out {
            if let Some(outbox) = outboxes.get(&conn) {
                let _ = outbox.send(msg);
            }
        }
        // Only counts are ever logged: never a code, a token or anyone's companions.
        if service.rooms() != last_rooms {
            last_rooms = service.rooms();
            eprintln!("formiga-battle room: {last_rooms} rooms open");
        }
    }
}

/// What the client end hears.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Msg(Box<ServerMsg>),
    /// The connection is gone, and why.
    Closed(String),
}

/// One Handler's connection to a room service.
pub struct Client {
    writer: Arc<Mutex<TcpStream>>,
    heard: Receiver<Heard>,
}

impl Client {
    /// Connects and says hello. The service's answer comes as the first message heard.
    pub fn connect(addr: &str, battle_version: &str) -> io::Result<Self> {
        let target = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such address"))?;
        let stream = TcpStream::connect_timeout(&target, Duration::from_secs(5))?;
        let _ = stream.set_nodelay(true);
        let reader = stream.try_clone()?;
        let (tell, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            loop {
                match read_line_bounded(&mut reader, MAX_LINE_BYTES) {
                    Ok(Some(line)) if line.trim().is_empty() => continue,
                    Ok(Some(line)) => match serde_json::from_str::<ServerMsg>(&line) {
                        Ok(msg) => {
                            if tell.send(Heard::Msg(Box::new(msg))).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tell.send(Heard::Closed(format!(
                                "the room said something unreadable: {error}"
                            )));
                            return;
                        }
                    },
                    Ok(None) => {
                        let _ =
                            tell.send(Heard::Closed("the room closed the connection".to_owned()));
                        return;
                    }
                    Err(error) => {
                        let _ =
                            tell.send(Heard::Closed(format!("the connection was lost: {error}")));
                        return;
                    }
                }
            }
        });
        let client = Self {
            writer: Arc::new(Mutex::new(stream)),
            heard,
        };
        client.send(&ClientMsg::Hello {
            protocol: PROTOCOL_VERSION,
            rules_version: formiga_battle_core::RULES_VERSION,
            battle_version: battle_version.to_owned(),
        })?;
        Ok(client)
    }

    pub fn send(&self, msg: &ClientMsg) -> io::Result<()> {
        let mut stream = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("the connection broke"))?;
        write_line(&mut *stream, msg)
    }

    /// Anything heard since last asked, without waiting.
    pub fn try_hear(&self) -> Option<Heard> {
        self.heard.try_recv().ok()
    }

    /// The next thing heard, waiting at most `wait`.
    pub fn hear(&self, wait: Duration) -> Option<Heard> {
        self.heard.recv_timeout(wait).ok()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Ok(stream) = self.writer.lock() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}
