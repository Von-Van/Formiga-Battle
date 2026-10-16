//! The relay over HTTP: the client Battle uses, and a small local relay serving the reference
//! state machine for development and tests. The relay players use is the one on the Formiga site;
//! its address is built into Battle and never shown or asked for.
//!
//! | Request | Body | Answer |
//! |---|---|---|
//! | `POST {base}/rooms` | an `Entrant` | 201 `{code, seat}` |
//! | `POST {base}/rooms/{code}/join` | an `Entrant` | 201 `{code, seat}` |
//! | `GET {base}/rooms/{code}` | | 200 a `View` |
//! | `POST {base}/rooms/{code}/orders` | `{turn, orders}` | 200 a `View` |
//! | `POST {base}/rooms/{code}/board` | `{turn, hash, finished}` | 200 a `View` |
//! | `DELETE {base}/rooms/{code}` | | 200 `{}` |
//!
//! Every request after opening or joining carries the seat in the `X-Formiga-Seat` header. A
//! refusal comes with its status and `{"refused": …}`.

use crate::relay::{
    Answer, Entrant, MAX_BODY_BYTES, Millis, RELAY_VERSION, Refusal, Relay, Request, SEAT_HEADER,
    View, hex_u64,
};
use formiga_battle_core::Orders;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Where friend matches go.
pub const RELAY_BASE: &str = "https://www.formigaworld.com/api/battle";
/// For development only: another relay, such as `http://127.0.0.1:7878/api/battle`.
pub const RELAY_ENV: &str = "FORMIGA_BATTLE_RELAY";

/// The relay Battle talks to: the site's, unless development points elsewhere.
pub fn relay_base() -> String {
    std::env::var(RELAY_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| RELAY_BASE.to_owned())
}

#[derive(Serialize, Deserialize)]
struct OrdersBody {
    turn: u8,
    orders: Orders,
}

#[derive(Serialize, Deserialize)]
struct BoardBody {
    turn: u8,
    #[serde(with = "hex_u64")]
    hash: u64,
    finished: bool,
}

#[derive(Serialize, Deserialize)]
struct SeatBody {
    code: String,
    seat: String,
}

/// What went wrong asking the relay.
#[derive(Debug, thiserror::Error)]
pub enum AskError {
    #[error("{}", .0.words())]
    Refused(Refusal),
    #[error("Battle could not reach friend matches: {0}")]
    Unreachable(String),
    #[error("the relay answered something Battle could not read")]
    Unreadable,
}

/// One Handler's way to the relay.
#[derive(Clone)]
pub struct RelayClient {
    base: String,
    agent: ureq::Agent,
}

