//! SQLite storage layer for `ziziphus`.
//!
//! This module owns filesystem and database access:
//!
//! - Vault / home directory resolution
//! - Database file paths and discovery
//! - Active database tracking and connections
//! - Record loading and saving
//! - Full-text search
//!
//! Data directory precedence:
//! `--vault` / `-v` > `ZIZIPHUS_VAULT` > `ZIZIPHUS_HOME` > `~/.ziziphus`.

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::ZiziphusError;
use crate::model::{entry_id, key_of, lang_of, normalise, now, searchable, title_of};

// -----------------------------------------------------------------------------
// Vault configuration
// -----------------------------------------------------------------------------

/// Process-wide vault override.
///
/// An explicit CLI value takes precedence over environment variables.
static VAULT: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

/// Access the vault mutex without panicking if it was poisoned.
fn vault_state() -> std::sync::MutexGuard<'static, Option<PathBuf>> {
    VAULT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Set the vault path from the command line.
///
/// Relative paths are resolved against the current working directory
/// at the time this function is called.
pub fn set_vault(vault: Option<&str>) {
    let resolved = vault.map(|value| {
        let path = PathBuf::from(value);

        let absolute = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        };

        absolute.canonicalize().unwrap_or(absolute)
    });

    *vault_state() = resolved;
}

/// Get the explicitly selected vault, excluding environment fallbacks.
pub fn get_vault() -> Option<PathBuf> {
    vault_state().clone()
}

/// Resolve the active vault directory.
///
/// An explicit CLI selection takes precedence over `ZIZIPHUS_VAULT`.
fn vault_dir() -> Option<PathBuf> {
    if let Some(path) = vault_state().clone() {
        return Some(path);
    }

    std::env::var_os("ZIZIPHUS_VAULT")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

// -----------------------------------------------------------------------------
// Paths and configuration
// -----------------------------------------------------------------------------

/// Resolve the ziziphus home directory.
///
/// Precedence:
/// 1. Explicit CLI vault
/// 2. `ZIZIPHUS_VAULT`
/// 3. `ZIZIPHUS_HOME`
/// 4. `~/.ziziphus`
///
/// When a vault is active, the application directory is `<vault>/.ziziphus`.
pub fn home_dir() -> PathBuf {
    if let Some(vault) = vault_dir() {
        return vault.join(".ziziphus");
    }

    if let Some(path) = std::env::var_os("ZIZIPHUS_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ziziphus")
}

/// Directory containing database files: `<home>/db`.
pub fn db_dir() -> PathBuf {
    home_dir().join("db")
}

/// Path to the configuration file: `<home>/config`.
///
/// Stores the active database name.
pub fn config_path() -> PathBuf {
    home_dir().join("config")
}

// -----------------------------------------------------------------------------
// Database names
// -----------------------------------------------------------------------------

/// Database name validation pattern.
///
/// Only letters, digits, underscores, and hyphens are allowed.
/// The regex `\w` is Unicode-aware.
pub const DB_NAME_RE: &str = r"^[\w-]+$";

static DB_NAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(DB_NAME_RE).expect("DB_NAME_RE must be a valid regex"));

/// Validate a database name.
///
/// This prevents path traversal and invalid database filenames.
pub fn validate_db_name(name: &str) -> Result<(), ZiziphusError> {
    if name.is_empty() || !DB_NAME_REGEX.is_match(name) {
        return Err(ZiziphusError::msg(format!(
            "invalid database name '{}'. use letters, numbers, '_' or '-'",
            name
        )));
    }

    Ok(())
}

/// Resolve the active database name.
///
/// If `override_name` is provided, it takes precedence over the config file.
///
/// # Errors
///
/// - No active database is configured.
/// - The selected database name is invalid.
pub fn active_name(override_name: Option<&str>) -> Result<String, ZiziphusError> {
    let name = if let Some(name) = override_name {
        name.trim().to_string()
    } else {
        let config = config_path();

        if !config.exists() {
            return Err(ZiziphusError::msg(
                "no active db. run: ziziphus init, then: ziziphus db new <name>",
            ));
        }

        std::fs::read_to_string(config)?.trim().to_string()
    };

    if name.is_empty() {
        return Err(ZiziphusError::msg(
            "no active db. run: ziziphus init, then: ziziphus db new <name>",
        ));
    }

    validate_db_name(&name)?;

    Ok(name)
}

/// Path to a database file by name.
///
/// Callers that accept user input should validate the name first.
pub fn db_path(name: &str) -> PathBuf {
    db_dir().join(format!("{}.db", name))
}

