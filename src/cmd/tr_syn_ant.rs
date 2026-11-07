//! Translation, synonym, and antonym commands: `tr`, `syn`, `ant`.
//!
//! All three use the same `+add` / `-remove` operation syntax.
//! `tr` manages the `translations` object (keyed by language code);
//! `syn` and `ant` manage the `synonyms` and `antonyms` arrays.

use rusqlite::Connection;
use serde_json::Value;

use crate::db::{load_rec_fallback, save_rec};
use crate::error::ZiziphusError;
use crate::model::{add_unique, clean, remove_item};

/// `ziziphus tr <word> [-x LANG] +text -text ...`
///
/// Manage translations for a word entry.
///
/// The translation language defaults to `fa`.
/// Use `-x` / `--language` through the CLI to select another language.
///
/// `+text` adds, `-text` removes, and plain `text` adds.
/// Duplicate translations are skipped.
///
/// If a translation list becomes empty, its language key is removed.
pub fn cmd_tr(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus tr <word> [-lang] +text -text",
        ));
    }

    let mut rec = load_rec_fallback(con, "word", &args[0])?;
    let rest = &args[1..];

    // The dispatcher converts the CLI language option into -LANG.
    // Example: `-fa`, `-en`.
    let mut lang = "fa";

    let mut ops = Vec::new();

    for arg in rest {
        if arg.starts_with('-')
            && arg.len() == 3
            && arg[1..].chars().all(|c| c.is_ascii_lowercase())
        {
            if lang != "fa" {
                return Err(ZiziphusError::msg(
                    "translation language specified more than once",
                ));
            }

            lang = &arg[1..];
        } else {
            ops.push(arg.clone());
        }
    }

    let (add, remove) = parse_ops(&ops);

    if add.is_empty() && remove.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus tr <word> [-lang] +text -text",
        ));
    }

    if rec.get("translations").is_none() {
        rec["translations"] = Value::Object(serde_json::Map::new());
    }

    let translations = rec["translations"]
        .as_object_mut()
        .ok_or_else(|| ZiziphusError::msg("invalid translations data"))?;

    let mut values = translations
        .get(lang)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for item in add {
        add_unique(&mut values, item);
    }

    for item in remove {
        if !remove_item(&mut values, &item) {
            eprintln!("not found: {}", item);
        }
    }

    if values.is_empty() {
        translations.remove(lang);
    } else {
        translations.insert(lang.to_string(), Value::Array(values.clone()));
    }

    save_rec(con, "word", &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        let joined = values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "ok: {} = {}",
            lang,
            if joined.is_empty() { "-" } else { &joined }
        );
    }

    Ok(())
}

/// `ziziphus syn <word> +text -text`
/// `ziziphus ant <word> +text -text`
///
/// Manage synonyms or antonyms for a word entry.
///
/// Without operations, lists current values comma-separated.
/// `+text` adds an item; `-text` removes it.
pub fn cmd_syn_ant(con: &Connection, field: &str, args: &[String]) -> Result<(), ZiziphusError> {
    let key = match field {
        "syn" => "synonyms",
        "ant" => "antonyms",
        _ => {
            return Err(ZiziphusError::msg(format!(
                "unknown word relation field: {}",
                field
            )));
        }
    };

    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} <word> +text -text",
            field
        )));
    }

    let mut rec = load_rec_fallback(con, "word", &args[0])?;

    if args.len() == 1 {
        let values = rec
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        println!("{}", values);
        return Ok(());
    }

    let (add, remove) = parse_ops(&args[1..]);

    if add.is_empty() && remove.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} <word> +text -text",
            field
        )));
    }

    if rec.get(key).is_none() {
        rec[key] = Value::Array(vec![]);
    }

    let values = rec[key]
        .as_array_mut()
        .ok_or_else(|| ZiziphusError::msg(format!("invalid {} data", key)))?;

    for item in add {
        add_unique(values, item);
    }

    for item in remove {
        if !remove_item(values, &item) {
            eprintln!("not found: {}", item);
        }
    }

    let output = values
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    save_rec(con, "word", &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        println!(
            "{} {}",
            crate::style::ok("ok:"),
            if output.is_empty() { "-" } else { &output }
        );
    }

    Ok(())
}

/// Parse `+add`, `-remove`, and plain `add` tokens.
///
/// Empty values are ignored.
fn parse_ops(tokens: &[String]) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();

    for token in tokens {
        if let Some(value) = token.strip_prefix('-') {
            if !value.is_empty() && !value.starts_with('-') {
                let value = clean(value);
                if !value.is_empty() {
                    remove.push(value);
                }
            }
        } else if let Some(value) = token.strip_prefix('+') {
            if !value.is_empty() {
                let value = clean(value);
                if !value.is_empty() {
                    add.push(value);
                }
            }
        } else {
            let value = clean(token);
            if !value.is_empty() {
                add.push(value);
            }
        }
    }

    (add, remove)
}
