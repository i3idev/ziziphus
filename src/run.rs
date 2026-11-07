//! Command dispatch for the clap-based CLI.
//!
//! This module connects `crate::cli` argument definitions to
//! `crate::cmd` command implementations.
//!
//! Responsibilities:
//! - Apply global flags.
//! - Open the selected database.
//! - Translate clap arguments into legacy argument vectors.
//! - Handle JSON output for supported commands.
//! - Provide `run_internal` for command files.

use crate::cli::{Cli, CollAction, Command, DbCommand, FamilyAction, GramCommand, SenseAction};
use crate::cmd::db::{cmd_db, cmd_init};
use crate::cmd::grammar::cmd_group;
use crate::cmd::load::cmd_load;
use crate::cmd::tr_syn_ant::{cmd_syn_ant, cmd_tr};
use crate::cmd::word::{
    cmd_add, cmd_coll, cmd_del, cmd_ex_word, cmd_family, cmd_find, cmd_list, cmd_sense,
    cmd_set_word, cmd_show,
};
use crate::error::ZiziphusError;
use rusqlite::Connection;

/// Run a clap-parsed CLI.
///
/// Applies global flags before dispatching the selected command.
pub fn run_cli(cli: Cli) -> Result<(), ZiziphusError> {
    crate::model::set_quiet(cli.quiet);
    crate::style::set_mode(if cli.no_color {
        crate::style::Mode::Never
    } else if cli.color {
        crate::style::Mode::Always
    } else {
        crate::style::Mode::Auto
    });

    match cli.command {
        Some(cmd) => run_clap(cmd, cli.db.as_deref(), cli.vault.as_deref(), cli.json),
        None => {
            print_usage();
            Ok(())
        }
    }
}

/// Run a clap-parsed command.
///
/// Applies the vault override and dispatches the command.
pub fn run_clap(
    cmd: Command,
    db_override: Option<&str>,
    vault_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    if let Some(vault) = vault_override {
        crate::db::set_vault(Some(vault));
    }

    // JSON is currently supported only by these command paths.
    if json_output
        && !matches!(
            &cmd,
            Command::Show(_) | Command::Find(_) | Command::List(_) | Command::External(_)
        )
    {
        return Err(ZiziphusError::msg(
            "--json is supported only by show, find, list, and direct word lookup",
        ));
    }

    match cmd {
        Command::Init => cmd_init(),

        Command::Db(sub) => run_db(sub),

        Command::Add(args) => run_add(args, db_override),

        Command::Show(args) => run_show(args, db_override, json_output),

        Command::Find(args) => run_find(args, db_override, json_output),

        Command::List(args) => run_list(args, db_override, json_output),

        Command::Del(args) => run_del(args, db_override),

        Command::Tr(args) => run_tr(args, db_override),

        Command::Syn(args) => run_syn_ant("syn", args, db_override),

        Command::Ant(args) => run_syn_ant("ant", args, db_override),

        Command::Ex(args) => run_ex(args, db_override),

        Command::Sense(args) => run_sense(args, db_override),

        Command::Coll(args) => run_coll(args, db_override),

        Command::Family(args) => run_family(args, db_override),

        Command::Pos(args) => run_field("pos", args, db_override),

        Command::Lvl(args) => run_field("lvl", args, db_override),

        Command::Pron(args) => run_field("pron", args, db_override),

        Command::Def(args) => run_field("def", args, db_override),

        Command::Note(args) => run_field("note", args, db_override),

        Command::Gram(sub) => run_gram("gram", sub, db_override, json_output),

        Command::Style(sub) => run_gram("style", sub, db_override, json_output),

        Command::Form(sub) => run_gram("form", sub, db_override, json_output),

        Command::Load(args) => cmd_load(&[args.file], db_override),

        Command::Topics(args) => crate::topics::print_topic(args.topic.as_deref()),

        Command::External(args) => run_external(args, db_override, json_output),
    }
}

