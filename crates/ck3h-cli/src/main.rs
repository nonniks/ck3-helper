//! `ck3h`: a thin command-line layer over `ck3h-core` (spec §4.7).

use ck3h_core::apply::{ApplyResult, DEFAULT_LAUNCHER_PROCESS_NAMES, apply_order};
use ck3h_core::calibrate::{CalibrationReport, calibrate, check_last_run};
use ck3h_core::conflicts::{
    ConflictLevel, ConflictReport, Contender, Source, Verdict, WinReason, analyze,
};
use ck3h_core::deps::{DepIssueKind, build_dep_graph};
use ck3h_core::error::CoreError;
use ck3h_core::index::ModIndex;
use ck3h_core::launcher::Playset;
use ck3h_core::paths::{RULES_FILE, config_dir, default_game_dir, default_user_dir};
use ck3h_core::playset::ModEntry;
use ck3h_core::session::{
    PlaysetSelector, analysis_lang, launcher_db, load_vanilla, read_conflicts_log,
};
use ck3h_core::sort::{MoveReason, SortProposal, load_rules, propose};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "ck3h",
    version,
    about = "Crusader Kings III mod conflict analysis"
)]
struct Cli {
    /// CK3 user directory. Default: `Documents/Paradox Interactive/Crusader Kings III` on
    /// Windows and macOS, `~/.local/share/Paradox Interactive/Crusader Kings III` on Linux.
    #[arg(long, global = true)]
    user_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List launcher playsets.
    Playsets {
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Show which mods override which.
    Conflicts {
        #[command(flatten)]
        target: Target,
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Propose a load order; `--apply` saves it as a new playset.
    Sort {
        #[command(flatten)]
        target: Target,
        /// Rules file (default: <config dir>/ck3-helper/rules.toml, which may be absent; an explicit
        /// path must exist).
        #[arg(long)]
        rules: Option<PathBuf>,
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Write the proposed order as a NEW playset (after a backup of the launcher database).
        /// The only flag that writes anything; refuses while the Paradox Launcher is running.
        #[arg(long)]
        apply: bool,
    },
    /// Compare the prediction with logs/database_conflicts.log.
    Calibrate {
        #[command(flatten)]
        target: Target,
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Args)]
struct Target {
    /// Playset name (default: the active playset).
    #[arg(long)]
    playset: Option<String>,
    /// Localization language (default: the game's language, else l_english).
    #[arg(long)]
    lang: Option<String>,
    /// Path to the game's `game/` folder, to include vanilla files (calibrate defaults to the
    /// Steam install).
    #[arg(long)]
    game: Option<PathBuf>,
}

enum Failure {
    Core(CoreError),
    Message(String),
    Output(io::Error),
}

impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Self {
        Failure::Output(e)
    }
}

impl From<CoreError> for Failure {
    fn from(e: CoreError) -> Self {
        Failure::Core(e)
    }
}

type Outcome = Result<(), Failure>;

fn exit_for(failure: &Failure) -> (u8, String) {
    match failure {
        Failure::Core(CoreError::LauncherRunning(names)) => (
            2,
            format!("Close the Paradox Launcher first: {}", names.join(", ")),
        ),
        Failure::Core(CoreError::ProcessListUnavailable) => (
            1,
            "Cannot check whether the Paradox Launcher is running: the process list is unavailable"
                .to_string(),
        ),
        Failure::Core(e) => (1, format!("error: {e}")),
        Failure::Message(m) => (1, format!("error: {m}")),
        // A reader that stops early (`ck3h conflicts | head`) is not an error.
        Failure::Output(e) if e.kind() == io::ErrorKind::BrokenPipe => (0, String::new()),
        Failure::Output(e) => (1, format!("error: cannot write output: {e}")),
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) if !e.use_stderr() => e.exit(), // --help, --version
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(1); // exit code 2 means "launcher running"
        }
    };
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let (code, message) = exit_for(&failure);
            if !message.is_empty() {
                let _ = writeln!(io::stderr(), "{message}");
            }
            ExitCode::from(code)
        }
    }
}

fn run(cli: Cli) -> Outcome {
    let user_dir = match cli.user_dir.or_else(default_user_dir) {
        Some(dir) => dir,
        None => {
            return Err(Failure::Message(
                "cannot determine the CK3 user directory; pass --user-dir".into(),
            ));
        }
    };
    let mut out = io::stdout().lock();
    let out = &mut out;
    match cli.command {
        Command::Playsets { json } => playsets(out, &user_dir, json),
        Command::Conflicts { target, json } => conflicts(out, &user_dir, &target, json),
        Command::Sort {
            target,
            rules,
            json,
            apply,
        } => sort(out, &user_dir, &target, rules, json, apply),
        Command::Calibrate { target, json } => calibrate_cmd(out, &user_dir, &target, json),
    }
}

