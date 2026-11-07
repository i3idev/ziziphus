//! Example commands: `ex` (shared by word and grammar kinds).
//!
//! Examples are bilingual sentence pairs with an English side (`en`)
//! and a Persian side (`fa`). Language is auto-detected from Unicode
//! script.
//!
//! Operations:
//!
//! - List: `gxx ex <word>`
//! - Add: `gxx ex <word> ["en text"] ["fa text"]`
//! - Edit one side: `gxx ex <word> N "text"`
//! - Delete: `gxx ex <word> -N` (or `--delete --number N` via clap)

use rusqlite::Connection;
use serde_json::Value;

use crate::db::{load_rec_fallback, save_rec};
use crate::error::ZiziphusError;
use crate::model::lang_of;
use crate::render::ex_lines;

/// `gxx ex <word> ["text"] | N "text" | -N`
///
/// Dispatches example management for a word or grammar/style/form
/// entry. `kind` is `"word"`, `"grammar"`, `"style"`, or `"form"`.
///
/// # Errors
///
/// - `usage: gxx ex <name> ["text"] | N "text" | -N` — no name given
/// - `no <kind> '<name>'. add it first: gxx <kind> add <name>` — entry not found
/// - `no example #N (have M)` — example number out of range
/// - `one example per command: quote each sentence` — more than two texts
/// - `two texts in the same language; give one en and one fa` — same-language pair
pub fn cmd_ex(con: &Connection, kind: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: gxx ex <name> [\"text\"] | N \"text\" | -N",
        ));
    }
    let mut rec = load_rec_fallback(con, kind, &args[0])?;
    let rest = &args[1..];

    // Ensure examples array exists
    if rec.get("examples").is_none() {
        rec["examples"] = Value::Array(vec![]);
    }

    if rest.is_empty() {
        // List examples
        let exs = rec["examples"].as_array().unwrap();
        println!("{}", ex_lines(exs));
        return Ok(());
    }

    // Check for -N (remove example N)
    if let Some(first) = rest.first()
        && first.starts_with('-')
    {
        if first.len() == 1 || !first[1..].chars().all(|c| c.is_ascii_digit()) {
            return Err(ZiziphusError::msg("invalid example number"));
        }

        let n: usize = first[1..]
            .parse()
            .map_err(|_| ZiziphusError::msg("invalid example number"))?;
        {
            let exs = rec["examples"].as_array_mut().unwrap();
            check_n(exs, n, "example")?;
            exs.remove(n - 1);
        }
        save_rec(con, kind, &mut rec)?;
        if !crate::model::STATE.lock().unwrap().quiet {
            let exs = rec["examples"].as_array().unwrap();
            println!("{}", ex_lines(exs));
        }
        return Ok(());
    }

    // Check for N "text" (set one side of example N)
    if let Some(first) = rest.first()
        && first.chars().all(|c| c.is_ascii_digit())
    {
        let n: usize = first.parse().unwrap();
        {
            let exs = rec["examples"].as_array_mut().unwrap();
            check_n(exs, n, "example")?;
            if rest.len() != 2 {
                return Err(ZiziphusError::msg("usage: gxx ex <name> N \"text\""));
            }
            let lang = lang_of(&rest[1]);
            if exs[n - 1].get(lang).is_some() {
                // Allow overwrite
            }
            exs[n - 1][lang] = Value::String(rest[1].clone());
        }
        save_rec(con, kind, &mut rec)?;
        if !crate::model::STATE.lock().unwrap().quiet {
            let exs = rec["examples"].as_array().unwrap();
            println!("{}", ex_lines(exs));
        }
        return Ok(());
    }

    // Add new example: one or two texts, different languages
    if rest.len() > 2 {
        return Err(ZiziphusError::msg(
            "one example per command: quote each sentence",
        ));
    }
    {
        let exs = rec["examples"].as_array_mut().unwrap();
        let mut ex = serde_json::Map::new();
        for t in rest {
            let lg = lang_of(t);
            if ex.contains_key(lg) {
                return Err(ZiziphusError::msg(
                    "two texts in the same language; give one en and one fa",
                ));
            }
            ex.insert(lg.to_string(), Value::String(t.clone()));
        }
        exs.push(Value::Object(ex));
    }
    save_rec(con, kind, &mut rec)?;
    if !crate::model::STATE.lock().unwrap().quiet {
        let exs = rec["examples"].as_array().unwrap();
        println!("{}", ex_lines(exs));
    }
    Ok(())
}

fn check_n(items: &mut [Value], n: usize, what: &str) -> Result<(), ZiziphusError> {
    if n == 0 || n > items.len() {
        return Err(ZiziphusError::msg(format!(
            "no {} #{} (have {})",
            what,
            n,
            items.len()
        )));
    }
    Ok(())
}