/// Dispatch a direct word lookup or a legacy external command.
fn run_external(
    args: Vec<String>,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    let first = &args[0];

    match first.as_str() {
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }

        _ => {
            let con = crate::db::connect(db_override)?;

            if json_output {
                print_entry_json(&con, std::slice::from_ref(first))
            } else {
                cmd_show(&con, &args)
            }
        }
    }
}

/// Dispatch database management commands.
///
/// These commands operate on the data directory rather than on the
/// currently selected database. The connection is only required by
/// the legacy handler interface.
fn run_db(sub: DbCommand) -> Result<(), ZiziphusError> {
    let con = Connection::open_in_memory()?;

    match sub {
        DbCommand::New { name } => cmd_db(&con, &["new".into(), name]),

        DbCommand::List => cmd_db(&con, &["list".into()]),

        DbCommand::Use { name } => cmd_db(&con, &["use".into(), name]),

        DbCommand::Info { name } => {
            let mut argv = vec!["info".to_string()];

            if let Some(name) = name {
                argv.push(name);
            }

            cmd_db(&con, &argv)
        }

        DbCommand::Del { name, yes } => {
            let mut argv = vec!["del".to_string(), name];

            if yes {
                argv.push("--yes".into());
            }

            cmd_db(&con, &argv)
        }
    }
}

/// Dispatch `add`.
///
/// Translates clap arguments into the legacy `cmd_add` format.
fn run_add(args: crate::cli::AddArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];
    argv.extend(args.translations);

    if let Some(lang) = args.lang {
        argv.push("--lang".into());
        argv.push(lang);
    }

    if let Some(pos) = args.pos {
        argv.push("-p".into());
        argv.push(pos);
    }

    if let Some(level) = args.level {
        argv.push("-l".into());
        argv.push(level);
    }

    cmd_add(&con, &argv)
}

/// Dispatch `show`.
fn run_show(
    args: crate::cli::ShowArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];
    argv.extend(args.sections);

    if json_output {
        print_entry_json(&con, &argv)
    } else {
        cmd_show(&con, &argv)
    }
}

/// Dispatch `find`.
fn run_find(
    args: crate::cli::FindArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.query];

    if args.all {
        argv.push("--all".into());
    }

    if json_output {
        print_find_json(&con, &argv)
    } else {
        cmd_find(&con, &argv, db_override)
    }
}

/// Dispatch `list`.
fn run_list(
    args: crate::cli::ListArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = Vec::new();

    if let Some(pos) = args.pos {
        argv.push("--pos".into());
        argv.push(pos);
    }

    if let Some(level) = args.lvl {
        argv.push("--lvl".into());
        argv.push(level);
    }

    if let Some(lang) = args.lang {
        argv.push("--lang".into());
        argv.push(lang);
    }

    if json_output {
        print_list_json(&con, &argv)
    } else {
        cmd_list(&con, &argv)
    }
}

/// Dispatch `del`.
fn run_del(args: crate::cli::DelArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if args.yes {
        argv.push("--yes".into());
    }

    cmd_del(&con, &argv)
}

/// Dispatch `tr`.
///
/// The legacy handler expects the language argument in `-LANG` form.
fn run_tr(args: crate::cli::TrArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if let Some(lang) = args.lang {
        argv.push(format!("-{lang}"));
    }

    argv.extend(args.ops);

    cmd_tr(&con, &argv)
}

/// Dispatch `syn` and `ant`.
fn run_syn_ant(
    field: &str,
    args: crate::cli::SynAntArgs,
    db_override: Option<&str>,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];
    argv.extend(args.ops);

    cmd_syn_ant(&con, field, &argv)
}

/// Dispatch word examples.
fn run_ex(args: crate::cli::ExArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if args.delete {
        let number = args
            .number
            .ok_or_else(|| ZiziphusError::msg("--delete requires --number N"))?;

        if !args.texts.is_empty() {
            return Err(ZiziphusError::msg(
                "--delete cannot be combined with example text",
            ));
        }

        argv.push(format!("-{number}"));
    } else if let Some(number) = args.number {
        argv.push(number.to_string());
    }

    argv.extend(args.texts);

    cmd_ex_word(&con, &argv)
}

