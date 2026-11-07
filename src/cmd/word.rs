//! Word commands: `add`, `show`, `find`, `list`, `del`, `pos`, `lvl`,
//! `pron`, `def`, `note`, `ex`, `sense`, `coll`, `family`.
//!
//! A word entry stores: word/title, translations, part of speech,
//! proficiency level, pronunciation, definition, notes, examples,
//! synonyms, antonyms, senses, collocations, and word-family members.

use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;

use crate::cmd::ex::cmd_ex;
use crate::db::{load_rec, load_rec_fallback, save_rec, search};
use crate::error::ZiziphusError;
use crate::model::{
    add_unique, clean, collocations_mut, entry_id, family_mut, lang_of, new_family_member,
    new_sense, new_word, senses_mut, title_of,
};
use crate::render::{render_entry, render_word_list};

/// Add a new word entry, or update an existing one.
///
/// Translations default to Persian (`fa`). Use `--lang` to select
/// the translation language. Duplicate translations are skipped.
pub fn cmd_add(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    let (pos, opts) = parse_flags(
        args,
        &[
            ("-p", "pos"),
            ("--pos", "pos"),
            ("-l", "level"),
            ("--lvl", "level"),
            ("--lang", "lang"),
        ],
    )?;

    if pos.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus add <word> [translations...] [--lang LANG] [-p pos] [-l level]",
        ));
    }

    let name = clean(&pos[0]);

    if name.is_empty() {
        return Err(ZiziphusError::msg("word cannot be empty"));
    }

    let existing = load_rec(con, "word", &name)?;
    let created = existing.is_none();

    let mut rec = match existing {
        Some(rec) => rec,
        None => new_word(&name),
    };

    // Keep the word's language based on its script.
    rec["lang"] = Value::String(lang_of(&name).to_string());

    let target = opts.get("lang").map(String::as_str).unwrap_or("fa");

    for translation in &pos[1..] {
        let translation = clean(translation);

        if translation.is_empty() {
            continue;
        }

        let translations = rec
            .as_object_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid word record"))?
            .entry("translations")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));

        let translations = translations
            .as_object_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid translations data"))?;

        let list = translations
            .entry(target.to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid translation list"))?;

        add_unique(list, translation);
    }

    for (key, value) in opts {
        if key == "lang" {
            continue;
        }

        if value.trim().is_empty() {
            rec.as_object_mut()
                .ok_or_else(|| ZiziphusError::msg("invalid word record"))?
                .remove(&key);
        } else {
            rec[key] = Value::String(value);
        }
    }

    save_rec(con, "word", &mut rec)?;

    if !crate::model::state().quiet {
        let tag = if created { "added" } else { "updated" };
        println!("{}: {}", crate::style::ok(tag), name);
    }

    Ok(())
}

/// Display an entry's content.
///
/// Searches word, grammar, style, and form entries.
pub fn cmd_show(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus show <word> [tr|syn|ant|ex]",
        ));
    }

    let name = &args[0];
    let sections = args[1..].to_vec();
    let mut shown = Vec::new();

    for kind in ["word", "grammar", "style", "form"] {
        match load_rec_fallback(con, kind, name) {
            Ok(rec) => shown.push(render_entry(kind, &rec, &sections)),
            Err(_) => continue,
        }
    }

    if shown.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "not found: {} (try: ziziphus find {})",
            name, name
        )));
    }

    println!("{}", shown.join("\n\n"));
    Ok(())
}

/// Search entries by title or content.
///
/// `--all` searches all databases.
pub fn cmd_find(
    con: &Connection,
    args: &[String],
    _db_override: Option<&str>,
) -> Result<(), ZiziphusError> {
    let filtered: Vec<String> = args
        .iter()
        .filter(|arg| arg.as_str() != "--all")
        .cloned()
        .collect();

    let all = args.iter().any(|arg| arg == "--all");
    let (pos, _) = parse_flags(&filtered, &[])?;

    if pos.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus find <text> [--all]"));
    }

    let query = pos.join(" ");

    if all {
        let names = crate::db::db_names()?;
        let mut total = 0;

        for name in names {
            let path = crate::db::db_path(&name);
            let db = Connection::open(&path)?;
            let lines = search(&db, &query)?;

            for line in &lines {
                println!("{}: {}", name, line);
            }

            total += lines.len();
        }

        println!("-- {} found", total);
        return Ok(());
    }

    let lines = search(con, &query)?;

    if lines.is_empty() {
        println!("nothing found");
    } else {
        println!("{}", lines.join("\n"));
        println!("-- {} found", lines.len());
    }

    Ok(())
}

