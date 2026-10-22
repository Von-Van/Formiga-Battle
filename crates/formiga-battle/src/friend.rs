//! The line to the relay during a match with a friend, on a thread of its own so the window never
//! waits on the network. The window asks it to send its plan or a board; it answers with every
//! view of the room it gets, and asks the relay for one about once a second.

use formiga_battle_core::TeamPlan;
use formiga_battle_room::{AskError, Entrant, RelayClient, View};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// How often the room is looked at while nothing else is said.
const POLL: Duration = Duration::from_millis(900);
/// While the relay cannot be reached, or says it is busy, each look waits twice as long as the
/// last, up to this many doublings, so a relay that is down or struggling is not asked again and
/// again.
const MOST_DOUBLINGS: u32 = 3;

pub enum Ask {
    Orders { turn: u8, orders: TeamPlan },
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

    /// A line to no relay, for tests: what the window asks comes out of the receiver, and what
    /// is sent in is told to the window.
    #[cfg(test)]
    pub fn fake() -> (Self, Receiver<Ask>, Sender<Told>) {
        let (asks, asked) = mpsc::channel();
        let (tell, told) = mpsc::channel();
        (Self { asks, told }, asked, tell)
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
    // Reading the program to digest it takes a moment, so it is done here, off the window.
    let entrant = match entrant.build.clone().digested() {
        Ok(build) => Entrant { build, ..entrant },
        Err(error) => {
            let _ = tell.send(Told::Gone(format!(
                "Battle could not read its own program to say which copy it is: {error}"
            )));
            return;
        }
    };
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
    let mut last_look: Option<Instant> = None;
    let mut strained_looks = 0u32;
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
            None if last_look.is_none_or(|at| {
                at.elapsed() >= POLL * (1 << strained_looks.min(MOST_DOUBLINGS))
            }) =>
            {
                last_look = Some(Instant::now());
                Some(client.look(&code, &seat))
            }
            None => None,
        };
        match &answer {
            Some(answer) if strained(answer) => strained_looks += 1,
            Some(_) => strained_looks = 0,
            None => {}
        }
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

/// Whether an answer says the relay is down or too busy to answer, so the next look waits longer.
fn strained(answer: &Result<View, AskError>) -> bool {
    matches!(
        answer,
        Err(AskError::Unreachable(_) | AskError::Refused(formiga_battle_room::relay::Refusal::Busy))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_room::relay::Refusal;

    #[test]
    fn a_busy_relay_is_asked_less_often_like_one_that_is_down() {
        assert!(strained(&Err(AskError::Unreachable("no route".to_owned()))));
        assert!(strained(&Err(AskError::Refused(Refusal::Busy))));
        // Anything else is an answer: the relay is there and coping.
        assert!(!strained(&Err(AskError::Refused(Refusal::NotThisTurn))));
        assert!(!strained(&Err(AskError::Unreadable)));
    }
}
