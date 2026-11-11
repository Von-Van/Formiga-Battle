//! The line to the relay during a match with a friend, on a thread of its own so the window never
//! waits on the network. The window asks it to send its plan or a board; it answers with every
//! view of the room it gets, and asks the relay for one about once a second.
//!
//! What the window asks to send goes into an outbox and stays there until the relay has taken
//! it: one failed request (a Wi-Fi blip, a relay slow to start) must not cost a turn's plan, or
//! the last board a win waits on to be confirmed.

use formiga_battle_core::TeamPlan;
use formiga_battle_room::relay::Refusal;
use formiga_battle_room::{AskError, Entrant, RelayClient, View};
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// How often the room is looked at while nothing else is said.
const POLL: Duration = Duration::from_millis(900);
/// While the relay cannot be reached, or says it is busy, each look waits twice as long as the
/// last, up to this many doublings, so a relay that is down or struggling is not asked again and
/// again.
const MOST_DOUBLINGS: u32 = 3;
/// How long a send the relay did not take waits before it is tried again, doubling with each
/// failure up to [`MOST_DOUBLINGS`] times.
const RETRY: Duration = Duration::from_millis(500);
/// How long leaving waits for the outbox to empty before it leaves anyway: the last board must
/// reach the relay before the room is left, or a friend's win can never be confirmed.
const FLUSH_ON_LEAVE: Duration = Duration::from_secs(4);

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
    let mut outbox = Outbox::default();
    loop {
        let ask = match asked.recv_timeout(Duration::from_millis(100)) {
            Ok(ask) => Some(ask),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                outbox.flush(&client, &code, &seat);
                let _ = client.leave(&code, &seat);
                return;
            }
        };
        match ask {
            Some(Ask::Leave) => {
                outbox.flush(&client, &code, &seat);
                let _ = client.leave(&code, &seat);
                return;
            }
            Some(Ask::Orders { turn, orders }) => outbox.push(Sending::Orders { turn, orders }),
            Some(Ask::Board {
                turn,
                hash,
                finished,
            }) => outbox.push(Sending::Board {
                turn,
                hash,
                finished,
            }),
            None => {}
        }
        let now = Instant::now();
        // Anything waiting to be sent goes before a look: the relay answers it with a view too.
        let answer = if let Some(sending) = outbox.due(now) {
            let answer = sending.send(&client, &code, &seat);
            outbox.answered(&answer, now);
            Some(answer)
        } else if last_look
            .is_none_or(|at| at.elapsed() >= POLL * (1 << strained_looks.min(MOST_DOUBLINGS)))
        {
            last_look = Some(now);
            let answer = client.look(&code, &seat);
            if strained(&answer) {
                strained_looks += 1;
            } else {
                strained_looks = 0;
            }
            Some(answer)
        } else {
            None
        };
        let told = match answer {
            None => continue,
            Some(Ok(view)) => Told::View(Box::new(view)),
            Some(Err(AskError::Refused(Refusal::NoSuchRoom))) => {
                let _ = tell.send(Told::Gone("The room has closed.".to_owned()));
                return;
            }
            Some(Err(error)) => Told::Problem(error.to_string()),
        };
        if tell.send(told).is_err() {
            outbox.flush(&client, &code, &seat);
            let _ = client.leave(&code, &seat);
            return;
        }
    }
}

/// Something the window asked the relay to take.
#[derive(Clone, Debug, PartialEq)]
enum Sending {
    Orders { turn: u8, orders: TeamPlan },
    Board { turn: u8, hash: u64, finished: bool },
}

impl Sending {
    fn send(&self, client: &RelayClient, code: &str, seat: &str) -> Result<View, AskError> {
        match self {
            Self::Orders { turn, orders } => client.orders(code, seat, *turn, orders.clone()),
            Self::Board {
                turn,
                hash,
                finished,
            } => client.board(code, seat, *turn, *hash, *finished),
        }
    }
}

/// What the window asked to send and the relay has not taken yet, oldest first. Sending either
/// again is safe: the relay keeps a side's newest plan for the turn and its newest board.
#[derive(Default)]
struct Outbox {
    waiting: VecDeque<Sending>,
    /// Failed sends in a row, for how long the next try waits.
    failures: u32,
    /// Not before this, after a failed send.
    next_try: Option<Instant>,
}

impl Outbox {
    /// A newer plan replaces any plan still waiting, which is for this turn or one already
    /// locked; a board for a turn already waiting replaces that one.
    fn push(&mut self, sending: Sending) {
        match &sending {
            Sending::Orders { .. } => self
                .waiting
                .retain(|s| !matches!(s, Sending::Orders { .. })),
            Sending::Board { turn, .. } => self
                .waiting
                .retain(|s| !matches!(s, Sending::Board { turn: t, .. } if t == turn)),
        }
        self.waiting.push_back(sending);
    }

    /// The oldest send, if it may be tried now.
    fn due(&self, now: Instant) -> Option<&Sending> {
        if self.next_try.is_some_and(|at| now < at) {
            return None;
        }
        self.waiting.front()
    }