/// List word entries with optional filters.
pub fn cmd_list(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    let (_, opts) = parse_flags(
        args,
        &[("--pos", "pos"), ("--lvl", "lvl"), ("--lang", "lang")],
    )?;

    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind='word'")?;
    let mut rows = stmt.query([])?;
    let mut count = 0;

    while let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&json_str)?;

        if let Some(level) = opts.get("lvl") {
            let actual = rec.get("level").and_then(Value::as_str).unwrap_or("");

            if !actual.eq_ignore_ascii_case(level) {
                continue;
            }
        }

        if let Some(lang) = opts.get("lang") {
            let title = title_of(&rec);

            if lang_of(&title) != lang.as_str() {
                continue;
            }
        }

        if let Some(pos) = opts.get("pos") {
            let actual = rec.get("pos").and_then(Value::as_str).unwrap_or("");

            if !actual.eq_ignore_ascii_case(pos) {
                continue;
            }
        }

        println!("{}", render_word_list(&rec));
        count += 1;
    }

    println!("-- {} words", count);
    Ok(())
}

/// Delete a word entry.
///
/// Note: confirmation is handled by the CLI layer, if applicable.
pub fn cmd_del(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus del <name>"));
    }

    let rec = load_rec_fallback(con, "word", &args[0])?;
    let title = title_of(&rec);

    con.execute(
        "DELETE FROM entries WHERE id = ?1",
        [entry_id("word", &title)],
    )?;

    if !crate::model::state().quiet {
        println!("{}: {}", crate::style::warn("deleted"), clean(&args[0]));
    }

    Ok(())
}

/// Get or set a single field on a word entry.
pub fn cmd_set_word(con: &Connection, field: &str, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(format!(
            "usage: ziziphus {} <name> [value]",
            field
        )));
    }

    let mut rec = load_rec_fallback(con, "word", &args[0])?;

    let key = match field {
        "pos" => "pos",
        "lvl" => "level",
        "pron" => "pronunciation",
        "def" => "def",
        "note" => "notes",
        _ => return Err(ZiziphusError::msg("unknown field")),
    };

    if args.len() == 1 {
        let value = rec.get(key).and_then(Value::as_str).unwrap_or("-");
        println!("{}", value);
        return Ok(());
    }

    let value = args[1..].join(" ").trim().to_string();

    if value.is_empty() {
        rec.as_object_mut()
            .ok_or_else(|| ZiziphusError::msg("invalid word record"))?
            .remove(key);
    } else {
        rec[key] = Value::String(value.clone());
    }

    save_rec(con, "word", &mut rec)?;

    if !crate::model::state().quiet {
        println!("{} {} = {}", crate::style::ok("ok:"), field, value);
    }

    Ok(())
}

/// Manage examples for a word entry.
pub fn cmd_ex_word(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    cmd_ex(con, "word", args)
}