fn print_json<T: Serialize>(out: &mut impl Write, value: &T) -> Outcome {
    let text = serde_json::to_string_pretty(value).map_err(|e| Failure::Message(e.to_string()))?;
    writeln!(out, "{text}")?;
    out.flush()?;
    Ok(())
}

struct Loaded {
    db_path: PathBuf,
    playset: Playset,
    mods: Vec<ModEntry>,
    vanilla: Option<ModIndex>,
}

fn load(user_dir: &Path, target: &Target) -> Result<Loaded, Failure> {
    let selector = match &target.playset {
        Some(name) => PlaysetSelector::ByName(name.clone()),
        None => PlaysetSelector::Active,
    };
    let lang = analysis_lang(user_dir, target.lang.as_deref());
    let loaded = ck3h_core::session::load(user_dir, &selector, &lang, &|_, _| {})?;
    let vanilla = target
        .game
        .as_deref()
        .map(|g| load_vanilla(g, &lang))
        .transpose()?;
    Ok(Loaded {
        db_path: loaded.db_path,
        playset: loaded.playset,
        mods: loaded.mods,
        vanilla,
    })
}

fn names(mods: &[ModEntry]) -> BTreeMap<&str, &str> {
    mods.iter()
        .map(|m| (m.id.as_str(), m.name.as_str()))
        .collect()
}

fn name_of<'a>(names: &BTreeMap<&'a str, &'a str>, id: &'a str) -> &'a str {
    names.get(id).copied().unwrap_or(id)
}

#[derive(Serialize)]
struct PlaysetRow {
    id: String,
    name: String,
    active: bool,
    mods: usize,
    enabled: usize,
}

fn playsets(out: &mut impl Write, user_dir: &Path, json: bool) -> Outcome {
    let (_, db) = launcher_db(user_dir)?;
    let mut rows = Vec::new();
    for p in db.playsets()? {
        let mods = db.playset_mods(&p.id)?;
        rows.push(PlaysetRow {
            enabled: mods.iter().filter(|m| m.enabled).count(),
            mods: mods.len(),
            active: p.is_active,
            id: p.id,
            name: p.name,
        });
    }
    if json {
        return print_json(out, &rows);
    }
    for r in &rows {
        let mark = if r.active { '*' } else { ' ' };
        writeln!(
            out,
            "{mark} {} ({} mods, {} enabled)",
            r.name, r.mods, r.enabled
        )?;
    }
    out.flush()?;
    Ok(())
}

fn source_name(names: &BTreeMap<&str, &str>, c: &Contender) -> String {
    match &c.source {
        Source::Mod(id) => name_of(names, id).to_string(),
        Source::Vanilla => "vanilla".to_string(),
    }
}

fn level_word(l: ConflictLevel) -> &'static str {
    match l {
        ConflictLevel::File => "file",
        ConflictLevel::Object => "object",
        ConflictLevel::Loc => "loc",
    }
}

fn reason_word(r: WinReason) -> &'static str {
    match r {
        WinReason::LowerInPlayset => "lower in playset",
        WinReason::LaterFilePath => "later file path",
        WinReason::EarlierFilePath => "earlier file path",
        WinReason::ReplaceFolder => "replace folder",
    }
}

fn print_conflicts(
    out: &mut impl Write,
    report: &ConflictReport,
    mods: &[ModEntry],
) -> io::Result<()> {
    let names = names(mods);
    for c in &report.conflicts {
        let verdict = match c.verdict {
            Verdict::Intended => "intended",
            Verdict::Suspicious => "suspicious",
            Verdict::Vanilla => "vanilla",
        };
        let losers: Vec<String> = c.losers.iter().map(|l| source_name(&names, l)).collect();
        writeln!(
            out,
            "[{verdict}] {} {} — winner: {} ({}); losers: {}",
            level_word(c.level),
            c.target,
            source_name(&names, &c.winner),
            reason_word(c.reason),
            losers.join(", ")
        )?;
    }
    for id in &report.missing_mods {
        writeln!(out, "missing on disk: {}", name_of(&names, id))?;
    }
    for issue in &report.dep_issues {
        let kind = match issue.kind {
            DepIssueKind::Missing => "missing",
            DepIssueKind::Ambiguous => "ambiguous",
        };
        writeln!(
            out,
            "dependency {kind}: {} -> {}",
            name_of(&names, &issue.mod_id),
            issue.dependency
        )?;
    }
    for (id, paths) in &report.replace_paths {
        for path in paths {
            writeln!(
                out,
                "replaces vanilla folder: {} -> {path}",
                name_of(&names, id)
            )?;
        }
    }
    for id in &report.compatches {
        writeln!(out, "compatch: {}", name_of(&names, id))?;
    }
    Ok(())
}

