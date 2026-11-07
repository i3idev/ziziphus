//! Database commands: `init`, `db new|list|use|info|del`.
//!
//! This module implements the database management surface of `ziziphus`:
//!
//! - `cmd_init` — initialize the ziziphus data directory
//! - `cmd_db` — dispatch `ziziphus db` subcommands
//!
//! Database files are SQLite databases named `<name>.db`, stored under
//! `<data-dir>/db/`. The active database name is recorded in `config`.

use regex::Regex;
use rusqlite::Connection;

use crate::db::{DB_NAME_RE, SCHEMA, active_name, db_names, db_path};
use crate::error::ZiziphusError;
use crate::render::{render_db_info, render_db_list_line};

/// Initialize the ziziphus data directory and config file.
pub fn cmd_init() -> Result<(), ZiziphusError> {
    std::fs::create_dir_all(crate::db::db_dir())?;

    let config = crate::db::config_path();

    if !config.exists() {
        std::fs::write(&config, "")?;
    }

    println!("ready: {}", crate::db::home_dir().display());

    let names = db_names()?;

    if names.is_empty() {
        println!("next : ziziphus db new <name>");
    }

    Ok(())
}

/// Dispatch the `ziziphus db` subcommand.
///
/// All subcommands require an initialized data directory.
pub fn cmd_db(_con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus db new|list|use|info|del [name]",
        ));
    }

    // Require initialization for every database command.
    if !crate::db::config_path().exists() {
        return Err(ZiziphusError::msg("not initialised. run: ziziphus init"));
    }

    let sub = &args[0];
    let rest = &args[1..];

    match sub.as_str() {
        "new" => cmd_db_new(rest),
        "list" => cmd_db_list(rest),
        "use" => cmd_db_use(rest),
        "info" => cmd_db_info(rest),
        "del" => cmd_db_del(rest),
        _ => Err(ZiziphusError::msg(format!("unknown: ziziphus db {}", sub))),
    }
}

/// Create a new SQLite database.
fn cmd_db_new(rest: &[String]) -> Result<(), ZiziphusError> {
    if rest.len() != 1 {
        return Err(ZiziphusError::msg("usage: ziziphus db new <name>"));
    }

    let name = &rest[0];

    let re = Regex::new(DB_NAME_RE).unwrap();

    if !re.is_match(name) {
        return Err(ZiziphusError::msg("db name: letters, digits, - and _ only"));
    }

    let path = db_path(name);

    if path.exists() {
        return Err(ZiziphusError::msg(format!("'{}' already exists", name)));
    }

    std::fs::create_dir_all(crate::db::db_dir())?;

    let con = Connection::open(&path)?;
    con.execute_batch(SCHEMA)?;
    drop(con);

    println!("created: {}", name);

    // Make the new database active only if no active database exists.
    let current = active_name(None).unwrap_or_default();

    if current.is_empty() {
        std::fs::write(crate::db::config_path(), name)?;
        println!("active : {}", name);
    }

    Ok(())
}

/// List all databases with entry counts and last-updated timestamps.
fn cmd_db_list(rest: &[String]) -> Result<(), ZiziphusError> {
    if !rest.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus db list"));
    }

    let names = db_names()?;

    if names.is_empty() {
        println!("no databases. run: ziziphus db new <name>");
        return Ok(());
    }

    let current = active_name(None).unwrap_or_default();

    println!("  NAME            ENTRIES  UPDATED");

    for name in names {
        let path = db_path(&name);
        let con = Connection::open(&path)?;

        let (count, updated): (i64, Option<String>) =
            con.query_row("SELECT COUNT(*), MAX(updated_at) FROM entries", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

        println!("{}", render_db_list_line(&current, &name, count, updated));
    }

    Ok(())
}

/// Set the active database by name.
fn cmd_db_use(rest: &[String]) -> Result<(), ZiziphusError> {
    if rest.len() != 1 {
        return Err(ZiziphusError::msg(
            "usage: ziziphus db use <name> (see: ziziphus db list)",
        ));
    }

    let name = &rest[0];
    let names = db_names()?;

    if !names.contains(name) {
        return Err(ZiziphusError::msg(format!(
            "no database '{}'. run: ziziphus db list",
            name
        )));
    }

    std::fs::write(crate::db::config_path(), name)?;

    println!("active : {}", name);

    Ok(())
}

/// Show database info.
fn cmd_db_info(rest: &[String]) -> Result<(), ZiziphusError> {
    if rest.len() > 1 {
        return Err(ZiziphusError::msg("usage: ziziphus db info [name]"));
    }

    let name = match rest.first() {
        Some(name) => name.clone(),
        None => active_name(None)?,
    };

    if name.is_empty() {
        return Err(ZiziphusError::msg(
            "no active db. run: ziziphus db new <name>",
        ));
    }

    let path = db_path(&name);

    if !path.exists() {
        return Err(ZiziphusError::msg(format!("no database '{}'", name)));
    }

    let con = Connection::open(&path)?;

    let current = active_name(None).unwrap_or_default();
    let is_active = name == current;

    let mut counts = Vec::new();

    for kind in ["word", "grammar", "style", "form"] {
        let count: i64 = con.query_row(
            "SELECT COUNT(*) FROM entries WHERE kind = ?1",
            [kind],
            |row| row.get(0),
        )?;

        counts.push((kind, count));
    }

    for line in render_db_info(&name, &path, is_active, &counts) {
        println!("{}", line);
    }

    Ok(())
}

/// Delete a database.
fn cmd_db_del(rest: &[String]) -> Result<(), ZiziphusError> {
    if rest.is_empty() || rest.len() > 2 {
        return Err(ZiziphusError::msg("usage: ziziphus db del <name> [--yes]"));
    }

    let name = &rest[0];

    let skip_confirmation = match rest.get(1) {
        None => false,
        Some(flag) if flag == "--yes" => true,
        Some(flag) => {
            return Err(ZiziphusError::msg(format!("unknown option: {}", flag)));
        }
    };

    let names = db_names()?;

    if !names.contains(name) {
        return Err(ZiziphusError::msg(format!("no database '{}'", name)));
    }

    if !skip_confirmation {
        eprint!("type '{}' to delete it: ", name);

        use std::io::{Write, stdin, stdout};

        stdout().flush()?;

        let mut input = String::new();
        stdin().read_line(&mut input)?;

        if input.trim() != name.as_str() {
            return Err(ZiziphusError::msg("cancelled"));
        }
    }

    std::fs::remove_file(db_path(name))?;

    let current = active_name(None).unwrap_or_default();

    if name == &current {
        std::fs::write(crate::db::config_path(), "")?;
    }

    println!("{}: {}", crate::style::warn("deleted"), name);

    Ok(())
}