/// Manage numbered senses.
pub fn cmd_sense(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus sense <word> [add|del|show]",
        ));
    }

    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        let senses = senses_mut(&mut rec).map_err(ZiziphusError::msg)?;

        if senses.is_empty() {
            println!("no senses");
        } else {
            for (index, sense) in senses.iter().enumerate() {
                if let Some(definition) = sense.get("def").and_then(Value::as_str) {
                    println!(" {}> {}", index + 1, definition);
                }

                if let Some(note) = sense.get("note").and_then(Value::as_str)
                    && !note.is_empty()
                {
                    println!("    note: {}", note);
                }
            }
        }

        return Ok(());
    }

    match args[1].as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(ZiziphusError::msg(
                    "usage: ziziphus sense <word> add \"definition\" [--note \"...\"]",
                ));
            }

            let definition = args[2].clone();

            if definition.trim().is_empty() {
                return Err(ZiziphusError::msg("sense definition cannot be empty"));
            }

            let mut note = None;
            let mut index = 3;

            while index < args.len() {
                if args[index] == "--note" {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| ZiziphusError::msg("missing value for --note"))?;

                    if note.is_some() {
                        return Err(ZiziphusError::msg("duplicate option: --note"));
                    }

                    note = Some(value.clone());
                    index += 2;
                } else {
                    return Err(ZiziphusError::msg(format!(
                        "unknown sense option: {}",
                        args[index]
                    )));
                }
            }

            let mut sense = new_sense(&definition);

            if let Some(note) = note
                && !note.is_empty()
            {
                sense["note"] = Value::String(note);
            }

            let number = {
                let senses = senses_mut(&mut rec).map_err(ZiziphusError::msg)?;

                senses.push(sense);
                senses.len()
            };

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!("{} added sense #{}", crate::style::ok("ok:"), number);
            }
        }

        "del" => {
            if args.len() != 3 {
                return Err(ZiziphusError::msg("usage: ziziphus sense <word> del N"));
            }

            let number: usize = args[2]
                .parse()
                .map_err(|_| ZiziphusError::msg("invalid sense number"))?;

            {
                let senses = senses_mut(&mut rec).map_err(ZiziphusError::msg)?;

                if number == 0 || number > senses.len() {
                    return Err(ZiziphusError::msg(format!(
                        "no sense #{} (have {})",
                        number,
                        senses.len()
                    )));
                }

                senses.remove(number - 1);
            }

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!("{} deleted sense #{}", crate::style::ok("ok:"), number);
            }
        }

        "show" => {
            if args.len() != 3 {
                return Err(ZiziphusError::msg("usage: ziziphus sense <word> show N"));
            }

            let number: usize = args[2]
                .parse()
                .map_err(|_| ZiziphusError::msg("invalid sense number"))?;

            let senses = senses_mut(&mut rec).map_err(ZiziphusError::msg)?;

            if number == 0 || number > senses.len() {
                return Err(ZiziphusError::msg(format!(
                    "no sense #{} (have {})",
                    number,
                    senses.len()
                )));
            }

            let sense = &senses[number - 1];

            if let Some(definition) = sense.get("def").and_then(Value::as_str) {
                println!("def: {}", definition);
            }

            if let Some(note) = sense.get("note").and_then(Value::as_str)
                && !note.is_empty()
            {
                println!("note: {}", note);
            }

            if let Some(examples) = sense.get("examples").and_then(Value::as_array)
                && !examples.is_empty()
            {
                println!("{}", crate::render::ex_lines(examples));
            }
        }

        sub => {
            return Err(ZiziphusError::msg(format!(
                "unknown sense subcommand: {}",
                sub
            )));
        }
    }

    Ok(())
}

/// Manage collocations.
pub fn cmd_coll(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus coll <word> [add|del]"));
    }

    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        let colls = collocations_mut(&mut rec).map_err(ZiziphusError::msg)?;

        if colls.is_empty() {
            println!("no collocations");
        } else {
            for (index, collocation) in colls.iter().enumerate() {
                if let Some(text) = collocation.as_str() {
                    println!(" {}> {}", index + 1, text);
                }
            }
        }

        return Ok(());
    }

    match args[1].as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(ZiziphusError::msg(
                    "usage: ziziphus coll <word> add \"collocation\"",
                ));
            }

            let collocation = clean(&args[2..].join(" "));

            if collocation.is_empty() {
                return Err(ZiziphusError::msg("collocation cannot be empty"));
            }

            let number = {
                let colls = collocations_mut(&mut rec).map_err(ZiziphusError::msg)?;

                if colls.iter().any(|item| {
                    item.as_str()
                        .map(|text| clean(text) == collocation)
                        .unwrap_or(false)
                }) {
                    return Err(ZiziphusError::msg("collocation already exists"));
                }

                colls.push(Value::String(collocation));
                colls.len()
            };

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!("{} added collocation #{}", crate::style::ok("ok:"), number);
            }
        }

        "del" => {
            if args.len() != 3 {
                return Err(ZiziphusError::msg("usage: ziziphus coll <word> del N"));
            }

            let number: usize = args[2]
                .parse()
                .map_err(|_| ZiziphusError::msg("invalid collocation number"))?;

            {
                let colls = collocations_mut(&mut rec).map_err(ZiziphusError::msg)?;

                if number == 0 || number > colls.len() {
                    return Err(ZiziphusError::msg(format!(
                        "no collocation #{} (have {})",
                        number,
                        colls.len()
                    )));
                }

                colls.remove(number - 1);
            }

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!(
                    "{} deleted collocation #{}",
                    crate::style::ok("ok:"),
                    number
                );
            }
        }

        sub => {
            return Err(ZiziphusError::msg(format!(
                "unknown collocation subcommand: {}",
                sub
            )));
        }
    }

    Ok(())
}