/// Dispatch sense commands.
fn run_sense(args: crate::cli::SenseArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if let Some(action) = args.action {
        match action {
            SenseAction::Add { definition, note } => {
                argv.push("add".into());
                argv.push(definition);

                if let Some(note) = note {
                    argv.push("--note".into());
                    argv.push(note);
                }
            }

            SenseAction::Del { number } => {
                argv.push("del".into());
                argv.push(number.to_string());
            }

            SenseAction::Show { number } => {
                argv.push("show".into());
                argv.push(number.to_string());
            }
        }
    }

    cmd_sense(&con, &argv)
}

/// Dispatch collocation commands.
fn run_coll(args: crate::cli::CollArgs, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if let Some(action) = args.action {
        match action {
            CollAction::Add { text } => {
                argv.push("add".into());
                argv.push(text);
            }

            CollAction::Del { number } => {
                argv.push("del".into());
                argv.push(number.to_string());
            }
        }
    }

    cmd_coll(&con, &argv)
}

/// Dispatch word-family commands.
fn run_family(
    args: crate::cli::FamilyArgs,
    db_override: Option<&str>,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if let Some(action) = args.action {
        match action {
            FamilyAction::Add { word, pos } => {
                argv.push("add".into());
                argv.push(word);

                if let Some(pos) = pos {
                    argv.push("-p".into());
                    argv.push(pos);
                }
            }

            FamilyAction::Del { word } => {
                argv.push("del".into());
                argv.push(word);
            }
        }
    }

    cmd_family(&con, &argv)
}

/// Dispatch word field get/set commands.
fn run_field(
    field: &str,
    args: crate::cli::FieldArgs,
    db_override: Option<&str>,
) -> Result<(), ZiziphusError> {
    let con = crate::db::connect(db_override)?;

    let mut argv = vec![args.word];

    if let Some(value) = args.value {
        argv.push(value);
    }

    cmd_set_word(&con, field, &argv)
}

/// Dispatch grammar, style and form commands.
///
/// Legacy grammar handlers use `-N` to delete an example or error
/// entry at position N.
fn run_gram(
    kind: &str,
    sub: GramCommand,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), ZiziphusError> {
    if json_output {
        return Err(ZiziphusError::msg(
            "--json is not supported for grammar, style, or form commands",
        ));
    }

    let con = crate::db::connect(db_override)?;

    let mut argv = vec![sub_to_str(&sub).to_string()];

    match sub {
        GramCommand::Add(args) => {
            argv.push(args.name);

            if let Some(cat) = args.cat {
                argv.push("--cat".into());
                argv.push(cat);
            }

            if let Some(lvl) = args.lvl {
                argv.push("--lvl".into());
                argv.push(lvl);
            }

            if let Some(rule) = args.rule {
                argv.push("--rule".into());
                argv.push(rule);
            }

            if let Some(desc) = args.desc {
                argv.push("--desc".into());
                argv.push(desc);
            }
        }

        GramCommand::List { prefix } => {
            if let Some(prefix) = prefix {
                argv.push(prefix);
            }
        }

        GramCommand::Show { name } => {
            argv.push(name);
        }

        GramCommand::Del { name } => {
            argv.push(name);
        }

        GramCommand::Rule(args) => {
            argv.push(args.name);

            if let Some(value) = args.value {
                argv.push(value);
            }
        }

        GramCommand::Desc(args) => {
            argv.push(args.name);

            if let Some(value) = args.value {
                argv.push(value);
            }
        }

        GramCommand::Cat(args) => {
            argv.push(args.name);

            if let Some(value) = args.value {
                argv.push(value);
            }
        }

        GramCommand::Lvl(args) => {
            argv.push(args.name);

            if let Some(value) = args.value {
                argv.push(value);
            }
        }

        GramCommand::Note(args) => {
            argv.push(args.name);

            if let Some(value) = args.value {
                argv.push(value);
            }
        }

        GramCommand::Ex(args) => {
            argv.push(args.name);

            if args.delete {
                let number = args
                    .number
                    .ok_or_else(|| ZiziphusError::msg("--delete requires --number N"))?;

                if !args.texts.is_empty() {
                    return Err(ZiziphusError::msg(
                        "--delete cannot be combined with example text",
                    ));
                }

                argv.push(format!("-{number}"));
            } else if let Some(number) = args.number {
                argv.push(number.to_string());
            }

            argv.extend(args.texts);
        }

        GramCommand::Err(args) => {
            argv.push(args.name);

            if args.delete {
                let number = args
                    .number
                    .ok_or_else(|| ZiziphusError::msg("--delete requires --number N"))?;

                if !args.texts.is_empty() {
                    return Err(ZiziphusError::msg(
                        "--delete cannot be combined with error text",
                    ));
                }

                argv.push(format!("-{number}"));
            } else if let Some(number) = args.number {
                argv.push(number.to_string());
            }

            argv.extend(args.texts);
        }
    }

    cmd_group(&con, kind, &argv)
}

