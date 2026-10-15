//! Private rooms for Formiga Battle: two friends, an invite code, and every turn resolved by the
//! room, which both clients check themselves against.
//!
//! - [`protocol`]: what a client and a room say to each other.
//! - [`service`]: the rooms as a state machine: codes, seats, deadlines, forfeits.
//! - [`net`]: the service on sockets, and the client end.
//! - [`code`]: invite codes.
//! - [`roster`]: how a team of Desktop's companions is read as match profiles, the same on every
//!   machine.

pub mod code;
pub mod net;
pub mod protocol;
pub mod roster;
pub mod service;

pub use net::{Client, Heard, Server, serve};
pub use protocol::{ClientMsg, DEFAULT_PORT, PROTOCOL_VERSION, Refusal, ServerMsg};
pub use service::{Limits, Service};