    /// What the relay said to the oldest send. Taken, or turned away for good, it leaves the
    /// outbox; not heard (the relay down, busy, or its answer unreadable) it is tried again
    /// later, each wait twice the last.
    fn answered(&mut self, answer: &Result<View, AskError>, now: Instant) {
        let again = matches!(
            answer,
            Err(AskError::Unreachable(_) | AskError::Unreadable | AskError::Refused(Refusal::Busy))
        );
        if again {
            self.failures += 1;
            self.next_try = Some(now + RETRY * (1 << self.failures.min(MOST_DOUBLINGS)));
        } else {
            self.waiting.pop_front();
            self.failures = 0;
            self.next_try = None;
        }
    }

    /// Sends what is waiting before the room is left, for a few seconds at most.
    fn flush(&mut self, client: &RelayClient, code: &str, seat: &str) {
        let until = Instant::now() + FLUSH_ON_LEAVE;
        while let Some(sending) = self.waiting.front().cloned() {
            let now = Instant::now();
            if now >= until {
                return;
            }
            if let Some(at) = self.next_try
                && at > now
            {
                std::thread::sleep((at - now).min(until - now));
                continue;
            }
            let answer = sending.send(client, code, seat);
            if matches!(answer, Err(AskError::Refused(Refusal::NoSuchRoom))) {
                return;
            }
            self.answered(&answer, Instant::now());
        }
    }
}

/// Whether an answer says the relay is down or too busy to answer, so the next look waits longer.
fn strained(answer: &Result<View, AskError>) -> bool {
    matches!(
        answer,
        Err(AskError::Unreachable(_) | AskError::Refused(Refusal::Busy))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_core::{Directive, Tile};

    #[test]
    fn a_busy_relay_is_asked_less_often_like_one_that_is_down() {
        assert!(strained(&Err(AskError::Unreachable("no route".to_owned()))));
        assert!(strained(&Err(AskError::Refused(Refusal::Busy))));
        // Anything else is an answer: the relay is there and coping.
        assert!(!strained(&Err(AskError::Refused(Refusal::NotThisTurn))));
        assert!(!strained(&Err(AskError::Unreadable)));
    }

    fn plan(x: i8) -> TeamPlan {
        vec![Directive::Interact {
            at: Tile::new(x, 0),
        }]
    }

    fn board(turn: u8) -> Sending {
        Sending::Board {
            turn,
            hash: u64::from(turn),
            finished: false,
        }
    }

    fn unreachable() -> Result<View, AskError> {
        Err(AskError::Unreachable("no route".to_owned()))
    }

    #[test]
    fn a_send_the_relay_did_not_hear_stays_and_is_tried_again_later() {
        let mut outbox = Outbox::default();
        let start = Instant::now();
        outbox.push(board(1));
        assert_eq!(outbox.due(start), Some(&board(1)));
        outbox.answered(&unreachable(), start);
        assert_eq!(outbox.due(start), None, "not straight away");
        let later = start + RETRY * 2;
        assert_eq!(outbox.due(later), Some(&board(1)), "but again soon");
        outbox.answered(&Err(AskError::Refused(Refusal::Busy)), later);
        outbox.answered(&Err(AskError::Unreadable), later);
        assert_eq!(
            outbox.due(later + RETRY * 4),
            None,
            "each wait longer than the last"
        );
        assert_eq!(
            outbox.due(later + RETRY * (1 << MOST_DOUBLINGS)),
            Some(&board(1)),
            "up to a limit"
        );
    }

    #[test]
    fn a_send_the_relay_answered_leaves_the_outbox_whatever_it_said() {
        let mut outbox = Outbox::default();
        let now = Instant::now();
        outbox.push(Sending::Orders {
            turn: 3,
            orders: plan(1),
        });
        outbox.push(board(2));
        // Orders for a turn that locked without them can never be taken: they are dropped.
        outbox.answered(&Err(AskError::Refused(Refusal::NotThisTurn)), now);
        assert_eq!(outbox.due(now), Some(&board(2)));
        outbox.answered(&Err(AskError::Refused(Refusal::Malformed)), now);
        assert_eq!(outbox.due(now), None);
    }

    #[test]
    fn a_newer_plan_replaces_one_still_waiting_and_boards_keep_their_order() {
        let mut outbox = Outbox::default();
        let now = Instant::now();
        outbox.push(board(1));
        outbox.push(Sending::Orders {
            turn: 2,
            orders: plan(1),
        });
        outbox.push(Sending::Orders {
            turn: 2,
            orders: plan(2),
        });
        outbox.push(board(2));
        outbox.push(Sending::Board {
            turn: 2,
            hash: 9,
            finished: true,
        });
        let waiting: Vec<Sending> = outbox.waiting.iter().cloned().collect();
        assert_eq!(
            waiting,
            vec![
                board(1),
                Sending::Orders {
                    turn: 2,
                    orders: plan(2)
                },
                Sending::Board {
                    turn: 2,
                    hash: 9,
                    finished: true
                },
            ]
        );
        assert_eq!(outbox.due(now), Some(&board(1)));
    }
}