/// Convert a grammar subcommand to its legacy name.
fn sub_to_str(sub: &GramCommand) -> &'static str {
    match sub {
        GramCommand::Add(_) => "add",
        GramCommand::List { .. } => "list",
        GramCommand::Show { .. } => "show",
        GramCommand::Del { .. } => "del",
        GramCommand::Rule(_) => "rule",
        GramCommand::Desc(_) => "desc",
        GramCommand::Cat(_) => "cat",
        GramCommand::Lvl(_) => "lvl",
        GramCommand::Note(_) => "note",
        GramCommand::Ex(_) => "ex",
        GramCommand::Err(_) => "err",
    }
}

/// Run a command from a legacy argument vector.
///
/// Used by command files loaded through `ziziphus load`.
pub fn run_internal(argv: Vec<String>, db_override: Option<&str>) -> Result<(), ZiziphusError> {
    if argv.is_empty() {
        return Ok(());
    }

    match argv[0].as_str() {
        "help" | "-h" | "--help" => {
            print_usage();
            return Ok(());
        }
        "load" => {
            return Err(ZiziphusError::msg("load inside a file is not allowed"));
        }
        _ => {}
    }

    let mut args = Vec::with_capacity(argv.len() + 1);
    args.push("ziziphus".to_string());
    args.extend(argv);

    let cli = match <crate::cli::Cli as clap::Parser>::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                print!("{error}");
                return Ok(());
            }
            _ => return Err(ZiziphusError::msg(error.to_string())),
        },
    };

    let Some(command) = cli.command else {
        print_usage();
        return Ok(());
    };

    if matches!(&command, crate::cli::Command::Load(_)) {
        return Err(ZiziphusError::msg("load inside a file is not allowed"));
    }

    // Preserve quiet mode inherited from `load`, but honour an
    // explicit --quiet flag in command files.
    if cli.quiet {
        crate::model::set_quiet(true);
    }

    if cli.no_color {
        crate::style::set_mode(crate::style::Mode::Never);
    } else if cli.color {
        crate::style::set_mode(crate::style::Mode::Always);
    }

    run_clap(
        command,
        db_override.or(cli.db.as_deref()),
        cli.vault.as_deref(),
        cli.json,
    )
}