/// List database names in the current data directory.
///
/// Results are sorted alphabetically.
/// A missing database directory produces an empty list.
pub fn db_names() -> Result<Vec<String>, ZiziphusError> {
    let directory = db_dir();

    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();

    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("db") {
            continue;
        }

        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            names.push(stem.to_string());
        }
    }

    names.sort();
    Ok(names)
}

// -----------------------------------------------------------------------------
// Schema and connections
// -----------------------------------------------------------------------------

/// Schema for the `entries` table.
///
/// The JSON `data` column stores the complete record.
/// Other columns provide indexed and searchable metadata.
pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS entries (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL,
    lang       TEXT NOT NULL,
    level      TEXT,
    title      TEXT NOT NULL,
    text       TEXT,
    data       TEXT NOT NULL,
    created_at TEXT,
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_kind_lang
ON entries(kind, lang);
";

/// Open a connection to the active or explicitly selected database.
///
/// The database must already exist. This function does not create
/// a new database file.
///
/// # Errors
///
/// - No active database is configured.
/// - The database name is invalid.
/// - The database file does not exist.
/// - SQLite cannot open the database or apply the schema.
pub fn connect(override_name: Option<&str>) -> Result<Connection, ZiziphusError> {
    let name = active_name(override_name)?;
    let path = db_path(&name);

    if !path.is_file() {
        return Err(ZiziphusError::msg(format!(
            "no database '{}'. see: ziziphus db list",
            name
        )));
    }

    let connection = Connection::open(path)?;
    connection.execute_batch(SCHEMA)?;

    Ok(connection)
}

// -----------------------------------------------------------------------------
// Record loading
// -----------------------------------------------------------------------------

/// Load a record by exact database ID.
///
/// Database ID format: `kind:key_of(name)`.
///
/// Returns `Ok(None)` if no matching row exists.
pub fn load_rec(con: &Connection, kind: &str, name: &str) -> Result<Option<Value>, ZiziphusError> {
    let id = entry_id(kind, name);

    let mut statement = con.prepare("SELECT data FROM entries WHERE id = ?1")?;

    let mut rows = statement.query([&id])?;

    match rows.next()? {
        Some(row) => {
            let json: String = row.get(0)?;
            let record: Value = serde_json::from_str(&json)?;
            Ok(Some(record))
        }
        None => Ok(None),
    }
}

/// Load a record using exact ID lookup followed by normalised title lookup.
///
/// The fallback handles Persian/Arabic character variants and ZWNJ.
///
/// It does not modify the stored record or its original title.
pub fn load_rec_fallback(con: &Connection, kind: &str, name: &str) -> Result<Value, ZiziphusError> {
    if let Some(record) = load_rec(con, kind, name)? {
        return Ok(record);
    }

    let normalised_name = normalise(&key_of(name));

    let mut statement = con.prepare("SELECT data FROM entries WHERE kind = ?1")?;

    let mut rows = statement.query([kind])?;

    while let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let record: Value = serde_json::from_str(&json)?;

        let title = title_of(&record);

        if normalise(&key_of(&title)) == normalised_name {
            return Ok(record);
        }
    }

    Err(ZiziphusError::msg(format!(
        "no {} '{}'. add it first: ziziphus {} add {}",
        label(kind),
        name,
        label(kind),
        name
    )))
}

fn label(kind: &str) -> &str {
    crate::model::label(kind)
}

// -----------------------------------------------------------------------------
// Record saving
// -----------------------------------------------------------------------------

