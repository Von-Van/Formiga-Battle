//! Formiga Battle: 3v3 simultaneous-order tactics for Formiga Desktop's companions. With no
//! command it opens the window; the headless commands are in `cli`.

mod announce;
mod app;
mod chrome;
mod cli;
mod feel;
mod found;
mod friend;
mod garden;
mod heraldry;
mod host;
mod icon;
mod identity;
mod iso;
mod lettering;
mod look;
mod masthead;
mod pennant;
mod playback;
mod rewards;
mod scene;
mod sound;
mod station;
mod table;
mod ticket;

use anyhow::{Context, Result, bail};
use formiga_art::Canvas;
use formiga_battle_contract::{BATTLE_FORMAT_VERSION, LAUNCH_ARGUMENT, sample};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
Formiga Battle

USAGE:
    formiga-battle [OPTIONS]              Open the window and play against the visitors
    formiga-battle <COMMAND> [OPTIONS]    A headless command (below)

WINDOW:
    --sample                 The sample colony's companions (the default)
    --from-save <FILE>       A Desktop colony.json, read and never written
    --snapshot <FILE>        A Battle roster snapshot
    --formiga-battle <DIR>   A session Desktop opened (how Desktop starts Battle)
    --seed <N>               The match seed (default: from the clock)
    --arena <ID>             Which arena the visitors are played in: backyard (the default),
                             woodland, millpond or cove
    --visitors <N>           Which neighbouring colony visits, 0 to 255 (default 9)
    --snap <PNG>             Open, save a picture of the window after --at seconds (default 6)
                             and close
    --at <SECONDS>           With --snap
    --review <STAGE>         With --snap: station (the train in at the Battleground, everyone
                             aboard), where (choosing a place), picking or who (the team
                             choice), friend (a friend's room about to be opened), history,
                             settings, orders (a
                             turn's orders half given, three turns in, PRESS chosen), playing (the AI giving
                             your orders too,
                             every turn locked at once; the default) or notebook (the sample
                             colony's notebook, as Desktop would send it, open) or room (a room
                             opened on the relay, its code showing; FORMIGA_BATTLE_RELAY points
                             at a local one from `formiga-battle relay`) or account (as
                             playing, with the whole match open) or lessons (choosing a Field
                             Lesson) or lesson (the lesson on flanks, orders not yet given)
    --render-board <PNG>     Draw the board without a window, after --turns turns of AI play
                             (default 0), at --scale (default 3), --into seconds into the
                             turn after (default: still)
    --turns <N>, --into <SECONDS>, --scale <N>
    --window <W>x<H>         The window's size in points (default 1280x780), for a picture of
                             how Battle looks in a smaller or larger window
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // For the packaging scripts: the newest battle contract version this Battle reads, which goes
    // in the macOS bundle and the Windows registry for Desktop to find.
    if args.first().is_some_and(|a| a == "--battle-version") {
        println!("{BATTLE_FORMAT_VERSION}");
        return Ok(());
    }
    // For the packaging scripts too: Battle's icon, for the bundle and the installer.
    if args.first().is_some_and(|a| a == "--icon") {
        let folder = args
            .get(1)
            .map(PathBuf::from)
            .context("--icon needs a folder to write the icon into")?;
        icon::write(&folder)?;
        println!("Wrote Battle's icon to {}", folder.display());
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}\n{}", cli::HELP);
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|a| cli::COMMANDS.contains(&a.as_str()))
    {
        return cli::run(args);
    }
    window(args)
}

struct Parsed {
    source: host::Source,
    seed: Option<u64>,
    visitors: u8,
    snap: Option<PathBuf>,
    at: f32,
    review: app::Review,
    render: Option<PathBuf>,
    arena: Option<String>,
    turns: u32,
    into: Option<f32>,
    scale: u32,
    /// The window's size, in points, for a review picture at a size of its own.
    window: Option<(f32, f32)>,
}

fn parse(args: Vec<String>) -> Result<Parsed> {
    let mut parsed = Parsed {
        source: host::Source::Sample,
        seed: None,
        visitors: 9,
        snap: None,
        at: 6.0,
        review: app::Review::Playing,
        render: None,
        arena: None,
        turns: 0,
        into: None,
        scale: 3,
        window: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--sample" => parsed.source = host::Source::Sample,
            "--from-save" => parsed.source = host::Source::Save(value()?.into()),
            "--snapshot" => parsed.source = host::Source::Snapshot(value()?.into()),
            a if a == LAUNCH_ARGUMENT => parsed.source = host::Source::Session(value()?.into()),
            "--seed" => parsed.seed = Some(value()?.parse().context("--seed takes a number")?),
            "--visitors" => {
                parsed.visitors = value()?.parse().context("--visitors takes 0 to 255")?
            }
            "--snap" => parsed.snap = Some(value()?.into()),
            "--at" => parsed.at = value()?.parse().context("--at takes seconds")?,
            "--review" => {
                parsed.review = match value()?.as_str() {
                    "station" => app::Review::Station,
                    "where" => app::Review::Where,
                    "picking" | "who" => app::Review::Picking,
                    "friend" => app::Review::Friend,
                    "history" => app::Review::History,
                    "handler" => app::Review::Handler,
                    "settings" => app::Review::Settings,
                    "orders" => app::Review::Orders,
                    "playing" => app::Review::Playing,
                    "notebook" => app::Review::Notebook,
                    "room" => app::Review::Room,
                    "account" => app::Review::Account,
                    "lessons" => app::Review::Lessons,
                    "lesson" => app::Review::Lesson,
                    other => bail!("unknown stage {other:?}"),
                }
            }
            "--render-board" => parsed.render = Some(value()?.into()),
            "--arena" => parsed.arena = Some(value()?),
            "--turns" => parsed.turns = value()?.parse().context("--turns takes a number")?,
            "--into" => parsed.into = Some(value()?.parse().context("--into takes seconds")?),
            "--scale" => parsed.scale = value()?.parse().context("--scale takes a number")?,
            "--window" => {
                let size = value()?;
                let (w, h) = size
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                    .context("--window takes a size such as 1024x640")?;
                parsed.window = Some((w, h));
            }
            other => bail!("unknown option {other:?}; see --help"),
        }
    }
    Ok(parsed)
}

