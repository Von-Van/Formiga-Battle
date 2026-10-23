//! Formiga Battle: 3v3 simultaneous-order tactics for Formiga Desktop's companions. With no
//! command it opens the window; the headless commands are in `cli`.

mod announce;
mod app;
mod career;
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
mod keepsake;
mod lettering;
mod look;
mod masthead;
mod pennant;
mod playback;
mod rewards;
mod rival;
mod scene;
mod settings;
mod sound;
mod station;
mod still;
mod table;
mod ticket;

use anyhow::{Context, Result, bail};
use formiga_art::Canvas;
use formiga_battle_contract::{BATTLE_FORMAT_VERSION, LAUNCH_ARGUMENT, sample};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
Formiga Battle

USAGE:
    formiga-battle [OPTIONS]              Open the window and play against the visitors. With
                                          nothing to say whose colony, the sample colony's
                                          companions under a card saying where your own come
                                          from
    formiga-battle <COMMAND> [OPTIONS]    A headless command (below)

WINDOW:
    --sample                 The sample colony's companions, with no welcome card
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
                             or welcome (the station under the card Battle opened by itself
                             shows)
    --render-window <PNG>    A picture of the window as --snap takes it (at 2 pixels to the point),
                             made without opening one: --review, --at and --window as for --snap
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
    /// A picture of the whole window, made without one.
    still: Option<PathBuf>,
    arena: Option<String>,
    turns: u32,
    into: Option<f32>,
    scale: u32,
    /// The window's size, in points, for a review picture at a size of its own.
    window: Option<(f32, f32)>,
    /// Opened by itself, with nothing to say whose colony and nothing to picture: the sample
    /// colony comes with the welcome card, saying where a player's own companions come from.
    welcome: bool,
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
        still: None,
        arena: None,
        turns: 0,
        into: None,
        scale: 3,
        window: None,
        welcome: false,
    };
    // Anything asked for at all, a colony, a session or a picture, is someone who knows what
    // they want, and gets it without the welcome.
    let mut asked = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
        if matches!(
            arg.as_str(),
            "--sample"
                | "--from-save"
                | "--snapshot"
                | "--review"
                | "--snap"
                | "--render-board"
                | "--render-window"
        ) || arg == LAUNCH_ARGUMENT
        {
            asked = true;
        }
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
                    "welcome" => app::Review::Welcome,
                    other => bail!("unknown stage {other:?}"),
                }
            }
            "--render-board" => parsed.render = Some(value()?.into()),
            "--render-window" => parsed.still = Some(value()?.into()),
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
    parsed.welcome = !asked;
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
    // The colony's career against the rivals is Battle's own to keep, for a colony Desktop sent.
    let career = match (&host, settings::folder()) {
        (host::Host::Visit { .. }, Some(data)) => Some((
            career::folder(&data),
            career::Career::load(&career::folder(&data), &roster.colony_id),
        )),
        _ => None,
    };
    let mut table = table::Table::new(roster, sample::visitors(args.visitors), seed);
    table.career = career;
    if let Some(id) = &args.arena {
        table.arena = formiga_battle_core::Arena::by_id(id)
            .with_context(|| format!("there is no arena called {id:?}"))?;
        // A picture of an arena shows that arena, whichever cups the colony has won so far.
        if args.render.is_some() || args.snap.is_some() || args.still.is_some() {
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
    if let Some(path) = &args.still {
        let size = args.window.unwrap_or((1280.0, 780.0));
        let mut app =
            app::BattleApp::new(host, table, Some(args.review), None, sound::Sound::silent());
        let canvas = still::picture(&mut app, size, args.at, 2.0);
        write_png(path, &canvas, 1)?;
        println!("Wrote {}", path.display());
        return Ok(());
    }
    let snap = args.snap.map(|path| app::Snap {
        path,
        at: args.at,
        asked: false,
    });
    let review = snap.as_ref().map(|_| args.review);
    let reviewing = review.is_some();
    // How Battle looked and sounded last time. A window opened for its picture reads and keeps
    // none of it, so a review looks the same on every machine.
    let keeper = if reviewing {
        None
    } else {
        settings::folder().map(settings::Keeper::open)
    };
    let kept = keeper.as_ref().map(settings::Keeper::settings);
    if let Some(reduce) = kept.and_then(|k| k.reduce_motion) {
        table.reduce_motion = reduce;
    }
    let size = args
        .window
        .map(|(w, h)| [w, h])
        .or(kept.and_then(|k| k.window))
        .unwrap_or([1280.0, 780.0]);
    let welcome = args.welcome;
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Formiga Battle")
        .with_inner_size(size)
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
                let mut sound = sound::Sound::open();
                if let Some(kept) = kept {
                    sound.set_levels(kept.levels());
                }
                sound
            };
            let mut app = app::BattleApp::new(host, table, review, snap, sound);
            if let Some(keeper) = keeper {
                app.keep(keeper);
            }
            Ok(Box::new(if welcome { app.welcoming() } else { app }))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Parsed {
        parse(args.iter().map(|a| (*a).to_owned()).collect()).expect("arguments Battle reads")
    }

    #[test]
    fn opened_by_itself_battle_welcomes_the_player_over_the_sample_colony() {
        let opened = parsed(&[]);
        assert!(opened.welcome);
        assert!(matches!(opened.source, host::Source::Sample));
        // Choosing the place or the seed still says nothing of whose colony.
        assert!(parsed(&["--arena", "woodland", "--seed", "4"]).welcome);
    }

    #[test]
    fn the_sample_asked_for_comes_without_the_welcome() {
        let sample = parsed(&["--sample"]);
        assert!(!sample.welcome);
        assert!(matches!(sample.source, host::Source::Sample));
    }

    #[test]
    fn opened_by_formiga_there_is_no_welcome() {
        let visit = parsed(&[LAUNCH_ARGUMENT, "/tmp/0123456789abcdef0123456789abcdef"]);
        assert!(!visit.welcome);
        assert!(matches!(visit.source, host::Source::Session(_)));
    }

    #[test]
    fn a_colony_or_a_picture_asked_for_comes_without_the_welcome() {
        for args in [
            &["--from-save", "colony.json"][..],
            &["--snapshot", "roster.json"],
            &["--review", "station", "--render-window", "a.png"],
            &["--render-window", "a.png"],
            &["--snap", "a.png"],
            &["--render-board", "a.png"],
        ] {
            assert!(!parsed(args).welcome, "{args:?}");
        }
    }
}
