//! Formiga Battle's rules: an arena, three universal orders, and a turn that resolves both teams'
//! orders at once, the same way on every machine.
//!
//! Nothing here draws, waits on a clock or touches a network. Given the same arena, ruleset,
//! seed, rosters and orders, a match comes out the same, byte for byte, which is what lets a
//! room service resolve turns for two players, a client animate the same events, and a recorded
//! match replay exactly.
//!
//! - [`arena`]: the board, authored as data.
//! - [`order`]: MOVE, HOLD and CONTEST.
//! - [`game`]: a match and how a turn resolves, tick by tick.
//! - [`profile`]: who a creature is, read from Desktop's own record.
//! - [`interpret`]: how a creature carries out its order, and why.
//! - [`ai`]: the fallback for a missing order, and the coaches of AI matches.
//! - [`plan`]: coaches that think a turn ahead, knowing their creatures or not.
//! - [`recap`]: a turn told in sentences.
//! - [`notebook`]: the Handler's Notebook: what each creature was seen to do, in plain words.
//! - [`replay`]: a whole match as data, and replaying it.
//! - [`tournament`]: many matches at once, for the fairness checks.

pub mod ai;
pub mod arena;
pub mod event;
pub mod game;
pub mod grid;
pub mod interpret;
pub mod notebook;
pub mod order;
pub mod plan;
pub mod profile;
pub mod recap;
pub mod replay;
pub mod rng;
pub mod ruleset;
pub mod tournament;

pub use arena::{Arena, TEAM_SIZE};
pub use event::{Event, EventKind, Outcome, Reason, Victory};
pub use game::{Match, TurnReport};
pub use grid::Tile;
pub use order::{Order, Orders, Side, Slot, Target};
pub use profile::{Behavior, Bond, Leanings, Nature, Profile, Strength};
pub use replay::MatchRecord;
pub use ruleset::{CaptureRule, RULES_VERSION, Ruleset};
