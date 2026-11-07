#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::invalid_html_tags)]
#![allow(rustdoc::bare_urls)]

//! # ziziphus
//!
//! vocabulary and grammar notebook CLI with a
//!
//! This page contains the complete practical tutorial for every `ziziphus` CLI
//! command. The Rust API documentation follows further down this page.
//!
//! ## Table of contents
//!
//! - [Overview](#overview)
//! - [Global options](#global-options)
//! - [Data directory and initialization](#data-directory-and-initialization)
//! - [Database management](#database-management)
//! - [Words: core workflow](#words-core-workflow)
//! - [Word fields](#word-fields)
//! - [Translations](#translations)
//! - [Synonyms and antonyms](#synonyms-and-antonyms)
//! - [Examples](#examples)
//! - [Senses (polysemy)](#senses-polysemy)
//! - [Collocations](#collocations)
//! - [Word families](#word-families)
//! - [Grammar, style, and form entries](#grammar-style-and-form-entries)
//! - [Searching and listing](#searching-and-listing)
//! - [Deleting entries](#deleting-entries)
//! - [Batch loading](#batch-loading)
//! - [Topics](#topics)
//! - [JSON output](#json-output)
//! - [Sequential tutorial: create a database and add words](#sequential-tutorial-create-a-database-and-add-words)
//! - [Sequential tutorial: enrich a word entry](#sequential-tutorial-enrich-a-word-entry)
//! - [Sequential tutorial: organize word relationships](#sequential-tutorial-organize-word-relationships)
//! - [Sequential tutorial: manage grammar, style, and form entries](#sequential-tutorial-manage-grammar-style-and-form-entries)
//! - [Restrictions, limitations, and common mistakes](#restrictions-limitations-and-common-mistakes)
//!
//! ## Overview
//!
//! `ziziphus` is a local, offline-first tool. Data lives in SQLite databases
//! under `<data-dir>/db/` (default `~/.ziziphus/db/`).
//!
//! The "data directory" is resolved in this order:
//!
//! 1. `--vault PATH` / `-v PATH` (CLI flag, highest priority)
//! 2. `ZIZIPHUS_VAULT` environment variable
//! 3. `ZIZIPHUS_HOME` environment variable
//! 4. `~/.ziziphus`
//!
//! When a vault is active, data is stored in `<vault>/.ziziphus/`. All
//! databases share one data directory; switch between them with
//! `ziziphus db use`.
//!
//! ## Modules
//!
//! - [`cli`] — command-line surface (clap)
//! - [`run`] — command dispatch and orchestration
//! - [`cmd`] — individual subcommand implementations
//! - [`db`] — SQLite storage layer
//! - [`model`] — in-memory entry model
//! - [`render`] — plain-text rendering of entries
//! - [`error`] — shared error type
//!
//! ## Quick start
//!
//! ```text
//! ziziphus init
//! ziziphus db new demo
//! ziziphus add book "کتاب" -p n -l A1
//! ```
//!
//! ## Global options
//!
//! These flags are defined on the root parser and are accepted before or
//! after any subcommand.
//!
//! - `--db NAME` — use database `NAME` for this command, overriding the
//!   active database. Resolves to `<data-dir>/db/<NAME>.db`.
//! - `--vault PATH`, `-v PATH` — use `<PATH>/.ziziphus/` as the data
//!   directory for this command. Overrides `ZIZIPHUS_VAULT`.
//! - `--json` — emit JSON instead of human-readable text. Supported by
//!   `show`, `find`, `list`, and direct word lookup. JSON is written to
//!   stdout.
//! - `--quiet`, `-q` — suppress non-error informational output. Errors
//!   are still printed. Success messages ("added", "updated", "ok:",
//! - `--no-color` — disable ANSI colors in output.
//!
//! Example:
//!
//! ```text
//! ziziphus --vault ~/project --db demo --json list
//! ```
//!
//! `ziziphus` with no subcommand prints the usage summary and exits
//! successfully. `ziziphus <word> [sections...]` is a legacy shortcut for
//! `ziziphus show <word> [sections...]`.
//!
//! ## Data directory and initialization
//!
//! ### `ziziphus init`
//!
//! ```text
//! ziziphus init
//! ```
//!
//! Creates the data directory, `<data-dir>/db/`, and an empty `config`
//! file if they do not exist. Prints `ready: <data-dir>`. If no
//! databases exist yet, prints `next : ziziphus db new <name>`.
//!
//! All subcommands require initialization (a `config` file must
//! exist); run `ziziphus init` first. Even `ziziphus db new` needs the data
//! directory created by `ziziphus init`.
//!
//! ### Local and global databases
//!
//! A database is always stored relative to the selected data directory.
//!
//! - Global database: use no `--vault` and no `ZIZIPHUS_VAULT`. The data
//!   directory is `ZIZIPHUS_HOME` if set, otherwise `~/.ziziphus`. Global
//!   databases are available across projects.
//! - Local (project) database: select it explicitly with `--vault PATH`
//!   or `ZIZIPHUS_VAULT`. Data is stored under `<PATH>/.ziziphus/`. There is no
//!   implicit use of the current working directory; running `ziziphus` from
//!   another directory does not select a project database unless you
//!   also select that vault.
//! - Active-database metadata is separate per data directory: each
//!   global or local `.ziziphus/config` stores only the active name for
//!   that scope. Local and global databases may share a name; their
//!   paths differ and operations are scoped to the selected data
//!   directory.
//! - Path precedence: `--vault` / `-v` > `ZIZIPHUS_VAULT` > `ZIZIPHUS_HOME` >
//!   `~/.ziziphus`.
//! - If the selected data directory has not been initialized, commands
//!   report `not initialised. run: ziziphus init` instead of falling back to
//!   another location.
//! - Deletion affects only the selected data directory. Deleting `foo`
//!   in a vault never deletes `foo` from global storage (or another
//!   vault), and changing the active scope does not change which file
//!   a previously selected name referred to in another scope.
//!
//! ## Database management
//!
//! Manage the SQLite database files themselves. Database commands
//! ignore `--db` and operate on the data directory directly.
//!
//! ### `ziziphus db new NAME`
//!
//! ```text
//! ziziphus db new demo
//! ```
//!
//! Create a new SQLite database named `NAME`. `NAME` must match
//! `^[\w-]+$` (letters, digits, underscore, hyphen). The file is
//! `<data-dir>/db/<NAME>.db` and is created with the ziziphus schema. If no
//! active database is set, the new database becomes active
//! automatically and `active : NAME` is printed.
//!
//! Errors: invalid name (`db name: letters, digits, - and _ only`);
//! `'<NAME>' already exists`.
//!
//! ### `ziziphus db list`
//!
//! ```text
//! ziziphus db list
//! ```
//!
//! List all databases with entry counts and last-updated timestamps.
//! The active database is marked with `*`:
//!
//! ```text
//!   NAME            ENTRIES  UPDATED
//! * demo            42       2026-...
//!   test             3        2026-...
//! ```
//!
//! If none exist, prints `no databases. run: ziziphus db new <name>`.
//!
//! ### `ziziphus db use NAME`
//!
//! ```text
//! ziziphus db use demo
//! ```
//!
//! Set the active database by writing `NAME` to `config`. `NAME` must
//! exist (`ziziphus db list` shows valid names); otherwise usage is
//! printed.
//!
//! ### `ziziphus db info [NAME]`
//!
//! ```text
//! ziziphus db info
//! ziziphus db info demo
//! ```
//!
//! Show database info: path, active status, and per-kind entry counts
//! (word, grammar, style, form). Without `NAME`, shows the active
//! database. Errors: `no database '<NAME>'`,
//! `no active db. run: ziziphus init, then: ziziphus db new <name>`.
//!
//! Verify creation with `ziziphus db list` or `ziziphus db info`.
//!
//! ### `ziziphus db del NAME [--yes]`
//!
//! ```text
//! ziziphus db del old
//! ziziphus db del old --yes
//! ```
//!
//! Delete a database file. Without `--yes`, prompts `type '<NAME>' to
//! delete it:` and requires typing the name exactly; otherwise prints
//! `cancelled`. With `--yes`, skips the prompt. If the deleted database
//! was active, the active setting is cleared (config becomes empty).
//!
//! Restrictions: deletion is permanent; there is no undo. Do not delete
//! the active database without creating/switching to another one. Do not
//! delete a name that does not exist (usage error).
//!
//! ## Words: core workflow
//!
//! Words are entries of kind `word`. They store a title, translations,
//! part of speech, level, pronunciation, definition, notes, examples,
//! synonyms, antonyms, senses, collocations, and family members.
//!
//! ### `ziziphus add WORD [TRANSLATION...] [-p POS] [-l LEVEL]`
//!
//! ```text
//! ziziphus add book "کتاب" -p n -l A1
//! ziziphus add run "run, execute" -p v -l B1
//! ```
//!
//! Add a new word entry, or update an existing one. Language is
//! auto-detected from the word's script (English vs. Persian). Each
//! `TRANSLATION` is auto-directed to the opposite language of the word.
//! Duplicates are skipped. `-p/--pos` sets part of speech; `-l/--level` sets proficiency
//! level. In the
//! interactive examples flags are shown as `-p` and `-l`.
//!
//! Empty option values remove the field. Prints `added: WORD` or
//! `updated: WORD` unless `--quiet`.
//!
//! Errors: `usage: ziziphus add <word> [translations...] [-p pos] [-l level]`
//! when no word is given.
//!
//! ### Direct lookup (legacy)
//!
//! ```text
//! ziziphus book tr syn
//! ```
//!
//! Equivalent to `ziziphus show book tr syn`.
//!
//! ## Word fields
//!
//! The commands `pos`, `lvl`, `pron`, `def`, `note` get or set a single
//! field. Without a value they print the current value, or `-` if unset;
//! with a value they set it (multi-word values are joined with spaces).
//! A non-quiet run prints `ok: <field> = <value>`.
//!
//! ```text
//! ziziphus pos book n
//! ziziphus pos book
//! ziziphus lvl book A2
//! ziziphus pron book /bʊk/
//! ziziphus def book "a written or printed work"
//! ziziphus note book "IR regular noun; countable"
//! ```
//!
//! Field mapping: `pos` → `pos`, `lvl` → `level`, `pron` →
//! `pronunciation`, `def` → `def`, `note` → `notes`.
//!
//! Empty value removes the field. Re-running `ziziphus add` with
//! `-p`/`-l` sets the same fields.
//!
//! Verify with `ziziphus show book` or `ziziphus <field> book`.
//!
//! ## Translations
//!
//! ### `ziziphus tr WORD [LANG] OP...`
//!
//! ```text
//! ziziphus tr book fa +کتاب +کتابچه
//! ziziphus tr book fa -کتابچه
//! ziziphus tr book +kitab            # auto-detects opposite language
//! ```
//!
//! Manage the `translations` object keyed by language code. `LANG` is a
//! two-letter code (e.g. `fa`, `en`); if omitted it is auto-detected
//! from the word's script (English word → `fa` target; Persian word →
//! `en` target). Operations: `+text` adds, `-text` removes, plain `text`
//! adds. Duplicates are skipped; removing a missing item prints
//! `not found: ...` to stderr. If a language list becomes empty, its
//! key is removed. Prints `ok: <lang> = <comma list>` or `-`.
//!
//! Errors: `usage: ziziphus tr <word> [lang] +x -y`;
//! `no word '<WORD>'. add it first: ziziphus add <WORD>`.
//! Restrictions: `LANG` must match `^[a-z]{2}$` to be treated as a
//! language code; otherwise it is treated as an operation.
//!
//! ## Synonyms and antonyms
//!
//! ### `ziziphus syn WORD OP...`
//! ### `ziziphus ant WORD OP...`
//!
//! ```text
//! ziziphus syn book +volume +tome
//! ziziphus syn book -tome
//! ziziphus ant book +cover
//! ```
//!
//! Manage the `synonyms` / `antonyms` arrays. `+text` adds, `-text`
//! removes, plain `text` adds, deduplicated. With no operations, prints
//! the current comma-separated list. Removing a missing item prints
//! `not found: ...`.
//!
//! Verify with `ziziphus show book syn` or `ziziphus syn book`.
//!
//! ## Examples
//!
//! ### `ziziphus ex WORD ...` (words)
//! ### `ziziphus gram|style|form ex NAME ...` (grammar/style/form)
//!
//! ```text
//! ziziphus ex book "I read a book." "کتابی خواندم."
//! ziziphus ex book 1 "I read a big book."
//! ziziphus ex book --delete --number 2
//! ```
//!
//! Examples are bilingual pairs with an English side (`en`) and a
//! Persian side (`fa`); language is auto-detected from Unicode script.
//!
//! Operations:
//!
//! - List all: `ziziphus ex WORD`
//! - Add a pair or single side: `ziziphus ex WORD "en text" "fa text"`
//!   (one or two texts; maximum two; both sides must be different
//!   languages).
//! - Edit one side of example N: `ziziphus ex WORD N "text"` or
//!   `ziziphus ex WORD --number N "text"`.
//! - Delete example N: `ziziphus ex WORD --delete --number N` or
//!   `ziziphus ex WORD -- -N`.
//!
//! For grammar/style/form entries the CLI form is
//! `ziziphus gram ex NAME ["text"]`, `ziziphus gram ex NAME N "text"`, or
//! `ziziphus gram ex NAME -- -N` (deletion via a negative number, because there
//! is no `--delete` flag on that group). Editing uses the `--number`
//! flag: `ziziphus gram ex NAME --number 2 "text"`.
//!
//! Restrictions and errors: `one example per command: quote each
//! sentence` (>2 texts); `two texts in the same language; give one en and
//! one fa`; `no example #N (have M)`; `usage: ziziphus ex <name> N "text"`.
//! Deletion requires `--number` on the word form and cannot be combined
//! with example text.
//!
//! Verify with `ziziphus show book ex` or `ziziphus ex book`.
//!
//! ## Senses (polysemy)
//!
//! ### `ziziphus sense WORD [add|del|show]`
//!
//! ```text
//! ziziphus sense book add "written or printed work consisting of pages"
//! ziziphus sense book add "unit of something" --note "mathematical"
//! ziziphus sense book show 1
//! ziziphus sense book del 2
//! ziziphus sense book
//! ```
//!
//! Manage numbered senses (definitions with optional notes and examples).
//! `ziziphus sense WORD` with no action lists senses (`1> definition`,
//! optional `note:`). `add` takes a definition and optional `--note`.
//! `del N` deletes sense N. `show N` prints `def:`, optional `note:`,
//! and any examples for that sense.
//!
//! Errors: `usage: ...`; `no sense #N (have M)`; `invalid sense
//! number`; `unknown sense subcommand`.
//!
//! ## Collocations
//!
//! ### `ziziphus coll WORD [add|del]`
//!
//! ```text
//! ziziphus coll book add "book club"
//! ziziphus coll book
//! ziziphus coll book del 1
//! ```
//!
//! Manage collocations (common word pairings). `ziziphus coll WORD` lists
//! them numbered (`1> text`) or prints `no collocations`. `add TEXT`
//! adds (text may contain spaces). `del N` deletes number N.
//!
//! Errors: `usage: ziziphus coll <word> add "collocation"`,
//! `no collocation #N (have M)`, `invalid collocation number`,
//! `unknown collocation subcommand`.
//!
//! ## Word families
//!
//! ### `ziziphus family WORD [add|del]`
//!
//! ```text
//! ziziphus family book add textbook -p n
//! ziziphus family book add booklet
//! ziziphus family book del booklet
//! ziziphus family book
//! ```
//!
//! Manage word-family members (related words with optional POS). List
//! shows `word  (pos)`. Add: `ziziphus family WORD add RELATED [-p POS]`.
//! Delete by related word (`ziziphus family WORD del RELATED`). Errors if the
//! member is not found: `family member '<WORD>' not found`.
//!
//! ## Grammar, style, and form entries
//!
//! `gram`, `style`, and `form` share one subcommand set and differ only
//! in the stored entry kind (`grammar`, `style`, `form`). All use the
//! same handler.
//!
//! ```text
//! ziziphus gram add "present perfect" --cat tense --lvl B1 --rule "have + V3" --desc "..."
//! ziziphus style add "formal register" --cat style
//! ziziphus form add "plural nouns" --cat morphology
//! ```
//!
//! Subcommands (shown for `gram`; identical for `style`/`form`):
//!
//! - `ziziphus gram add NAME [--cat C] [--lvl L] [--rule R] [--desc D]`
//!   (flags also accept `-c`, `-l`, `-r`, `-d`)
//! - `ziziphus gram list [PREFIX]` — list by title; filter by key/prefix.
//! - `ziziphus gram show NAME [rule|desc|ex|err|note]` — show (optional
//!   sections).
//! - `ziziphus gram del NAME` — delete.
//! - `ziziphus gram rule NAME [VALUE]` — get/set `rule`.
//! - `ziziphus gram desc NAME [VALUE]` — get/set `desc`.
//! - `ziziphus gram cat NAME [VALUE]` — get/set `cat`.
//! - `ziziphus gram lvl NAME [VALUE]` — get/set `level`.
//! - `ziziphus gram note NAME [VALUE]` — get/set `notes`.
//! - `ziziphus gram ex NAME ...` — examples (same semantics as word
//!   examples).
//! - `ziziphus gram err NAME ...` — error examples (single text). List, add
//!   (`"text"`), delete (`-N`).
//!
//! Empty value removes a field. Adding an existing name updates it
//! (`updated`); new names print `added`.
//!
//! Sections accepted by `show`: `rule`, `desc`, `ex`, `err`, `note`
//! (grammar/style/form). Word `show` accepts `tr`, `syn`, `ant`, `ex`,
//! `sense`, `coll`, `family`, `note`.
//!
//! Verify with `ziziphus gram show "present perfect"` and `ziziphus gram list`.
//!
//! ## Searching and listing
//!
//! ### `ziziphus show WORD [SECTIONS...]`
//!
//! ```text
//! ziziphus show book
//! ziziphus show book tr syn ex
//! ziziphus show book --json
//! ```
//!
//! Display an entry (searches all four kinds: word, grammar, style,
//! form). Optional section filters narrow output. If the name matches
//! multiple kinds, all are rendered. Not found:
//! `not found: WORD  (try: ziziphus find WORD)`.
//!
//! ### `ziziphus find QUERY [--all]`
//!
//! ```text
//! ziziphus find book
//! ziziphus find "b*" --all
//! ```
//!
//! Case-insensitive substring search on normalised text; `*` wildcards.
//! Results sorted with exact matches first, then by title. `--all`
//! searches every database (printing `DB: ...`). Prints `--
//! N found` (or `nothing found`). With `--json`, prints a JSON array.
//!
//! ### `ziziphus list [--pos POS] [--lvl LEVEL] [--lang en|fa]`
//!
//! ```text
//! ziziphus list
//! ziziphus list --pos n --lvl A1 --lang en
//! ```
//!
//! List word entries with optional filters (case-insensitive for POS
//! and level). Prints `-- N words`.
//!
//! ## Deleting entries
//!
//! ### `ziziphus del WORD [--yes]`
//!
//! ```text
//! ziziphus del obsolete
//! ziziphus del obsolete --yes
//! ```
//!
//! Delete a word entry. Without `--yes`, asks for interactive
//! confirmation: type the word. Prints `cancelled` on mismatch.
//! On success prints `deleted: WORD`. Errors:
//! `usage: ziziphus del <name>`,
//! `no word '<WORD>'. add it first: ziziphus add <WORD>`.
//! Restriction: `del` targets word entries only; grammar/style/form use
//! their own `del` subcommand.
//!
//! ## Batch loading
//!
//! ### `ziziphus load FILE.ziziphus`
//!
//! ```text
//! ziziphus load demo.ziziphus
//! ```
//!
//! Execute commands from a text file (one command per line; `#` starts a
//! comment; empty lines skipped). Leading `ziziphus` token is optional.
//! Nested `load` is forbidden. Errors on individual lines are reported
//! (`line N: ...`) but processing continues. Quiet mode is forced during
//! processing. Prints `loaded N commands, M errors` at the end.
//!
//! Missing file: `no such file: PATH`. Parse error: `line N: parse
//! error`.
//!
//! ## Topics
//!
//! ### `ziziphus topics [TOPIC]`
//!
//! ```text
//! ziziphus topics
//! ```
//!
//! Print extended help. Topics: `language`,
//! `bookmarks`, `examples`. Without a topic, lists them. Unknown topic:
//! `unknown topic '...'`.
//!
//! ## JSON output
//!
//! With `--json`, `show` prints the entry (or array when several kinds
//! match), `find` prints a JSON array of records, and `list` prints a
//! JSON array of word records. JSON keys mirror stored fields.
//!
//! ## Sequential tutorial: create a database and add words
//!
//! 1. Initialize and create a database:
//!
//! ```text
//! ziziphus init
//! ziziphus db new vocab
//! ziziphus db use vocab
//! ziziphus db info
//! ```
//!
//! 2. Add entries:
//!
//! ```text
//! ziziphus add book کتاب -p n -l A1
//! ziziphus add run دویدن -p v -l A2
//! ```
//!
//! 3. Verify:
//!
//! ```text
//! ziziphus show book
//! ziziphus list
//! ziziphus find book
//! ```
//!
//! ## Sequential tutorial: enrich a word entry
//!
//! ```text
//! ziziphus pos book n
//! ziziphus lvl book A2
//! ziziphus pron book /bʊk/
//! ziziphus def book "a written or printed work"
//! ziziphus note book "countable"
//! ziziphus tr book fa +کتاب +کتابچه
//! ziziphus syn book +volume +tome
//! ziziphus ant book +cover
//! ziziphus sense book add "written work" --note "primary"
//! ziziphus coll book add "book club"
//! ziziphus family book add textbook -p n
//! ziziphus ex book "I read a book." "کتابی خواندم."
//! ```
//!
//! Verify each step with `ziziphus show book` and the relevant
//! `ziziphus show book <section>`.
//!
//! ## Sequential tutorial: organize word relationships
//!
//! ```text
//! ziziphus syn book +volume +tome
//! ziziphus syn book -tome
//! ziziphus ant book +cover
//! ziziphus family book add chapter -p n
//! ziziphus family book del chapter
//! ziziphus coll book add "book shelf"
//! ziziphus coll book del 1
//! ```
//!
//! List current values with `ziziphus syn book`, `ziziphus ant book`,
//! `ziziphus family book`, `ziziphus coll book`.
//!
//! ## Sequential tutorial: manage grammar, style, and form entries
//!
//! ```text
//! ziziphus gram add "present perfect" --cat tense --lvl B1 --rule "have + V3"
//! ziziphus gram desc "present perfect" "completed action with present result"
//! ziziphus gram ex "present perfect" "I have finished." "تمام کرده‌ام."
//! ziziphus gram err "present perfect" "I have went"
//! ziziphus gram show "present perfect"
//! ziziphus gram list
//! ```
//!
//! Same commands work with `style` and `form` instead of `gram`.
//!
//! ## Restrictions, limitations, and common mistakes
//!
//! - Run `ziziphus init` before any subcommand except `ziziphus db new`.
//! - `--db` overrides the active DB but database commands (`ziziphus db ...`)
//!   ignore it.
//! - `ziziphus add` requires at least one word; translations are pushed into
//!   the opposite language of the word; duplicates are skipped.
//! - `del` (word) needs confirmation unless `--yes`.
//! - `ziziphus ex` word form: `--delete` requires `--number` and cannot
//!   combine with text; maximum two texts and they must be different
//!   languages.
//! - Grammar/style/form entries use negative numbers (`-N`) to delete
//!   examples/errors from the CLI; there is no `--delete` flag on those.
//! - Field setters accept only one value token stream (joined with
//!   spaces); an empty value removes the field.
//! - `tr` LANG detection: a two-letter lowercase token is interpreted
//!   as the target language; quote or avoid such tokens as operations.
//! - `find` uses `*` glob syntax; other regex metacharacters are not
//!   special.
//! - `read --resume` on a finished file prints "already finished"; use
//!   `--clear-progress`.
//! - Do not create a separate manual or `docs/` directory; this page
//!   is the canonical command tutorial.
//!

mod cli;
mod cmd;
mod db;
mod error;
mod model;
mod render;
mod run;
mod style;
mod topics;

use crate::error::ZiziphusError;
use crate::run::run_cli;
use clap::Parser;
use cli::Cli;

fn main() {
    // Ensure stdout encoding on Windows
    #[cfg(windows)]
    {
        use std::io::{Write, stdout};
        let _ = stdout().flush();
    }

    let cli = Cli::parse();

    let result = run_cli(cli);

    match result {
        Ok(()) => {}
        Err(e) => {
            let msg = match e {
                ZiziphusError::Msg(msg) => msg,
                _ => e.to_string(),
            };
            eprintln!("{} {}", crate::style::err("error:"), msg);
            std::process::exit(1);
        }
    }
}
