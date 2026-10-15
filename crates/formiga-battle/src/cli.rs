//! The headless commands: AI matches told turn by turn, fairness tournaments, rosters read from a
//! colony the way Desktop would hand them over, and replays checked turn by turn.

use anyhow::{Context, Result, bail};
use formiga_battle_contract::{BattleSnapshot, SessionId, sample};
use formiga_battle_core::ai::Coach;
use formiga_battle_core::recap::{describe, recap, team_name};
use formiga_battle_core::tournament::{self, drawn_creature};
use formiga_battle_core::*;
use std::path::PathBuf;

pub const HELP: &str = "\
HEADLESS COMMANDS:
    formiga-battle <COMMAND> [OPTIONS]

    roster       List the companions a roster offers, and how each carries out orders
    match        Play an AI match and tell it turn by turn
    tournament   Play many AI matches between random trios and report fairness
    replay       Replay a recorded match and check every turn
    arena        Draw an arena's tiles
    serve        Run a room service for matches between friends (--bind <addr:port>, default
                 127.0.0.1:7878; 0.0.0.0:7878 for the local network)

ROSTER (roster, match):
    --sample             The sample colony (the default)
    --from-save <FILE>   A Desktop colony.json, read and never written
    --snapshot <FILE>    A Battle roster snapshot

MATCH:
    --seed <N>           Match seed (default 1)
    --left <i,j,k>       Roster places for the left team (default 0,1,2)
    --right <i,j,k>      Roster places for the right team (default: three drawn visitors)
    --coach <a>,<b>      exhibition, familiar, generic or absent, for left and right (default
                         exhibition,exhibition). familiar and generic think each turn ahead;
                         only familiar knows how its own creatures carry out orders
    --board              Draw the board after every turn
    --record <FILE>      Write the match record as JSON
    --json               Print every turn's report as JSON instead of telling it

TOURNAMENT:
    --pairings <N>       Pairs of random trios, each played from both sides (default 500)
    --seed <N>           Tournament seed (default 1)
    --coach <a>,<b>      As for match
    --trios <N>          How many best and worst trios to list (default 5)

RULES (match, tournament):
    --capture <rule>     uncontested (the default) or majority
    --capture-ticks <N>  Ticks of holding that light the beacon (default 6)
    --turn-cap <N>       The last turn (default 12)

ARENA:
    --file <FILE>        An arena file (default: Backyard)
";

struct Args(Vec<String>);

impl Args {
    fn flag(&mut self, name: &str) -> bool {
        match self.0.iter().position(|a| a == name) {
            Some(i) => {
                self.0.remove(i);
                true
            }
            None => false,
        }
    }

    fn value(&mut self, name: &str) -> Result<Option<String>> {
        match self.0.iter().position(|a| a == name) {
            Some(i) if i + 1 < self.0.len() => {
                self.0.remove(i);
                Ok(Some(self.0.remove(i)))
            }
            Some(_) => bail!("{name} needs a value"),
            None => Ok(None),
        }
    }

    fn number(&mut self, name: &str, default: u64) -> Result<u64> {
        self.value(name)?
            .map(|v| v.parse().with_context(|| format!("{name} takes a number")))
            .transpose()
            .map(|v| v.unwrap_or(default))
    }

    fn done(self) -> Result<()> {
        if let Some(extra) = self.0.first() {
            bail!("unexpected argument {extra:?}; see --help");
        }
        Ok(())
    }
}

/// The commands this module answers.
pub const COMMANDS: [&str; 6] = ["roster", "match", "tournament", "replay", "arena", "serve"];

