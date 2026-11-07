//! Data model for `Ziziphus` entries.
//!
//! The JSON shape must remain semantically compatible with the Python
//! version. Known fields are accessed explicitly, while unknown fields
//! are preserved through `serde_json::Value`.
//!
//! Entry kinds:
//! - `word`
//! - `grammar`
//! - `style`
//! - `form`
//!
//! Database-level fields such as `kind`, `level`, `created_at`, and
//! `updated_at` may be managed by the database layer.

use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::{Mutex, MutexGuard};

// -----------------------------------------------------------------------------
// Global state
// -----------------------------------------------------------------------------

/// Global quiet state used by command handlers.
pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State { quiet: false }));

/// Process-wide state shared across command handlers.
pub struct State {
    /// Whether informational output should be suppressed.
    pub quiet: bool,
}

/// Access global state without panicking if the mutex was poisoned.
///
/// If another thread panicked while holding the mutex, the contained state
/// is recovered instead of making the entire application unusable.
pub fn state() -> MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Set the global quiet state.
pub fn set_quiet(quiet: bool) {
    state().quiet = quiet;
}

// -----------------------------------------------------------------------------
// Text and language helpers
// -----------------------------------------------------------------------------

/// Normalise Persian/Arabic-script text for search keys.
///
/// - Arabic yeh (`ي`) becomes Persian yeh (`ی`).
/// - Arabic kaf (`ك`) becomes Persian kaf (`ک`).
/// - Zero-width non-joiner (U+200C) is removed.
///
/// This function must only be applied to search text.
/// Original display text and stored JSON values remain unchanged.
///
/// Note: This is a compatibility-oriented normalisation, not a
/// general-purpose Unicode normalisation algorithm.
pub fn normalise(text: &str) -> String {
    let mut output = String::with_capacity(text.len());

    for ch in text.chars() {
        match ch {
            '\u{064A}' => output.push('\u{06CC}'),
            '\u{0643}' => output.push('\u{06A9}'),
            '\u{200C}' => {}
            other => output.push(other),
        }
    }

    output
}

/// Detect the primary language using the legacy two-language heuristic.
///
/// Returns:
/// - `"fa"` if at least one character belongs to U+0600..U+06FF.
/// - `"en"` otherwise.
///
/// This is NOT a general language detector.
/// Arabic, Persian, Urdu, and other languages using this block cannot
/// reliably be distinguished by this heuristic.
pub fn lang_of(text: &str) -> &'static str {
    if text
        .chars()
        .any(|ch| ('\u{0600}'..='\u{06FF}').contains(&ch))
    {
        "fa"
    } else {
        "en"
    }
}

/// Convert underscores to spaces, trim, and lowercase.
///
/// Used for logical keys. This does not perform Persian normalisation,
/// because IDs must remain compatible with existing Python databases.
pub fn key_of(name: &str) -> String {
    name.replace('_', " ").trim().to_lowercase()
}

/// Convert a name into a slug.
///
/// Consecutive whitespace is collapsed into a single hyphen.
pub fn slug(name: &str) -> String {
    key_of(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Return the short display label for an entry kind.
pub fn label(kind: &str) -> &str {
    match kind {
        "word" => "word",
        "grammar" => "gram",
        "style" => "style",
        "form" => "form",
        _ => "unknown",
    }
}

/// Return the title of a record.
///
/// For word records, `word` takes precedence.
/// For grammar/style/form records, `title` is used.
///
/// An empty or whitespace-only `word` does not prevent fallback to `title`.
pub fn title_of(rec: &Value) -> String {
    let word = rec
        .get("word")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());

    let title = rec
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());

    word.or(title).unwrap_or_default().to_string()
}

/// Convert underscores to spaces and remove surrounding whitespace.
pub fn clean(token: &str) -> String {
    token.replace('_', " ").trim().to_string()
}

/// Return the current UTC time in ISO 8601 format without microseconds.
pub fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// -----------------------------------------------------------------------------
// Array helpers
// -----------------------------------------------------------------------------

