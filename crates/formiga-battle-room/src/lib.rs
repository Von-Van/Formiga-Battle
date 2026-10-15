//! Friend matches for Formiga Battle: two friends, an invite code, and a relay that passes their
//! orders on once both are in. Both windows work out every turn themselves with the same rules
//! and check each other through the relay; the relay never runs the rules.
//!
//! - [`relay`]: the relay as a state machine: rooms, seats, deadlines, forfeits. The reference
//!   the relay on the Formiga site follows.
//! - [`http`]: the relay over HTTP: the client Battle uses, and a local relay for development.
//! - [`code`]: invite codes.
//! - [`roster`]: how a team of Desktop's companions is read as match profiles, the same on every
//!   machine.

pub mod code;
pub mod http;
pub mod relay;
pub mod roster;

pub use http::{AskError, LocalRelay, RELAY_BASE, RELAY_ENV, RelayClient, relay_base, serve_relay};
pub use relay::{Entrant, Limits, Locked, RELAY_VERSION, Relay, Stage, View};
