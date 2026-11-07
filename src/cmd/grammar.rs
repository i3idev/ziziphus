//! Grammar / style / form commands: `add`, `list`, `del`, `err`, field
//! setters, `show`.
//!
//! `gram`, `style`, and `form` share the same command structure and
//! differ only by the entry `kind` stored in the database. Each entry
//! has: title, rule, desc, cat, level, notes, examples, errors.

use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;

use crate::cmd::ex::cmd_ex;
use crate::db::{load_rec, load_rec_fallback, save_rec};
use crate::error::ZiziphusError;
use crate::model::{clean, key_of, new_grammar, title_of};
use crate::render::{render_entry, render_gram_list};

/// Grammar / Style / Form field mapping.
const GRAM_FIELDS: &[(&str, &str)] = &[
    ("rule", "rule"),
    ("desc", "desc"),
    ("cat", "cat"),
    ("lvl", "level"),
    ("note", "notes"),
];

/// `ziziphus gram|style|form add <name> [--cat C] [--lvl L] [--rule R] [--desc D]`
pub fn cmd_gadd(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    let (pos, opts) = parse_flags(
        args,
        &[
            ("--cat", "cat"),
            ("-c", "cat"),
            ("--lvl", "level"),
            ("-l", "level"),
            ("--rule", "rule"),
            ("-r", "rule"),
            ("--desc", "desc"),
            ("-d", "desc"),
        ],
    )?;

    if pos.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} add <name> [--cat c] [--lvl l] [--rule r] [--desc d]",
            crate::model::label(kind)
        )));
    }

    // Allow both quoted names and names split across positional arguments.
    let raw_name = pos.join(" ");
    let name = if raw_name.contains(' ') {
        raw_name.trim().to_string()
    } else {
        clean(&raw_name)
    };

    if name.is_empty() {
        return Err(ZiziphusError::msg("name cannot be empty"));
    }

    let existing = load_rec(con, kind, &name)?;
    let created = existing.is_none();

    let mut rec = match existing {
        Some(rec) => rec,
        None => new_grammar(kind, &name),
    };

    for (key, value) in opts {
        if value.is_empty() {
            rec.as_object_mut()
                .ok_or_else(|| ZiziphusError::msg("invalid entry data"))?
                .remove(&key);
        } else {
            rec[&key] = Value::String(value);
        }
    }

    save_rec(con, kind, &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        println!("{}: {}", if created { "added" } else { "updated" }, name);
    }

    Ok(())
}

/// `ziziphus gram|style|form list [prefix]`
pub fn cmd_glist(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.len() > 1 {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} list [prefix]",
            crate::model::label(kind)
        )));
    }

    let prefix = args.first().map(|arg| key_of(arg)).unwrap_or_default();

    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind = ?1 ORDER BY lower(title)")?;

    let mut rows = stmt.query([kind])?;
    let mut count = 0;

    while let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&json_str)?;

        let key = key_of(&title_of(&rec));

        if !prefix.is_empty() && key != prefix && !key.starts_with(&format!("{}.", prefix)) {
            continue;
        }

        println!("{}", render_gram_list(&rec));
        count += 1;
    }

    println!("-- {} {} entries", count, crate::model::label(kind));

    Ok(())
}

/// `ziziphus gram|style|form del <name>`
pub fn cmd_gdel(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} del <name>",
            crate::model::label(kind)
        )));
    }

    let name = args.join(" ");
    let rec = load_rec_fallback(con, kind, &name)?;
    let title = title_of(&rec);

    con.execute(
        "DELETE FROM entries WHERE id = ?1",
        [crate::model::entry_id(kind, &title)],
    )?;

    if !crate::model::STATE.lock().unwrap().quiet {
        println!("{}: {}", crate::style::warn("deleted"), clean(&name));
    }

    Ok(())
}

/// `ziziphus gram|style|form err <name> ["text"] | -N`
pub fn cmd_gerr(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} err <name> [\"text\"] [-N]",
            crate::model::label(kind)
        )));
    }

    let name = &args[0];
    let rest = &args[1..];

    let mut rec = load_rec_fallback(con, kind, name)?;

    // Initialize the field only when it does not exist.
    if rec.get("errors").is_none() {
        rec["errors"] = Value::Array(Vec::new());
    }

    if !rec["errors"].is_array() {
        return Err(ZiziphusError::msg(
            "invalid entry data: errors must be an array",
        ));
    }

    // List errors.
    if rest.is_empty() {
        let errors = rec["errors"]
            .as_array()
            .ok_or_else(|| ZiziphusError::msg("invalid errors array"))?;

        if errors.is_empty() {
            println!("no errors listed");
        } else {
            for (index, error) in errors.iter().enumerate() {
                if let Some(text) = error.as_str() {
                    println!(" {}> {}", index + 1, text);
                }
            }
        }

        return Ok(());
    }

    // Edit one error by its 1-based index: N "text".
    if let Some(number_text) = rest.first()
        && !number_text.is_empty()
        && number_text.chars().all(|c| c.is_ascii_digit())
    {
        if rest.len() != 2 {
            return Err(ZiziphusError::msg("usage: err <name> N \"text\""));
        }

        let number = number_text
            .parse::<usize>()
            .map_err(|_| ZiziphusError::msg("invalid error number"))?;

        {
            let errors = rec["errors"]
                .as_array_mut()
                .ok_or_else(|| ZiziphusError::msg("invalid errors array"))?;

            check_n(errors, number, "error")?;
            errors[number - 1] = Value::String(rest[1].clone());
        }

        save_rec(con, kind, &mut rec)?;

        if !crate::model::STATE.lock().unwrap().quiet {
            println!("{} error #{}", crate::style::ok("ok:"), number);
        }

        return Ok(());
    }

    // Delete one error by its 1-based index.
    if rest.len() == 1
        && let Some(number_text) = rest[0].strip_prefix('-')
    {
        if number_text.is_empty() || !number_text.chars().all(|c| c.is_ascii_digit()) {
            return Err(ZiziphusError::msg("usage: err <name> [\"text\"] [-N]"));
        }

        let number = number_text
            .parse::<usize>()
            .map_err(|_| ZiziphusError::msg("invalid error number"))?;

        {
            let errors = rec["errors"]
                .as_array_mut()
                .ok_or_else(|| ZiziphusError::msg("invalid errors array"))?;

            check_n(errors, number, "error")?;
            errors.remove(number - 1);
        }

        save_rec(con, kind, &mut rec)?;

        if !crate::model::STATE.lock().unwrap().quiet {
            let count = rec["errors"].as_array().map(Vec::len).unwrap_or(0);

            println!("{} {} error(s)", crate::style::ok("ok:"), count);
        }

        return Ok(());
    }

    // Add a new error.
    let text = rest.join(" ").trim().to_string();

    if text.is_empty() {
        return Err(ZiziphusError::msg("error text cannot be empty"));
    }

    {
        let errors = rec["errors"]
            .as_array_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid errors array"))?;

        errors.push(Value::String(text));
    }

    save_rec(con, kind, &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        let count = rec["errors"].as_array().map(Vec::len).unwrap_or(0);

        println!("{} {} error(s)", crate::style::ok("ok:"), count);
    }

    Ok(())
}