/// Add an item to an array if it is not already present.
///
/// Comparison is case-insensitive.
/// The original item is preserved for display.
/// Empty or whitespace-only items are ignored.
pub fn add_unique(arr: &mut Vec<Value>, item: String) {
    if item.trim().is_empty() {
        return;
    }

    let lower = item.trim().to_lowercase();

    let exists = arr.iter().any(|value| {
        value
            .as_str()
            .map(|existing| existing.trim().to_lowercase() == lower)
            .unwrap_or(false)
    });

    if !exists {
        arr.push(Value::String(item));
    }
}

/// Remove the first matching string from an array.
///
/// Comparison is case-insensitive.
/// Non-string values are ignored.
///
/// Returns `true` if an item was removed.
pub fn remove_item(arr: &mut Vec<Value>, item: &str) -> bool {
    if item.trim().is_empty() {
        return false;
    }

    let lower = item.trim().to_lowercase();

    let position = arr.iter().position(|value| {
        value
            .as_str()
            .map(|existing| existing.trim().to_lowercase() == lower)
            .unwrap_or(false)
    });

    match position {
        Some(index) => {
            arr.remove(index);
            true
        }
        None => false,
    }
}

// -----------------------------------------------------------------------------
// Search
// -----------------------------------------------------------------------------

/// Metadata fields excluded from full-text search.
///
/// These fields identify or describe the database record rather than
/// its linguistic content.
const SEARCH_IGNORED_KEYS: &[&str] = &["id", "kind", "lang", "created_at", "updated_at"];

/// Recursively collect searchable strings from a JSON value.
///
/// This allows nested fields such as:
/// - examples
/// - notes
/// - senses
/// - collocations
/// - family
///
/// Unknown Python-created fields are also searchable as long as they
/// contain strings.
fn collect_searchable(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                output.push(text.to_string());
            }
        }

        Value::Array(items) => {
            for item in items {
                collect_searchable(item, output);
            }
        }

        Value::Object(object) => {
            for (key, value) in object {
                if SEARCH_IGNORED_KEYS.contains(&key.as_str()) {
                    continue;
                }

                collect_searchable(value, output);
            }
        }

        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// Build the searchable text column from a JSON record.
///
/// The search index is computed from the JSON record instead of relying
/// on a previously stored database column. This allows older Python-created
/// records and records containing unknown fields to be searched.
///
/// Original JSON values are never modified.
pub fn searchable(rec: &Value) -> String {
    let mut parts = Vec::new();

    collect_searchable(rec, &mut parts);

    normalise(&parts.join(" "))
}

// -----------------------------------------------------------------------------
// Record constructors
// -----------------------------------------------------------------------------

/// Create a new word record.
///
/// The JSON `id` format is intentionally kept compatible with the
/// existing Python implementation.
pub fn new_word(name: &str) -> Value {
    serde_json::json!({
        "id": format!("{}-{}", lang_of(name), slug(name)),
        "lang": lang_of(name),
        "word": name,
        "translations": {},
        "synonyms": [],
        "antonyms": [],
        "examples": [],
        "notes": ""
    })
}

/// Create a new grammar/style/form record.
///
/// `kind` is intentionally not stored here because the entry kind
/// is managed by the database layer.
///
/// The JSON `id` format is kept compatible with the existing
/// Python implementation.
pub fn new_grammar(_kind: &str, name: &str) -> Value {
    serde_json::json!({
        "id": format!("{}-{}", lang_of(name), slug(name)),
        "lang": lang_of(name),
        "title": name,
        "examples": [],
        "errors": [],
        "notes": ""
    })
}

/// Build a database entry ID from a kind and name.
///
/// Format: `<kind>:<key_of(name)>`.
///
/// This is a database-level identifier and is distinct from the
/// `id` field generated inside JSON records.
///
/// IDs must not be Persian-normalised, to preserve compatibility
/// with existing Python-created databases.
pub fn entry_id(kind: &str, name: &str) -> String {
    format!("{}:{}", kind, key_of(name))
}