/// JSON output for `list`.
///
/// Supports `--pos`, `--lvl`, and `--lang`.
fn print_list_json(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    use serde_json::Value;

    let mut pos_filter: Option<String> = None;
    let mut lvl_filter: Option<String> = None;
    let mut lang_filter: Option<String> = None;

    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--pos" | "--lvl" | "--lang" => {
                let flag = args[i].as_str();

                let value = args
                    .get(i + 1)
                    .ok_or_else(|| ZiziphusError::msg(format!("missing value after {}", flag)))?;

                match flag {
                    "--pos" => pos_filter = Some(value.to_lowercase()),
                    "--lvl" => lvl_filter = Some(value.to_lowercase()),
                    "--lang" => lang_filter = Some(value.to_lowercase()),
                    _ => unreachable!(),
                }

                i += 2;
            }

            other => {
                return Err(ZiziphusError::msg(format!(
                    "unknown list option: {}",
                    other
                )));
            }
        }
    }

    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind = 'word'")?;

    let mut rows = stmt.query([])?;
    let mut records: Vec<Value> = Vec::new();

    while let Some(row) = rows.next()? {
        let raw: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&raw)?;

        if let Some(ref level) = lvl_filter {
            let value = rec.get("level").and_then(Value::as_str).unwrap_or("");

            if value.to_lowercase() != *level {
                continue;
            }
        }

        if let Some(ref pos) = pos_filter {
            let value = rec.get("pos").and_then(Value::as_str).unwrap_or("");

            if value.to_lowercase() != *pos {
                continue;
            }
        }

        if let Some(ref lang) = lang_filter {
            let title = crate::model::title_of(&rec);

            if crate::model::lang_of(&title) != lang.as_str() {
                continue;
            }
        }

        records.push(rec);
    }

    println!("{}", serde_json::to_string_pretty(&records)?);

    Ok(())
}

/// JSON output for `find`.
///
/// `--all` is not implemented for JSON search yet. Reject it rather
/// than silently returning results from only the current database.
fn print_find_json(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.iter().any(|arg| arg == "--all") {
        return Err(ZiziphusError::msg(
            "`find --all --json` is not supported yet",
        ));
    }

    let query_parts: Vec<&str> = args.iter().map(String::as_str).collect();

    if query_parts.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus find <text> --json"));
    }

    let query = query_parts.join(" ");

    let records = crate::db::search_records(con, &query)?;

    println!("{}", serde_json::to_string_pretty(&records)?);

    Ok(())
}

/// JSON output for `show` and direct word lookup.
fn print_entry_json(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    use crate::db::load_rec_fallback;

    let name = args
        .first()
        .ok_or_else(|| ZiziphusError::msg("usage: ziziphus show <word> --json"))?;

    let mut found = Vec::new();

    for kind in ["word", "grammar", "style", "form"] {
        if let Ok(record) = load_rec_fallback(con, kind, name) {
            found.push(record);
        }
    }

    match found.len() {
        0 => Err(ZiziphusError::msg(format!("not found: {}", name))),

        1 => {
            println!("{}", serde_json::to_string_pretty(&found[0])?);

            Ok(())
        }

        _ => {
            println!("{}", serde_json::to_string_pretty(&found)?);

            Ok(())
        }
    }
}

/// Print the fallback usage summary.
fn print_usage() {
    println!(
        r#"ziziphus - word and grammar notebook

USAGE:
    ziziphus <COMMAND> [OPTIONS]

COMMANDS:
    init                    Initialize config directory
    db <SUBCOMMAND>         Database management
    add <WORD> [TRANS...]   Add a word
    show <WORD> [SECTIONS]  Show entry
    find <QUERY>            Search entries
    list                    List entries
    del <WORD>              Delete entry
    tr <WORD> [OPS]         Manage translations
    syn <WORD> [OPS]        Manage synonyms
    ant <WORD> [OPS]        Manage antonyms
    ex <WORD> [TEXTS]       Manage examples
    sense <WORD> <ACTION>   Manage senses
    coll <WORD> <ACTION>    Manage collocations
    family <WORD> <ACTION>  Manage word family
    pos|lvl|pron|def|note   Set/get field
    gram|style|form <CMD>   Grammar/style/form
    topics [TOPIC]          Extended help
    load <FILE>             Load commands from file

OPTIONS:
    --db <NAME>             Use specific database
    --vault <PATH>          Path to vault folder containing .ziziphus/ data
    -v <PATH>               Short form of --vault
    --json                  Output supported commands as JSON
    -q, --quiet             Suppress non-error output
    --no-color              Disable ANSI colors
    -h, --help              Show help
    -V, --version           Show version
"#
    );
}