/// Manage word-family members.
pub fn cmd_family(con: &Connection, args: &[String]) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg(
            "usage: ziziphus family <word> [add|del]",
        ));
    }

    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        let family = family_mut(&mut rec).map_err(ZiziphusError::msg)?;

        if family.is_empty() {
            println!("no family members");
        } else {
            for member in family.iter() {
                if let Some(word) = member.get("word").and_then(Value::as_str) {
                    let mut line = format!(" {}>", word);

                    if let Some(pos) = member.get("pos").and_then(Value::as_str)
                        && !pos.is_empty()
                    {
                        line.push_str(&format!(" ({})", pos));
                    }

                    println!("{}", line);
                }
            }
        }

        return Ok(());
    }

    match args[1].as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(ZiziphusError::msg(
                    "usage: ziziphus family <word> add <related-word> [-p pos]",
                ));
            }

            let related = clean(&args[2]);

            if related.is_empty() {
                return Err(ZiziphusError::msg("family member cannot be empty"));
            }

            let mut pos = None;
            let mut index = 3;

            while index < args.len() {
                match args[index].as_str() {
                    "-p" | "--pos" => {
                        let value = args
                            .get(index + 1)
                            .ok_or_else(|| ZiziphusError::msg("missing value for --pos"))?;

                        if value.starts_with('-') {
                            return Err(ZiziphusError::msg("missing value for --pos"));
                        }

                        if pos.is_some() {
                            return Err(ZiziphusError::msg("duplicate option: --pos"));
                        }

                        pos = Some(value.clone());
                        index += 2;
                    }

                    option => {
                        return Err(ZiziphusError::msg(format!(
                            "unknown family option: {}",
                            option
                        )));
                    }
                }
            }

            let number = {
                let family = family_mut(&mut rec).map_err(ZiziphusError::msg)?;

                if family.iter().any(|member| {
                    member
                        .get("word")
                        .and_then(Value::as_str)
                        .map(|word| clean(word) == related)
                        .unwrap_or(false)
                }) {
                    return Err(ZiziphusError::msg("family member already exists"));
                }

                family.push(new_family_member(&related, pos.as_deref()));
                family.len()
            };

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!(
                    "{} added family member #{}",
                    crate::style::ok("ok:"),
                    number
                );
            }
        }

        "del" => {
            if args.len() != 3 {
                return Err(ZiziphusError::msg(
                    "usage: ziziphus family <word> del <related-word>",
                ));
            }

            let related = clean(&args[2]);

            if related.is_empty() {
                return Err(ZiziphusError::msg("family member cannot be empty"));
            }

            {
                let family = family_mut(&mut rec).map_err(ZiziphusError::msg)?;

                let previous_len = family.len();

                family.retain(|member| {
                    member
                        .get("word")
                        .and_then(Value::as_str)
                        .map(|word| clean(word) != related)
                        .unwrap_or(true)
                });

                if family.len() == previous_len {
                    return Err(ZiziphusError::msg(format!(
                        "family member '{}' not found",
                        related
                    )));
                }
            }

            save_rec(con, "word", &mut rec)?;

            if !crate::model::state().quiet {
                println!(
                    "{} deleted family member '{}'",
                    crate::style::ok("ok:"),
                    related
                );
            }
        }

        sub => {
            return Err(ZiziphusError::msg(format!(
                "unknown family subcommand: {}",
                sub
            )));
        }
    }

    Ok(())
}

// ----- helpers -----

/// Parse positional arguments and options.
///
/// Every recognized option requires a following value.
/// Unknown long options and unknown short options are rejected.
fn parse_flags(
    args: &[String],
    spec: &[(&str, &str)],
) -> Result<(Vec<String>, HashMap<String, String>), ZiziphusError> {
    let mut positional = Vec::new();
    let mut opts = HashMap::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if let Some((_, key)) = spec.iter().find(|(flag, _)| *flag == arg) {
            let value = args
                .get(index + 1)
                .ok_or_else(|| ZiziphusError::msg(format!("missing value for {}", arg)))?;

            if value.starts_with('-') {
                return Err(ZiziphusError::msg(format!("missing value for {}", arg)));
            }

            if opts.contains_key(*key) {
                return Err(ZiziphusError::msg(format!("duplicate option: {}", arg)));
            }

            opts.insert((*key).to_string(), value.clone());
            index += 2;
            continue;
        }

        if arg.starts_with("--") || (arg.starts_with('-') && arg.len() > 1) {
            return Err(ZiziphusError::msg(format!("unknown option: {}", arg)));
        }

        positional.push(arg.clone());
        index += 1;
    }

    Ok((positional, opts))
}