// -----------------------------------------------------------------------------
// Mutable record fields
// -----------------------------------------------------------------------------

/// Ensure a JSON object contains an array field and return it.
///
/// Creates the field if it does not exist.
/// Returns an error if the record is not an object or if the field
/// already exists with a non-array value.
///
/// Existing data is never silently overwritten.
fn array_field_mut<'a>(
    rec: &'a mut Value,
    field: &'static str,
) -> Result<&'a mut Vec<Value>, &'static str> {
    let object = rec.as_object_mut().ok_or("entry must be a JSON object")?;

    let value = object
        .entry(field)
        .or_insert_with(|| Value::Array(Vec::new()));

    value.as_array_mut().ok_or(match field {
        "senses" => "entry field 'senses' must be an array",
        "collocations" => "entry field 'collocations' must be an array",
        "family" => "entry field 'family' must be an array",
        _ => "entry field must be an array",
    })
}

/// Ensure the `senses` array exists and return a mutable reference.
pub fn senses_mut(rec: &mut Value) -> Result<&mut Vec<Value>, &'static str> {
    array_field_mut(rec, "senses")
}

/// Ensure the `collocations` array exists and return a mutable reference.
pub fn collocations_mut(rec: &mut Value) -> Result<&mut Vec<Value>, &'static str> {
    array_field_mut(rec, "collocations")
}

/// Ensure the `family` array exists and return a mutable reference.
pub fn family_mut(rec: &mut Value) -> Result<&mut Vec<Value>, &'static str> {
    array_field_mut(rec, "family")
}

// -----------------------------------------------------------------------------
// Nested record constructors
// -----------------------------------------------------------------------------

/// Create a new sense object.
pub fn new_sense(def: &str) -> Value {
    serde_json::json!({
        "def": def,
        "note": "",
        "examples": []
    })
}