fn conflicts(out: &mut impl Write, user_dir: &Path, target: &Target, json: bool) -> Outcome {
    let l = load(user_dir, target)?;
    let report = analyze(&l.mods, &build_dep_graph(&l.mods), l.vanilla.as_ref());
    if json {
        return print_json(out, &report);
    }
    print_conflicts(out, &report, &l.mods)?;
    print_warnings(out, &l)?;
    out.flush()?;
    Ok(())
}

/// One line per index warning (spec §5): broken files do not stop the analysis, but are shown.
fn print_warnings(out: &mut impl Write, l: &Loaded) -> io::Result<()> {
    let mut sources: Vec<(&str, &ModIndex)> = l
        .mods
        .iter()
        .filter_map(|m| Some((m.name.as_str(), m.index.as_ref()?)))
        .collect();
    if let Some(v) = &l.vanilla {
        sources.push(("vanilla", v));
    }
    for (name, index) in sources {
        for w in &index.warnings {
            if w.file.is_empty() {
                writeln!(out, "warning: {name}: {}", w.message)?;
            } else {
                writeln!(out, "warning: {name}: {}: {}", w.file, w.message)?;
            }
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct SortOutput<'a> {
    #[serde(flatten)]
    proposal: &'a SortProposal,
    applied: Option<&'a ApplyResult>,
}

fn reason_text(
    names: &BTreeMap<&str, &str>,
    proposal: &SortProposal,
    mod_id: &str,
    reason: &MoveReason,
) -> String {
    match reason {
        MoveReason::Dependency { on } => format!("depends on {}", name_of(names, on)),
        MoveReason::RequiredBy { by } => format!("required by {}", name_of(names, by)),
        MoveReason::UserRule { other, note } => {
            let at = |id: &str| proposal.order.iter().position(|o| o == id);
            let side = if at(mod_id) < at(other) {
                "before"
            } else {
                "after"
            };
            let mut text = format!("your rule: {side} {}", name_of(names, other));
            if let Some(note) = note {
                text.push_str(&format!(" — {note}"));
            }
            text
        }
    }
}

fn sort(
    out: &mut impl Write,
    user_dir: &Path,
    target: &Target,
    rules: Option<PathBuf>,
    json: bool,
    apply: bool,
) -> Outcome {
    let l = load(user_dir, target)?;
    let rules = match rules {
        Some(path) if !path.is_file() => {
            return Err(Failure::Message(format!(
                "rules file not found: {}",
                path.display()
            )));
        }
        Some(path) => load_rules(&path)?,
        // The default location may be absent; `load_rules` then means "no rules".
        None => match config_dir() {
            Some(d) => load_rules(&d.join(RULES_FILE))?,
            None => Vec::new(),
        },
    };
    let deps = build_dep_graph(&l.mods);
    let proposal = propose(&l.mods, &deps, &rules, l.vanilla.as_ref())?;
    let nothing_to_apply = proposal.moves.is_empty();
    let applied = if apply && !nothing_to_apply {
        let applied = apply_order(
            &l.db_path,
            &l.playset.id,
            &proposal.order,
            chrono::Local::now(),
            DEFAULT_LAUNCHER_PROCESS_NAMES,
        )?;
        // The write is done: say so even if stdout is already gone.
        let _ = writeln!(io::stderr(), "{}", applied_line(&applied));
        Some(applied)
    } else {
        None
    };
    if json {
        return print_json(
            out,
            &SortOutput {
                proposal: &proposal,
                applied: applied.as_ref(),
            },
        );
    }
    let names = names(&l.mods);
    if nothing_to_apply {
        writeln!(
            out,
            "moves: 0 (the current order already satisfies dependencies and rules)"
        )?;
    } else {
        writeln!(out, "moves: {}", proposal.moves.len())?;
    }
    for m in &proposal.moves {
        let reasons: Vec<String> = m
            .reasons
            .iter()
            .map(|r| reason_text(&names, &proposal, &m.mod_id, r))
            .collect();
        writeln!(
            out,
            "move {}: #{} -> #{} ({})",
            name_of(&names, &m.mod_id),
            m.from + 1,
            m.to + 1,
            reasons.join(", ")
        )?;
    }
    writeln!(out, "winner changes: {}", proposal.winner_changes.len())?;
    for w in &proposal.winner_changes {
        writeln!(
            out,
            "winner change: {} {}: {} -> {}",
            level_word(w.level),
            w.target,
            source_name(&names, &w.before),
            source_name(&names, &w.after)
        )?;
    }
    for r in &proposal.unresolved_rules {
        writeln!(out, "unresolved rule: {} before {}", r.before, r.after)?;
    }
    print_warnings(out, &l)?;
    if apply && nothing_to_apply {
        writeln!(out, "nothing to apply: the order is already sorted")?;
    }
    if let Some(a) = &applied {
        writeln!(out, "{}", applied_line(a))?;
    }
    out.flush()?;
    Ok(())
}

fn applied_line(a: &ApplyResult) -> String {
    format!(
        "created playset \"{}\"; backup: {}",
        a.new_playset_name,
        a.backup_path.display()
    )
}

fn print_calibration(
    out: &mut impl Write,
    report: &CalibrationReport,
    mods: &[ModEntry],
) -> io::Result<()> {
    writeln!(
        out,
        "matched: {}, mismatched: {}, unknown in log: {}, predicted but not logged: {}",
        report.matched,
        report.mismatched.len(),
        report.log_unknown.values().sum::<usize>(),
        report.predicted_not_logged.len()
    )?;
    for m in &report.mismatched {
        writeln!(
            out,
            "mismatch: {} {}: log has {}, predicted {}",
            m.database, m.key, m.log_file, m.predicted_file
        )?;
    }
    if !report.logged_single_definition.is_empty() {
        writeln!(
            out,
            "logged with a single definition here: {}",
            report.logged_single_definition.values().sum::<usize>()
        )?;
        for (db, n) in &report.logged_single_definition {
            writeln!(out, "  {db}: {n}")?;
        }
    }
    if !report.log_unknown.is_empty() {
        writeln!(out, "unknown by database:")?;
        for (db, n) in &report.log_unknown {
            writeln!(out, "  {db}: {n}")?;
        }
    }
    if report.truncated {
        writeln!(out, "warning: log is truncated")?;
    }
    if let Some(rc) = &report.run_check
        && (!rc.not_in_last_run.is_empty() || !rc.only_in_last_run.is_empty())
    {
        writeln!(out, "warning: log is from a different set of mods")?;
        let names = names(mods);
        for id in &rc.not_in_last_run {
            writeln!(out, "  not in last run: {}", name_of(&names, id))?;
        }
        for shown in &rc.only_in_last_run {
            writeln!(out, "  only in last run: {shown}")?;
        }
    }
    Ok(())
}

fn calibrate_cmd(out: &mut impl Write, user_dir: &Path, target: &Target, json: bool) -> Outcome {
    let game = match &target.game {
        Some(path) => path.clone(),
        None => default_game_dir().ok_or_else(|| {
            Failure::Message("calibrate needs the game folder: pass --game <path to game/>".into())
        })?,
    };
    let target = Target {
        playset: target.playset.clone(),
        lang: target.lang.clone(),
        game: Some(game),
    };
    let l = load(user_dir, &target)?;
    let log = read_conflicts_log(user_dir).map_err(|e| match e {
        CoreError::Io(e) => Failure::Message(format!(
            "{}: {e}",
            user_dir
                .join("logs")
                .join("database_conflicts.log")
                .display()
        )),
        e => Failure::Core(e),
    })?;
    let run_check = check_last_run(user_dir, &l.mods);
    let report = calibrate(&log, &l.mods, l.vanilla.as_ref(), run_check);
    if json {
        return print_json(out, &report);
    }
    print_calibration(out, &report, &l.mods)?;
    print_warnings(out, &l)?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_running_exits_2_with_names() {
        let f = Failure::Core(CoreError::LauncherRunning(vec!["Paradox Launcher".into()]));
        assert_eq!(
            exit_for(&f),
            (
                2,
                "Close the Paradox Launcher first: Paradox Launcher".to_string()
            )
        );
    }

    #[test]
    fn broken_pipe_is_a_quiet_success() {
        let f = Failure::Output(io::ErrorKind::BrokenPipe.into());
        assert_eq!(exit_for(&f), (0, String::new()));
        let f = Failure::Output(io::ErrorKind::PermissionDenied.into());
        assert_eq!(exit_for(&f).0, 1);
    }

    #[test]
    fn other_errors_exit_1() {
        let f = Failure::Core(CoreError::PlaysetNotFound("Nope".into()));
        assert_eq!(
            exit_for(&f),
            (1, "error: playset not found: Nope".to_string())
        );
        assert_eq!(exit_for(&Failure::Message("x".into())).0, 1);
    }
}