pub fn run(mut args: Vec<String>) -> Result<()> {
    let command = args.remove(0);
    let mut args = Args(args);
    match command.as_str() {
        "roster" => roster(args),
        "match" => play_match(args),
        "tournament" => run_tournament(args),
        "replay" => {
            let file = args
                .0
                .first()
                .cloned()
                .context("replay needs a record file")?;
            args.0.remove(0);
            args.done()?;
            replay(PathBuf::from(file))
        }
        "serve" => {
            let bind = args
                .value("--bind")?
                .unwrap_or_else(|| format!("127.0.0.1:{}", formiga_battle_room::DEFAULT_PORT));
            args.done()?;
            let server = formiga_battle_room::serve(&bind, formiga_battle_room::Limits::default())
                .with_context(|| format!("could not listen on {bind}"))?;
            eprintln!(
                "formiga-battle: rooms open on {} (protocol {}, rules version {}); stop with Ctrl-C",
                server.addr(),
                formiga_battle_room::PROTOCOL_VERSION,
                RULES_VERSION
            );
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        "arena" => {
            let arena = load_arena(args.value("--file")?)?;
            args.done()?;
            println!("{} ({} by {})", arena.name, arena.width, arena.height);
            let m = Match::new(arena, Ruleset::default(), 0, visitors(0, 0))?;
            print!("{}", draw(&m));
            Ok(())
        }
        other => bail!("unknown command {other:?}; see --help"),
    }
}

fn load_arena(file: Option<String>) -> Result<Arena> {
    match file {
        Some(path) => {
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
            Ok(Arena::parse(&text).with_context(|| format!("reading the arena in {path}"))?)
        }
        None => Ok(Arena::backyard()),
    }
}

fn load_roster(args: &mut Args) -> Result<Vec<Profile>> {
    let from_save = args.value("--from-save")?;
    let snapshot = args.value("--snapshot")?;
    args.flag("--sample");
    let snapshot = match (from_save, snapshot) {
        (Some(path), _) => {
            let bytes = std::fs::read(&path).with_context(|| format!("reading {path}"))?;
            let save = formiga_core::decode(&bytes)
                .with_context(|| format!("{path} is not a colony Desktop can read"))?
                .into_inner();
            formiga_battle_contract::project_roster(
                &save,
                local_session(),
                save.maximum_seen_utc,
                "0.67.6",
            )?
        }
        (None, Some(path)) => BattleSnapshot::read(path.as_ref())?,
        (None, None) => sample::snapshot(),
    };
    Ok(formiga_battle_room::roster::profiles(&snapshot))
}

/// A fixed session for a roster read here; it never leaves this process.
fn local_session() -> SessionId {
    SessionId::parse("00000000000000000000000000000000").expect("a session id")
}

/// Three creatures drawn the way Desktop draws new companions: the AI's visiting team.
fn visitors(seed: u64, start: u64) -> [Vec<Profile>; 2] {
    let draw = |from: u64| {
        (from..from + 3)
            .map(|i| drawn_creature(seed, 1000 + i))
            .collect()
    };
    [draw(start), draw(start + 3)]
}

fn places(text: &str) -> Result<Vec<usize>> {
    text.split(',')
        .map(|p| {
            p.trim()
                .parse::<usize>()
                .context("roster places are numbers")
        })
        .collect()
}

fn ruleset(args: &mut Args) -> Result<Ruleset> {
    let mut rules = Ruleset::default();
    if let Some(rule) = args.value("--capture")? {
        rules.capture_rule = match rule.as_str() {
            "uncontested" => CaptureRule::Uncontested,
            "majority" => CaptureRule::Majority,
            other => bail!("unknown capture rule {other:?}"),
        };
    }
    rules.capture_ticks = args.number("--capture-ticks", u64::from(rules.capture_ticks))? as u8;
    rules.turn_cap = args.number("--turn-cap", u64::from(rules.turn_cap))? as u8;
    Ok(rules)
}

fn coaches(args: &mut Args) -> Result<[Coach; 2]> {
    let text = args
        .value("--coach")?
        .unwrap_or_else(|| "exhibition,exhibition".to_owned());
    let parse = |s: &str| match s.trim() {
        "exhibition" => Ok(Coach::Exhibition),
        "absent" => Ok(Coach::Absent),
        "familiar" => Ok(Coach::Familiar),
        "generic" => Ok(Coach::Generic),
        other => bail!("unknown coach {other:?}"),
    };
    let parts: Vec<&str> = text.split(',').collect();
    match parts.as_slice() {
        [one] => Ok([parse(one)?; 2]),
        [a, b] => Ok([parse(a)?, parse(b)?]),
        _ => bail!("--coach takes one or two coaches"),
    }
}

fn roster(mut args: Args) -> Result<()> {
    let profiles = load_roster(&mut args)?;
    args.done()?;
    println!(
        "{:<3} {:<12} {:<13} {:<34} {:>7} {:>8} {:>8} {:>7} {:>8} {:>6}",
        "#",
        "Name",
        "Kind",
        "Tension",
        "caution",
        "together",
        "wander",
        "opport",
        "patience",
        "whim‰"
    );
    for (i, p) in profiles.iter().enumerate() {
        let b = p.behavior();
        println!(
            "{:<3} {:<12} {:<13} {:<34} {:>7} {:>8} {:>8} {:>7} {:>8} {:>6}{}",
            i,
            p.name,
            p.kind.label(),
            p.tension.map(|t| format!("{t:?}")).unwrap_or_default(),
            b.caution,
            b.togetherness,
            b.wanderlust,
            b.opportunism,
            b.patience,
            b.variability,
            if b.steps_back { "  steps back" } else { "" }
        );
    }
    Ok(())
}

fn play_match(mut args: Args) -> Result<()> {
    let roster = load_roster(&mut args)?;
    let seed = args.number("--seed", 1)?;
    let left = args.value("--left")?;
    let right = args.value("--right")?;
    let coaches = coaches(&mut args)?;
    let rules = ruleset(&mut args)?;
    let board = args.flag("--board");
    let json = args.flag("--json");
    let record = args.value("--record")?;
    let arena = load_arena(args.value("--file")?)?;
    args.done()?;
    let pick = |text: Option<String>, default: Vec<Profile>| -> Result<Vec<Profile>> {
        match text {
            None => Ok(default),
            Some(text) => places(&text)?
                .into_iter()
                .map(|i| {
                    roster
                        .get(i)
                        .cloned()
                        .with_context(|| format!("the roster has no place {i}"))
                })
                .collect(),
        }
    };
    let [visit_left, visit_right] = visitors(seed, 0);
    let home: Vec<Profile> = if roster.len() >= 3 {
        roster[..3].to_vec()
    } else {
        visit_left
    };
    let left = pick(left, home)?;
    let right = pick(right, visit_right)?;
    let mut m = Match::new(arena, rules, seed, [left, right])?;
    if !json {
        for side in Side::BOTH {
            let team: Vec<String> = m.rosters[side.index()]
                .iter()
                .map(|p| format!("{} ({})", p.name, p.kind.label()))
                .collect();
            println!("{}: {}", team_name(side), team.join(", "));
        }
        println!();
        if board {
            print!("{}", draw(&m));
        }
    }
    while m.outcome().is_none() {
        let orders = [
            coaches[0].orders(&m, Side::Left),
            coaches[1].orders(&m, Side::Right),
        ];
        let report = m.resolve_turn(orders);
        if json {
            println!("{}", serde_json::to_string(&report)?);
            continue;
        }
        println!("Turn {}", report.turn);
        for slot in Slot::all() {
            if let Some(order) = report.orders[slot.side.index()][slot.index as usize] {
                println!("  {:<10} {}", m.profile(slot).name, describe(&m, &order));
            }
        }
        for line in recap(&m, &report.events) {
            println!("  · {line}");
        }
        if board {
            print!("{}", draw(&m));
        }
        println!();
    }
    if let Some(path) = record {
        std::fs::write(&path, serde_json::to_vec_pretty(&m.record())?)
            .with_context(|| format!("writing {path}"))?;
        if !json {
            println!("Recorded to {path}");
        }
    }
    Ok(())
}

fn run_tournament(mut args: Args) -> Result<()> {
    let pairings = args.number("--pairings", 500)? as u32;
    let seed = args.number("--seed", 1)?;
    let trios = args.number("--trios", 5)? as usize;
    let coaches = coaches(&mut args)?;
    let rules = ruleset(&mut args)?;
    let arena = load_arena(args.value("--file")?)?;
    args.done()?;
    let s = tournament::run(&arena, rules, seed, pairings, coaches);
    let pct = |n: u32| 100.0 * f64::from(n) / f64::from(s.matches.max(1));
    println!(
        "{} matches on {}, {pairings} pairings each played from both sides",
        s.matches, arena.name
    );
    println!(
        "  by capture {:.0}%, by knockout {:.0}%, at the turn cap {:.0}%, drawn {:.0}%",
        pct(s.by_capture),
        pct(s.by_knockout),
        pct(s.at_cap),
        pct(s.draws)
    );
    println!(
        "  {:.1} turns a match; {:.2} knockouts a match; {:.2} fallback orders a match",
        s.average_turns(),
        f64::from(s.knockouts) / f64::from(s.matches.max(1)),
        f64::from(s.fallback_orders) / f64::from(s.matches.max(1))
    );
    println!("  left side scores {:.3} (0.5 is even)", s.left.score());
    println!();
    println!("By temperament kind (wins plus half of draws):");
    let mut kinds: Vec<_> = s.by_kind.iter().collect();
    kinds.sort_by(|a, b| b.1.score().total_cmp(&a.1.score()));
    for (kind, r) in &kinds {
        println!("  {:<13} {:.3}  ({} played)", kind, r.score(), r.played);
    }
    println!("  spread between kinds: {:.3}", s.kind_spread());
    let mut trio_list: Vec<_> = s.by_trio.iter().filter(|(_, r)| r.played >= 10).collect();
    trio_list.sort_by(|a, b| b.1.score().total_cmp(&a.1.score()));
    if !trio_list.is_empty() {
        println!();
        println!("Trios played at least ten times: {}", trio_list.len());
        for (trio, r) in trio_list.iter().take(trios) {
            println!("  best  {:.3}  {trio} ({})", r.score(), r.played);
        }
        for (trio, r) in trio_list.iter().rev().take(trios).rev() {
            println!("  worst {:.3}  {trio} ({})", r.score(), r.played);
        }
    }
    Ok(())
}

fn replay(path: PathBuf) -> Result<()> {
    let text = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let record: MatchRecord = serde_json::from_slice(&text).context("reading the match record")?;
    let (m, reports) = record.replay()?;
    println!(
        "Replayed {} turns; every board matched the record.",
        reports.len()
    );
    if let Some(outcome) = m.outcome() {
        println!("{outcome:?}");
    }
    Ok(())
}

/// The board as text: left team A B C, right team x y z, `#` obstacles, `=` a closed gate, `/`
/// an open one, `L` the lever, `:` the zone.
fn draw(m: &Match) -> String {
    let mut out = String::new();
    for y in 0..m.arena.height as i8 {
        out.push_str("    ");
        for x in 0..m.arena.width as i8 {
            let tile = Tile::new(x, y);
            let symbol = match m.occupant(tile) {
                Some(slot) => match slot.side {
                    Side::Left => (b'A' + slot.index) as char,
                    Side::Right => (b'x' + slot.index) as char,
                },
                None => match m.arena.terrain(tile) {
                    arena::Terrain::Obstacle => '#',
                    arena::Terrain::Lever => 'L',
                    arena::Terrain::Gate if m.walkable(tile) => '/',
                    arena::Terrain::Gate => '=',
                    arena::Terrain::Open if m.arena.in_zone(tile) => ':',
                    arena::Terrain::Open => '.',
                },
            };
            out.push(symbol);
            out.push(' ');
        }
        out.push('\n');
    }
    let status: Vec<String> = Slot::all()
        .map(|s| {
            let f = m.fighter(s);
            let letter = match s.side {
                Side::Left => (b'A' + s.index) as char,
                Side::Right => (b'x' + s.index) as char,
            };
            if f.out {
                format!("{letter} {} out", m.profile(s).name)
            } else {
                format!(
                    "{letter} {} {}",
                    m.profile(s).name,
                    "●".repeat(f.composure as usize)
                )
            }
        })
        .collect();
    out.push_str(&format!(
        "    {}\n    beacon {}/{} left, {}/{} right\n",
        status.join("  "),
        m.board.capture[0],
        m.ruleset.capture_ticks,
        m.board.capture[1],
        m.ruleset.capture_ticks
    ));
    out
}
