//! The line to the relay during a match with a friend, on a thread of its own so the window never
//! waits on the network. The window asks it to send orders or a board; it answers with every
//! view of the room it gets, and asks the relay for one about once a second.

use formiga_battle_core::Orders;
use formiga_battle_room::{AskError, Entrant, RelayClient, View};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// How often the room is looked at while nothing else is said.
const POLL: Duration = Duration::from_millis(900);

pub enum Ask {
    Orders { turn: u8, orders: Orders },
    Board { turn: u8, hash: u64, finished: bool },
    Leave,
}

pub enum Told {
    /// A room is open, or joined, under this code.
    Seated {
        code: String,
    },
    View(Box<View>),
    /// Something went wrong; the line keeps trying unless the room is gone.
    Problem(String),
    /// The room is gone, or was never there.
    Gone(String),
}

pub struct Line {
    asks: Sender<Ask>,
    told: Receiver<Told>,
}

impl Line {
    /// Opens a room for a friend (`code` is `None`), or joins one, and keeps the line open.
    pub fn start(base: String, entrant: Entrant, code: Option<String>) -> Self {
        let (asks, asked) = mpsc::channel();
        let (tell, told) = mpsc::channel();
        std::thread::spawn(move || run(base, entrant, code, asked, tell));
        Self { asks, told }
    }

    pub fn ask(&self, ask: Ask) {
        let _ = self.asks.send(ask);
    }

    pub fn told(&self) -> Option<Told> {
        self.told.try_recv().ok()
    }
}

impl Drop for Line {
    fn drop(&mut self) {
        let _ = self.asks.send(Ask::Leave);
    }
}

fn run(
    base: String,
    entrant: Entrant,
    code: Option<String>,
    asked: Receiver<Ask>,
    tell: Sender<Told>,
) {
    let client = RelayClient::new(&base);
    let seated = match &code {
        None => client.open(&entrant),
        Some(code) => client.join(code, &entrant),
    };
    let (code, seat) = match seated {
        Ok(seat) => seat,
        Err(error) => {
            let _ = tell.send(Told::Gone(error.to_string()));
            return;
        }
    };
    if tell.send(Told::Seated { code: code.clone() }).is_err() {
        let _ = client.leave(&code, &seat);
        return;
    }
    let mut last_look = Instant::now() - POLL;
    loop {
        let ask = match asked.recv_timeout(Duration::from_millis(100)) {
            Ok(ask) => Some(ask),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = client.leave(&code, &seat);
                return;
            }
        };
        let answer = match ask {
            Some(Ask::Leave) => {
                let _ = client.leave(&code, &seat);
                return;
            }
            Some(Ask::Orders { turn, orders }) => Some(client.orders(&code, &seat, turn, orders)),
            Some(Ask::Board {
                turn,
                hash,
                finished,
            }) => Some(client.board(&code, &seat, turn, hash, finished)),
            None if last_look.elapsed() >= POLL => {
                last_look = Instant::now();
                Some(client.look(&code, &seat))
            }
            None => None,
        };
        let told = match answer {
            None => continue,
            Some(Ok(view)) => Told::View(Box::new(view)),
            Some(Err(AskError::Refused(formiga_battle_room::relay::Refusal::NoSuchRoom))) => {
                let _ = tell.send(Told::Gone("The room has closed.".to_owned()));
                return;
            }
            Some(Err(error)) => Told::Problem(error.to_string()),
        };
        if tell.send(told).is_err() {
            let _ = client.leave(&code, &seat);
            return;
        }
    }
}