/// Create a new family member object.
///
/// An empty or whitespace-only POS value is omitted.
pub fn new_family_member(word: &str, pos: Option<&str>) -> Value {
    let mut object = serde_json::json!({
        "word": word
    });

    if let Some(position) = pos
        && !position.trim().is_empty()
    {
        object["pos"] = Value::String(position.trim().to_string());
    }

    object
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalises_persian_search_text() {
        assert_eq!(normalise("ي ك می\u{200C}روم"), "ی ک میروم");
    }

    #[test]
    fn detects_legacy_language_heuristic() {
        assert_eq!(lang_of("book"), "en");
        assert_eq!(lang_of("کتاب"), "fa");
        assert_eq!(lang_of("book کتاب"), "fa");
    }

    #[test]
    fn creates_expected_keys() {
        assert_eq!(key_of("  Hello_World  "), "hello world");
        assert_eq!(slug("  Hello__World  "), "hello-world");
    }

    #[test]
    fn returns_correct_labels() {
        assert_eq!(label("word"), "word");
        assert_eq!(label("grammar"), "gram");
        assert_eq!(label("style"), "style");
        assert_eq!(label("form"), "form");
        assert_eq!(label("other"), "unknown");
    }

    #[test]
    fn title_falls_back_when_word_is_empty() {
        let record = json!({
            "word": "   ",
            "title": "Present Perfect"
        });

        assert_eq!(title_of(&record), "Present Perfect");
    }

    #[test]
    fn word_title_takes_precedence() {
        let record = json!({
            "word": "abandon",
            "title": "ignored"
        });

        assert_eq!(title_of(&record), "abandon");
    }

    #[test]
    fn adds_unique_items_case_insensitively() {
        let mut items = vec![json!("Book")];

        add_unique(&mut items, "book".to_string());
        add_unique(&mut items, "Novel".to_string());
        add_unique(&mut items, "   ".to_string());

        assert_eq!(items, vec![json!("Book"), json!("Novel")]);
    }

    #[test]
    fn removes_first_matching_item() {
        let mut items = vec![json!("Book"), json!("book"), json!("Novel")];

        assert!(remove_item(&mut items, "BOOK"));

        assert_eq!(items, vec![json!("book"), json!("Novel")]);
    }

    #[test]
    fn does_not_remove_non_string_values() {
        let mut items = vec![json!(42), json!("Book")];

        assert!(!remove_item(&mut items, "42"));
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn searchable_includes_nested_fields() {
        let record = json!({
            "id": "en-book",
            "lang": "en",
            "word": "book",
            "notes": "keep this note",
            "examples": [
                {
                    "en": "I read a book",
                    "fa": "من کتاب می‌خوانم"
                }
            ],
            "senses": [
                {
                    "def": "a written work",
                    "note": "a printed publication"
                }
            ],
            "collocations": ["read a book"],
            "family": [
                {
                    "word": "booklet",
                    "pos": "noun"
                }
            ],
            "custom_field": "unknown data"
        });

        let result = searchable(&record);

        for expected in [
            "book",
            "keep this note",
            "I read a book",
            "a written work",
            "a printed publication",
            "read a book",
            "booklet",
            "noun",
            "unknown data",
        ] {
            assert!(
                result.contains(expected),
                "missing searchable text: {expected}"
            );
        }

        assert!(!result.contains("en-book"));
    }

    #[test]
    fn searchable_handles_non_object_values() {
        assert_eq!(searchable(&json!("hello")), "hello");
        assert_eq!(searchable(&json!(null)), "");
        assert_eq!(searchable(&json!(42)), "");
    }

    #[test]
    fn creates_word_record() {
        let record = new_word("book");

        assert_eq!(record["id"], "en-book");
        assert_eq!(record["lang"], "en");
        assert_eq!(record["word"], "book");
        assert_eq!(record["translations"], json!({}));
        assert_eq!(record["synonyms"], json!([]));
    }

    #[test]
    fn creates_grammar_record() {
        let record = new_grammar("grammar", "Present Perfect");

        assert_eq!(record["id"], "en-present-perfect");
        assert_eq!(record["lang"], "en");
        assert_eq!(record["title"], "Present Perfect");
        assert_eq!(record["examples"], json!([]));
        assert_eq!(record["errors"], json!([]));
    }

    #[test]
    fn creates_database_entry_id() {
        assert_eq!(entry_id("word", "Hello_World"), "word:hello world");
    }

    #[test]
    fn creates_missing_array_fields() {
        let mut record = json!({});

        assert!(senses_mut(&mut record).is_ok());
        assert!(collocations_mut(&mut record).is_ok());
        assert!(family_mut(&mut record).is_ok());

        assert!(record["senses"].is_array());
        assert!(record["collocations"].is_array());
        assert!(record["family"].is_array());
    }

    #[test]
    fn rejects_invalid_array_field_type() {
        let mut record = json!({
            "senses": "invalid"
        });

        assert_eq!(
            senses_mut(&mut record).unwrap_err(),
            "entry field 'senses' must be an array"
        );

        assert_eq!(record["senses"], "invalid");
    }

    #[test]
    fn rejects_non_object_record() {
        let mut record = json!("not an object");

        assert_eq!(
            senses_mut(&mut record).unwrap_err(),
            "entry must be a JSON object"
        );
    }

    #[test]
    fn creates_sense_object() {
        let sense = new_sense("a written work");

        assert_eq!(sense["def"], "a written work");
        assert_eq!(sense["note"], "");
        assert_eq!(sense["examples"], json!([]));
    }

    #[test]
    fn creates_family_member() {
        let member = new_family_member("writer", Some("noun"));

        assert_eq!(member["word"], "writer");
        assert_eq!(member["pos"], "noun");
    }

    #[test]
    fn omits_empty_family_pos() {
        let member = new_family_member("writer", Some("  "));

        assert_eq!(
            member,
            json!({
                "word": "writer"
            })
        );
    }
}