fn window(args: Vec<String>) -> Result<()> {
    let args = parse(args)?;
    let seed = args.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)
    });
    let (host, roster) = host::open(args.source)?;
    let mut table = table::Table::new(roster, sample::visitors(args.visitors), seed);
    if let Some(id) = &args.arena {
        table.arena = formiga_battle_core::Arena::by_id(id)
            .with_context(|| format!("there is no arena called {id:?}"))?;
        // A picture of an arena shows that arena, whichever cups the colony has won so far.
        if args.render.is_some() || args.snap.is_some() {
            for cup in &rewards::CUPS {
                if !table.record.unlocked.iter().any(|u| u == cup.id) {
                    table.record.unlocked.push(cup.id.to_owned());
                }
            }
        }
    }
    if let Some(path) = args.render {
        return render_board(&path, &mut table, args.turns, args.into, args.scale);
    }
    if let host::Host::Visit {
        directory,
        snapshot_sha256,
    } = &host
    {
        table.session = Some(table::Session {
            directory: directory.clone(),
            result: formiga_battle_contract::BattleResult::new(
                session_of(directory)?,
                snapshot_sha256.clone(),
                env!("CARGO_PKG_VERSION"),
            ),
        });
    }
    let snap = args.snap.map(|path| app::Snap {
        path,
        at: args.at,
        asked: false,
    });
    let review = snap.as_ref().map(|_| args.review);
    let reviewing = review.is_some();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Formiga Battle")
        .with_inner_size(args.window.map_or([1280.0, 780.0], |(w, h)| [w, h]))
        .with_min_inner_size([1024.0, 640.0])
        .with_icon(icon::window());
    if reviewing {
        // A window for review only: it stays behind whatever the owner is doing.
        viewport = viewport.with_active(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Formiga Battle",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Dark);
            // A window opened only for its picture plays nothing.
            let sound = if reviewing {
                sound::Sound::silent()
            } else {
                sound::Sound::open()
            };
            Ok(Box::new(app::BattleApp::new(
                host, table, review, snap, sound,
            )))
        }),
    )
    .map_err(|error| anyhow::anyhow!("the window could not open: {error}"))
}

fn session_of(directory: &Path) -> Result<formiga_battle_contract::SessionId> {
    directory
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(formiga_battle_contract::SessionId::parse)
        .context("the session directory is not named after a session")
}

/// The board as the window would show it, without one.
fn render_board(
    path: &Path,
    table: &mut table::Table,
    turns: u32,
    into: Option<f32>,
    scale: u32,
) -> Result<()> {
    table.start(0.0)?;
    let mut now = 0.0;
    for _ in 0..turns {
        if table.outcome().is_some() {
            break;
        }
        table.coach_orders();
        table.lock(now);
        now += 60.0;
        table.frame(now);
    }
    if let Some(into) = into
        && table.outcome().is_none()
    {
        table.coach_orders();
        table.lock(now);
        now += into;
    } else {
        table.coach_orders();
    }
    let frame = table.frame(now).context("a match is on")?;
    let marks = table.marks(None, now);
    let m = table.m.as_ref().context("a match is on")?;
    let view = iso::View::new(m.arena.width, m.arena.height);
    let garden = garden::Garden::new(&m.arena, &view);
    let canvas = scene::paint(
        &m.arena,
        &garden,
        &frame,
        &mut table.looks,
        &marks,
        m.ruleset.capture_ticks,
        now,
        table.reduce_motion,
        &view,
    );
    write_png(path, &canvas, scale)
}

pub(crate) fn write_png(path: &Path, canvas: &Canvas, scale: u32) -> Result<()> {
    let scale = scale.max(1);
    let (width, height) = (canvas.width() * scale, canvas.height() * scale);
    let mut bytes = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let pixel = canvas.get((x / scale) as i32, (y / scale) as i32);
            bytes.extend([pixel.r, pixel.g, pixel.b, pixel.a]);
        }
    }
    let file = std::fs::File::create(path)
        .with_context(|| format!("could not write {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&bytes)?;
    Ok(())
}