impl RelayClient {
    pub fn new(base: &str) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .http_status_as_error(false)
            .build();
        Self {
            base: base.trim_end_matches('/').to_owned(),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    fn ask<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        seat: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<T, AskError> {
        let url = format!("{}{path}", self.base);
        let unreachable = |e: ureq::Error| AskError::Unreachable(e.to_string());
        let response = match method {
            "GET" => {
                let mut r = self.agent.get(&url);
                if let Some(seat) = seat {
                    r = r.header(SEAT_HEADER, seat);
                }
                r.call().map_err(unreachable)?
            }
            "DELETE" => {
                let mut r = self.agent.delete(&url);
                if let Some(seat) = seat {
                    r = r.header(SEAT_HEADER, seat);
                }
                r.call().map_err(unreachable)?
            }
            _ => {
                let mut r = self
                    .agent
                    .post(&url)
                    .header("Content-Type", "application/json");
                if let Some(seat) = seat {
                    r = r.header(SEAT_HEADER, seat);
                }
                r.send(&body.unwrap_or_default()[..]).map_err(unreachable)?
            }
        };
        let status = response.status().as_u16();
        let bytes = response
            .into_body()
            .with_config()
            .limit(256 * 1024)
            .read_to_vec()
            .map_err(|e| AskError::Unreachable(e.to_string()))?;
        if !(200..300).contains(&status) {
            return Err(match serde_json::from_slice::<Refusal>(&bytes) {
                Ok(refusal) => AskError::Refused(refusal),
                Err(_) => AskError::Unreachable(format!("the relay answered {status}")),
            });
        }
        serde_json::from_slice(&bytes).map_err(|_| AskError::Unreadable)
    }

    /// Opens a room. Gives its code and this side's seat.
    pub fn open(&self, entrant: &Entrant) -> Result<(String, String), AskError> {
        let body = serde_json::to_vec(entrant).map_err(|_| AskError::Unreadable)?;
        let seat: SeatBody = self.ask("POST", "/rooms", None, Some(body))?;
        Ok((seat.code, seat.seat))
    }

    /// Joins a friend's room by its code.
    pub fn join(&self, code: &str, entrant: &Entrant) -> Result<(String, String), AskError> {
        let code = crate::code::normalize(code).ok_or(AskError::Refused(Refusal::NoSuchRoom))?;
        let body = serde_json::to_vec(entrant).map_err(|_| AskError::Unreadable)?;
        let seat: SeatBody = self.ask("POST", &format!("/rooms/{code}/join"), None, Some(body))?;
        Ok((seat.code, seat.seat))
    }

    pub fn look(&self, code: &str, seat: &str) -> Result<View, AskError> {
        self.ask("GET", &format!("/rooms/{code}"), Some(seat), None)
    }

    pub fn orders(
        &self,
        code: &str,
        seat: &str,
        turn: u8,
        orders: Orders,
    ) -> Result<View, AskError> {
        let body =
            serde_json::to_vec(&OrdersBody { turn, orders }).map_err(|_| AskError::Unreadable)?;
        self.ask(
            "POST",
            &format!("/rooms/{code}/orders"),
            Some(seat),
            Some(body),
        )
    }

    pub fn board(
        &self,
        code: &str,
        seat: &str,
        turn: u8,
        hash: u64,
        finished: bool,
    ) -> Result<View, AskError> {
        let body = serde_json::to_vec(&BoardBody {
            turn,
            hash,
            finished,
        })
        .map_err(|_| AskError::Unreadable)?;
        self.ask(
            "POST",
            &format!("/rooms/{code}/board"),
            Some(seat),
            Some(body),
        )
    }

    pub fn leave(&self, code: &str, seat: &str) -> Result<(), AskError> {
        let _: serde_json::Value =
            self.ask("DELETE", &format!("/rooms/{code}"), Some(seat), None)?;
        Ok(())
    }
}

/// A running local relay.
pub struct LocalRelay {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl LocalRelay {
    /// The base address to give `RelayClient::new` or `FORMIGA_BATTLE_RELAY`.
    pub fn base(&self) -> String {
        format!("http://{}/api/battle", self.addr)
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

/// Starts a local relay on `bind`, serving the reference state machine at `/api/battle`. Port 0
/// picks a free one.
pub fn serve_relay(
    bind: impl ToSocketAddrs,
    limits: crate::relay::Limits,
) -> io::Result<LocalRelay> {
    let listener = TcpListener::bind(bind)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let relay = Arc::new(Mutex::new(Relay::new(limits)));
    let started = Instant::now();
    let accept_stop = stop.clone();
    let accept = std::thread::spawn(move || {
        for stream in listener.incoming() {
            if accept_stop.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = stream else {
                continue;
            };
            let relay = relay.clone();
            std::thread::spawn(move || {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let now = started.elapsed().as_millis() as Millis;
                let _ = serve_one(stream, &relay, now);
            });
        }
    });
    Ok(LocalRelay {
        addr,
        stop,
        accept: Some(accept),
    })
}

/// The most of an oversized body the local relay reads and throws away before refusing it.
const DRAIN_BYTES: usize = 1024 * 1024;

fn serve_one(stream: TcpStream, relay: &Mutex<Relay>, now: Millis) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let (mut length, mut seat) = (0usize, String::new());
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            if name == "content-length" {
                length = value.parse().unwrap_or(usize::MAX);
            } else if name == SEAT_HEADER.to_ascii_lowercase() {
                seat = value.to_owned();
            }
        }
    }
    let answer = if length > MAX_BODY_BYTES {
        // Read what was sent, up to a limit, before answering: closing with it unread would
        // reset the connection, and the client would never see why.
        let _ = io::copy(
            &mut reader.by_ref().take(length.min(DRAIN_BYTES) as u64),
            &mut io::sink(),
        );
        Answer::Refused(Refusal::Malformed)
    } else {
        let mut body = vec![0; length];
        reader.read_exact(&mut body)?;
        match route(method, path, seat, &body) {
            Some(request) => relay
                .lock()
                .map_or(Answer::Refused(Refusal::Busy), |mut r| {
                    r.handle(request, now)
                }),
            None => Answer::Refused(another_protocol(&body).unwrap_or(Refusal::Malformed)),
        }
    };
    let body = serde_json::to_vec(&answer).unwrap_or_default();
    let mut stream = stream;
    write!(
        stream,
        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        answer.status(),
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()
}

/// The request a method and path name, with its body read.
/// An entrant from a Battle that speaks another version of the relay: refused as a version, not
/// as malformed, whatever else it holds, so an older Battle is told to update.
fn another_protocol(body: &[u8]) -> Option<Refusal> {
    #[derive(Deserialize)]
    struct Spoken {
        protocol: u32,
        #[serde(default)]
        rules_version: u32,
    }
    let spoken: Spoken = serde_json::from_slice(body).ok()?;
    (spoken.protocol != RELAY_VERSION).then_some(Refusal::Version {
        relay: RELAY_VERSION,
        rules_version: spoken.rules_version,
    })
}

fn route(method: &str, path: &str, seat: String, body: &[u8]) -> Option<Request> {
    let rest = path.strip_prefix("/api/battle")?;
    let parts: Vec<&str> = rest.trim_matches('/').split('/').collect();
    match (method, parts.as_slice()) {
        ("POST", ["rooms"]) => Some(Request::Open {
            entrant: serde_json::from_slice(body).ok()?,
        }),
        ("POST", ["rooms", code, "join"]) => Some(Request::Join {
            code: (*code).to_owned(),
            entrant: serde_json::from_slice(body).ok()?,
        }),
        ("GET", ["rooms", code]) => Some(Request::Look {
            code: (*code).to_owned(),
            seat,
        }),
        ("POST", ["rooms", code, "orders"]) => {
            let b: OrdersBody = serde_json::from_slice(body).ok()?;
            Some(Request::Orders {
                code: (*code).to_owned(),
                seat,
                turn: b.turn,
                orders: b.orders,
            })
        }
        ("POST", ["rooms", code, "board"]) => {
            let b: BoardBody = serde_json::from_slice(body).ok()?;
            Some(Request::Board {
                code: (*code).to_owned(),
                seat,
                turn: b.turn,
                hash: b.hash,
                finished: b.finished,
            })
        }
        ("DELETE", ["rooms", code]) => Some(Request::Leave {
            code: (*code).to_owned(),
            seat,
        }),
        _ => None,
    }
}