/// Save a record using an SQLite upsert.
///
/// The database ID is generated from the entry kind and title.
/// The JSON record's own `id` field is preserved.
///
/// - `updated_at` is always refreshed.
/// - `created_at` is assigned if missing or invalid.
/// - `lang` is preserved when it is a non-empty string.
/// - The searchable text column is rebuilt from the JSON record.
///
/// Existing `created_at` values are not overwritten during updates.
pub fn save_rec(con: &Connection, kind: &str, rec: &mut Value) -> Result<(), ZiziphusError> {
    if !rec.is_object() {
        return Err(ZiziphusError::msg("entry must be a JSON object"));
    }

    let title = title_of(rec);

    if title.trim().is_empty() {
        return Err(ZiziphusError::msg("entry title must not be empty"));
    }

    let id = entry_id(kind, &title);
    let timestamp = now();

    let created_at = rec
        .get("created_at")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| timestamp.clone());

    let lang = rec
        .get("lang")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| lang_of(&title).to_string());

    rec["created_at"] = Value::String(created_at.clone());
    rec["updated_at"] = Value::String(timestamp.clone());
    rec["lang"] = Value::String(lang.clone());

    let level = rec.get("level").and_then(Value::as_str).map(str::to_string);

    let text = searchable(rec);
    let data = serde_json::to_string(rec)?;

    con.execute(
        "
        INSERT INTO entries (
            id,
            kind,
            lang,
            level,
            title,
            text,
            data,
            created_at,
            updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            lang       = excluded.lang,
            level      = excluded.level,
            title      = excluded.title,
            text       = excluded.text,
            data       = excluded.data,
            updated_at = excluded.updated_at
        ",
        (
            &id,
            kind,
            &lang,
            level.as_deref(),
            &title,
            &text,
            &data,
            &created_at,
            &timestamp,
        ),
    )?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Search helpers
// -----------------------------------------------------------------------------

/// Build a regex for substring-style wildcard matching.
///
/// `*` matches any sequence of characters.
/// All other characters are treated literally.
fn glob_regex(pattern: &str) -> Regex {
    let escaped = regex::escape(pattern).replace(r"\*", ".*");

    Regex::new(&format!("(?s)^.*{}.*$", escaped))
        .expect("escaped wildcard pattern must produce a valid regex")
}

/// Compare a title against a query for exact-match ranking.
fn is_exact_title(title: &str, query: &str) -> bool {
    normalise(&key_of(title)) == normalise(&key_of(query))
}

/// Search matching records.
///
/// Returns `(kind, title, record)` tuples sorted by:
/// 1. Exact title matches first.
/// 2. Title alphabetically, case-insensitively.
/// 3. Original title as a stable tie-breaker.
fn search_matches(
    con: &Connection,
    query: &str,
) -> Result<Vec<(String, String, Value)>, ZiziphusError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_lowercase();
    let normalised_query = normalise(&query_lower);
    let has_wildcard = query.contains('*');

    let wildcard = if has_wildcard {
        Some(glob_regex(&normalised_query))
    } else {
        None
    };

    let mut statement = con.prepare("SELECT kind, data FROM entries")?;

    let mut rows = statement.query([])?;
    let mut matches: Vec<(String, String, Value)> = Vec::new();

    while let Some(row) = rows.next()? {
        let kind: String = row.get(0)?;
        let json: String = row.get(1)?;
        let record: Value = serde_json::from_str(&json)?;

        let title = title_of(&record);

        let normalised_title = normalise(&title.to_lowercase());

        let searchable_text = normalise(&searchable(&record).to_lowercase());

        let title_match = match &wildcard {
            Some(regex) => regex.is_match(&normalised_title),
            None => normalised_title.contains(&normalised_query),
        };

        let text_match = match &wildcard {
            Some(regex) => regex.is_match(&searchable_text),
            None => searchable_text.contains(&normalised_query),
        };

        if title_match || text_match {
            matches.push((kind, title, record));
        }
    }

    matches.sort_by(|a, b| {
        let a_exact = !is_exact_title(&a.1, query);
        let b_exact = !is_exact_title(&b.1, query);

        a_exact
            .cmp(&b_exact)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
            .then_with(|| a.1.cmp(&b.1))
    });

    Ok(matches)
}

// -----------------------------------------------------------------------------
// Search output
// -----------------------------------------------------------------------------

/// Search entries in a database.
///
/// Returns human-readable result lines.
pub fn search(con: &Connection, query: &str) -> Result<Vec<String>, ZiziphusError> {
    let matches = search_matches(con, query)?;

    let mut output = Vec::with_capacity(matches.len());

    for (kind, title, record) in matches {
        let line = match kind.as_str() {
            "word" => {
                let translations: Vec<String> = record
                    .get("translations")
                    .and_then(Value::as_object)
                    .map(|map| {
                        map.values()
                            .filter_map(Value::as_array)
                            .flatten()
                            .filter_map(Value::as_str)
                            .take(4)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();

                let pos = record
                    .get("pos")
                    .and_then(Value::as_str)
                    .map(|value| format!(" ({})", value))
                    .unwrap_or_default();

                format!("[word] {}{} : {}", title, pos, translations.join(", "))
            }

            _ => {
                let rule = record.get("rule").and_then(Value::as_str).unwrap_or("");

                format!("[{}] {} : {}", label(&kind), title, rule)
            }
        };

        output.push(line);
    }

    Ok(output)
}

/// Search entries and return complete JSON records.
///
/// Uses exactly the same matching and ordering logic as `search()`.
pub fn search_records(con: &Connection, query: &str) -> Result<Vec<Value>, ZiziphusError> {
    Ok(search_matches(con, query)?
        .into_iter()
        .map(|(_, _, record)| record)
        .collect())
}
