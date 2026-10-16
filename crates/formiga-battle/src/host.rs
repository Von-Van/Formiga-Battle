//! Who the match is played for: Desktop, through a session directory, or a rehearsal that stands
//! in for Desktop with a roster read the same way.

use anyhow::{Context, Result, bail};
use formiga_battle_contract::{BattleSnapshot, Opened, SessionId, open_session, sample};
use std::path::PathBuf;

pub enum Host {
    /// Desktop started Battle with a session directory, and it has been answered. What the
    /// sitting earns is written back into it.
    Visit {
        directory: PathBuf,
        /// The SHA-256 of the snapshot's bytes, which every answer names.
        snapshot_sha256: String,
    },
    /// No Desktop: a roster from the sample colony or a colony file, read and never written.
    Rehearsal { from: String },
}

impl Host {
    pub fn describe(&self) -> String {
        match self {
            Self::Visit { .. } => "Opened from Formiga Desktop".to_owned(),
            Self::Rehearsal { from } => format!("Rehearsal: {from}"),
        }
    }
}

/// Where the roster comes from, as the command line asked.
pub enum Source {
    Sample,
    Save(PathBuf),
    Snapshot(PathBuf),
    Session(PathBuf),
}

pub fn open(source: Source) -> Result<(Host, BattleSnapshot)> {
    match source {
        Source::Sample => Ok((
            Host::Rehearsal {
                from: "the sample colony".to_owned(),
            },
            sample::snapshot(),
        )),
        Source::Save(path) => {
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            let save = formiga_core::decode(&bytes)
                .with_context(|| format!("{} is not a colony Desktop can read", path.display()))?
                .into_inner();
            let snapshot = formiga_battle_contract::project_roster(
                &save,
                SessionId::parse("00000000000000000000000000000000").expect("a session id"),
                save.maximum_seen_utc,
                // Battle shares its version with the Desktop release it is built on.
                env!("CARGO_PKG_VERSION"),
            )?;
            Ok((
                Host::Rehearsal {
                    from: path.display().to_string(),
                },
                snapshot,
            ))
        }
        Source::Snapshot(path) => Ok((
            Host::Rehearsal {
                from: path.display().to_string(),
            },
            BattleSnapshot::read(&path)?,
        )),
        Source::Session(directory) => match open_session(&directory, env!("CARGO_PKG_VERSION"))? {
            Opened::Roster(snapshot) => {
                let bytes = std::fs::read(directory.join(formiga_battle_contract::SNAPSHOT_FILE))?;
                let snapshot_sha256 = formiga_expansion_rulebook::sha256_hex(&bytes);
                Ok((
                    Host::Visit {
                        directory,
                        snapshot_sha256,
                    },
                    *snapshot,
                ))
            }
            Opened::Refused(refusal, error) => {
                bail!("turned the session away ({refusal:?}): {error}")
            }
        },
    }
}