/// `ziziphus gram|style|form rule|desc|cat|lvl|note <name> [value]`
pub fn cmd_gset(
    con: &Connection,
    kind: &str,
    field: &str,
    args: &[String],
) -> Result<(), ZiziphusError> {
    let key = GRAM_FIELDS
        .iter()
        .find(|(field_name, _)| *field_name == field)
        .map(|(_, key)| *key)
        .ok_or_else(|| ZiziphusError::msg("unknown field"))?;

    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} <name> [value]",
            field
        )));
    }

    let name = &args[0];
    let mut rec = load_rec_fallback(con, kind, name)?;

    // Read field.
    if args.len() == 1 {
        let value = rec.get(key).and_then(Value::as_str).unwrap_or("-");
        println!("{}", value);
        return Ok(());
    }

    // Set or remove field.
    let value = args[1..].join(" ").trim().to_string();

    if value.is_empty() {
        rec.as_object_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid entry data"))?
            .remove(key);
    } else {
        rec[key] = Value::String(value.clone());
    }

    save_rec(con, kind, &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        println!("{} {} = {}", crate::style::ok("ok:"), field, value);
    }

    Ok(())
}

/// `ziziphus gram|style|form show <name> [sections...]`
pub fn cmd_gshow(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} show <name>",
            crate::model::label(kind)
        )));
    }

    let name = &args[0];
    let sections = args[1..].to_vec();

    let rec = load_rec_fallback(con, kind, name)?;

    println!("{}", render_entry(kind, &rec, &sections));

    Ok(())
}

/// Main entry point for `gram` / `style` / `form` group commands.
pub fn cmd_group(con: &Connection, grp: &str, args: &[String]) -> Result<(), ZiziphusError> {
    let kind = match grp {
        "gram" => "grammar",
        "style" => "style",
        "form" => "form",
        _ => {
            return Err(ZiziphusError::msg(format!("unknown group: {}", grp)));
        }
    };

    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} add|list|del|ex|err|rule|desc|cat|lvl|note|show ...",
            grp
        )));
    }

    let sub = &args[0];
    let rest = &args[1..];

    match sub.as_str() {
        "add" => cmd_gadd(con, kind, rest),
        "list" => cmd_glist(con, kind, rest),
        "del" => cmd_gdel(con, kind, rest),
        "err" => cmd_gerr(con, kind, rest),
        "ex" => cmd_ex(con, kind, rest),
        "show" => cmd_gshow(con, kind, rest),
        "rule" | "desc" | "cat" | "lvl" | "note" => cmd_gset(con, kind, sub, rest),
        _ => Err(ZiziphusError::msg(format!(
            "unknown: ziziphus {} {}",
            grp, sub
        ))),
    }
}

/// Parse supported flags while preserving positional arguments.
///
/// Unknown flags and missing values are errors instead of silently
/// producing an empty argument list.
fn parse_flags(
    args: &[String],
    spec: &[(&str, &str)],
) -> Result<(Vec<String>, HashMap<String, String>), ZiziphusError> {
    let mut positional = Vec::new();
    let mut options = HashMap::new();

    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if let Some((_, key)) = spec.iter().find(|(flag, _)| *flag == arg) {
            if index + 1 >= args.len() {
                return Err(ZiziphusError::msg(format!("missing value for {}", arg)));
            }

            let value = &args[index + 1];

            // A following option is almost certainly a missing value.
            if value.starts_with('-') {
                return Err(ZiziphusError::msg(format!("missing value for {}", arg)));
            }

            options.insert((*key).to_string(), value.clone());
            index += 2;
            continue;
        }

        if arg.starts_with('-') {
            return Err(ZiziphusError::msg(format!("unknown option: {}", arg)));
        }

        positional.push(arg.clone());
        index += 1;
    }

    Ok((positional, options))
}

/// Validate a 1-based item index.
fn check_n(items: &[Value], number: usize, what: &str) -> Result<(), ZiziphusError> {
    if number == 0 || number > items.len() {
        return Err(ZiziphusError::msg(format!(
            "no {} #{} (have {})",
            what,
            number,
            items.len()
        )));
    }

    Ok(())
}
