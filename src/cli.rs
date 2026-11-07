// src/cli.rs

//! CLI definitions using clap.
//!
//! This module defines the command-line interface of `ziziphus`.

use clap::{Args, Parser, Subcommand};

/// Root CLI argument parser.
///
/// Global options are available before or after subcommands.
#[derive(Parser, Debug)]
#[command(
    name = "ziziphus",
    version,
    about = "Word and grammar notebook",
    long_about = "ziziphus - A CLI tool for managing vocabulary and grammar notes.\nStores data in SQLite databases under ~/.ziziphus/db/ or ./.ziziphus/db."
)]
pub struct Cli {
    /// Database to use for this command.
    #[arg(long, global = true, value_name = "NAME")]
    pub db: Option<String>,

    /// Path to vault folder containing `.ziziphus/` data.
    ///
    /// Takes precedence over `ZIZIPHUS_VAULT`.
    #[arg(long, short = 'v', global = true, value_name = "PATH")]
    pub vault: Option<String>,

    /// Output results as JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-error informational output.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Force ANSI colors, even when stdout is not a terminal.
    #[arg(long, global = true, conflicts_with = "no_color")]
    pub color: bool,

    /// Disable ANSI colors.
    #[arg(long, global = true, conflicts_with = "color")]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize the data directory and config file.
    Init,

    /// Database management.
    #[command(subcommand)]
    Db(DbCommand),

    /// Add a new word entry or update an existing one.
    Add(AddArgs),

    /// Show entry details.
    Show(ShowArgs),

    /// Search entries by title or content.
    Find(FindArgs),

    /// List word entries.
    List(ListArgs),

    /// Delete a word entry.
    Del(DelArgs),

    /// Manage translations.
    Tr(TrArgs),

    /// Manage synonyms.
    Syn(SynAntArgs),

    /// Manage antonyms.
    Ant(SynAntArgs),

    /// Manage examples.
    ///
    /// Add: `ziziphus ex <word> "text"`
    /// Edit: `ziziphus ex <word> --number N "text"`
    /// Delete: `ziziphus ex <word> --delete --number N`
    Ex(ExArgs),

    /// Manage polysemy senses.
    Sense(SenseArgs),

    /// Manage collocations.
    Coll(CollArgs),

    /// Manage word-family members.
    Family(FamilyArgs),

    /// Print extended help on a topic.
    Topics(TopicsArgs),

    /// Set or get part of speech.
    Pos(FieldArgs),

    /// Set or get proficiency level.
    Lvl(FieldArgs),

    /// Set or get pronunciation.
    Pron(FieldArgs),

    /// Set or get definition.
    Def(FieldArgs),

    /// Set or get notes.
    Note(FieldArgs),

    /// Grammar entries.
    #[command(subcommand)]
    Gram(GramCommand),

    /// Style entries.
    #[command(subcommand)]
    Style(GramCommand),

    /// Form entries.
    #[command(subcommand)]
    Form(GramCommand),

    /// Load and execute commands from a `.ziziphus` file.
    Load(LoadArgs),

    /// Direct word lookup: `ziziphus <word> [sections...]`.
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// Database management commands.
#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Create a new SQLite database.
    New { name: String },

    /// List all databases.
    List,

    /// Set the active database.
    Use { name: String },

    /// Show database information.
    Info { name: Option<String> },

    /// Delete a database.
    Del {
        name: String,

        /// Skip interactive confirmation.
        #[arg(long)]
        yes: bool,
    },
}

/// Arguments for `ziziphus add <word> [translations...]`.
#[derive(Args, Debug)]
pub struct AddArgs {
    /// Word to add.
    pub word: String,

    /// Translation language code (default: fa).
    ///
    /// The word's own language is detected from its script.
    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,

    /// Translations.
    #[arg(value_name = "TRANSLATION")]
    pub translations: Vec<String>,

    /// Part of speech.
    #[arg(short, long, value_name = "POS")]
    pub pos: Option<String>,

    /// Proficiency level.
    #[arg(short, long, value_name = "LEVEL")]
    pub level: Option<String>,
}

/// Arguments for `ziziphus show <word> [sections...]`.
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Word to look up.
    pub word: String,

    /// Optional section filters.
    #[arg(value_name = "SECTION")]
    pub sections: Vec<String>,
}

/// Arguments for `ziziphus find <query> [--all]`.
#[derive(Args, Debug)]
pub struct FindArgs {
    /// Search query.
    pub query: String,

    /// Search across all databases.
    #[arg(long)]
    pub all: bool,
}

/// Arguments for `ziziphus list`.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter by part of speech.
    #[arg(long, value_name = "POS")]
    pub pos: Option<String>,

    /// Filter by proficiency level.
    #[arg(long, alias = "level", value_name = "LEVEL")]
    pub lvl: Option<String>,

    /// Filter by the language of the word itself (e.g. en, fa).
    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,
}

/// Arguments for `ziziphus del <word>`.
#[derive(Args, Debug)]
pub struct DelArgs {
    /// Word to delete.
    pub word: String,

    /// Skip interactive confirmation.
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for `ziziphus tr <word> [-x LANG] [+text] [-text]`.
#[derive(Args, Debug)]
pub struct TrArgs {
    /// Word to modify.
    pub word: String,

    /// Target language.
    #[arg(short = 'x', long = "language", value_name = "LANG")]
    pub lang: Option<String>,

    /// Operations: `+text` adds, `-text` removes, plain text adds.
    #[arg(value_name = "OP", allow_hyphen_values = true)]
    pub ops: Vec<String>,
}

/// Arguments for synonym and antonym operations.
#[derive(Args, Debug)]
pub struct SynAntArgs {
    /// Word to modify.
    pub word: String,

    /// Operations: `+text` adds, `-text` removes, plain text adds.
    #[arg(value_name = "OP", allow_hyphen_values = true)]
    pub ops: Vec<String>,
}

/// Arguments for `ziziphus ex <word>`.
#[derive(Args, Debug)]
pub struct ExArgs {
    /// Word to modify.
    pub word: String,

    /// Example texts.
    #[arg(value_name = "TEXT", allow_hyphen_values = true)]
    pub texts: Vec<String>,

    /// Example number (1-based).
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,

    /// Delete the specified example.
    #[arg(long, requires = "number", conflicts_with = "texts")]
    pub delete: bool,
}

/// Arguments for `ziziphus sense <word>`.
#[derive(Args, Debug)]
pub struct SenseArgs {
    /// Word to modify.
    pub word: String,

    /// Sense action.
    #[command(subcommand)]
    pub action: Option<SenseAction>,
}

/// Sense actions.
#[derive(Subcommand, Debug)]
pub enum SenseAction {
    /// Add a new sense.
    Add {
        definition: String,

        /// Optional note.
        #[arg(long)]
        note: Option<String>,
    },

    /// Delete sense number N.
    Del { number: usize },

    /// Show sense number N.
    Show { number: usize },
}

/// Arguments for `ziziphus coll <word>`.
#[derive(Args, Debug)]
pub struct CollArgs {
    /// Word to modify.
    pub word: String,

    /// Collocation action.
    #[command(subcommand)]
    pub action: Option<CollAction>,
}

/// Collocation actions.
#[derive(Subcommand, Debug)]
pub enum CollAction {
    /// Add a collocation.
    Add { text: String },

    /// Delete collocation number N.
    Del { number: usize },
}

/// Arguments for `ziziphus family <word>`.
#[derive(Args, Debug)]
pub struct FamilyArgs {
    /// Word to modify.
    pub word: String,

    /// Family action.
    #[command(subcommand)]
    pub action: Option<FamilyAction>,
}

/// Family actions.
#[derive(Subcommand, Debug)]
pub enum FamilyAction {
    /// Add a family member.
    Add {
        word: String,

        /// Part of speech.
        #[arg(short, long)]
        pos: Option<String>,
    },

    /// Delete a family member.
    Del { word: String },
}

/// Arguments for `ziziphus topics [topic]`.
#[derive(Args, Debug)]
pub struct TopicsArgs {
    /// Topic name.
    pub topic: Option<String>,
}

/// Arguments for field get/set commands.
#[derive(Args, Debug)]
pub struct FieldArgs {
    /// Word to operate on.
    pub word: String,

    /// Value to set.
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Grammar, style and form command group.
#[derive(Subcommand, Debug)]
pub enum GramCommand {
    /// Add a new entry.
    Add(AddGramArgs),

    /// List entries.
    List { prefix: Option<String> },

    /// Show an entry.
    Show { name: String },

    /// Delete an entry.
    Del { name: String },

    /// Set or get the rule field.
    Rule(RuleArgs),

    /// Set or get the description field.
    Desc(DescArgs),

    /// Set or get the category field.
    Cat(CatArgs),

    /// Set or get the level field.
    Lvl(LvlArgs),

    /// Set or get notes.
    Note(NoteArgs),

    /// Manage examples.
    Ex(ExGramArgs),

    /// Manage error examples.
    Err(ErrArgs),
}

/// Arguments for grammar/style/form add.
#[derive(Args, Debug)]
pub struct AddGramArgs {
    /// Entry name.
    pub name: String,

    /// Category.
    #[arg(short, long)]
    pub cat: Option<String>,

    /// Proficiency level.
    #[arg(short, long)]
    pub lvl: Option<String>,

    /// Rule text.
    #[arg(short, long)]
    pub rule: Option<String>,

    /// Description text.
    #[arg(short, long)]
    pub desc: Option<String>,
}

/// Arguments for the rule field.
#[derive(Args, Debug)]
pub struct RuleArgs {
    pub name: String,

    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for the description field.
#[derive(Args, Debug)]
pub struct DescArgs {
    pub name: String,

    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for the category field.
#[derive(Args, Debug)]
pub struct CatArgs {
    pub name: String,

    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for the level field.
#[derive(Args, Debug)]
pub struct LvlArgs {
    pub name: String,

    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for the notes field.
#[derive(Args, Debug)]
pub struct NoteArgs {
    pub name: String,

    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `ziziphus gram ex <name>`.
#[derive(Args, Debug)]
pub struct ExGramArgs {
    pub name: String,

    #[arg(value_name = "TEXT", allow_hyphen_values = true)]
    pub texts: Vec<String>,

    /// Example number (1-based).
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,

    /// Delete the specified example.
    #[arg(long, requires = "number", conflicts_with = "texts")]
    pub delete: bool,
}

/// Arguments for `ziziphus gram err <name>`.
#[derive(Args, Debug)]
pub struct ErrArgs {
    pub name: String,

    #[arg(value_name = "TEXT", allow_hyphen_values = true)]
    pub texts: Vec<String>,

    /// Error number (1-based).
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,

    /// Delete the specified error.
    #[arg(long, requires = "number", conflicts_with = "texts")]
    pub delete: bool,
}

/// Arguments for `ziziphus load <file.ziziphus>`.
#[derive(Args, Debug)]
pub struct LoadArgs {
    /// Path to a command file.
    pub file: String,
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_global_flags_before_command() {
        let cli = Cli::try_parse_from(["ziziphus", "--json", "--quiet", "show", "book"]).unwrap();

        assert!(cli.json);
        assert!(cli.quiet);
    }

    #[test]
    fn parses_global_flags_after_command() {
        let cli = Cli::try_parse_from(["ziziphus", "show", "book", "--json", "--quiet"]).unwrap();

        assert!(cli.json);
        assert!(cli.quiet);
    }

    #[test]
    fn parses_color_flags_before_and_after_command() {
        for args in [
            vec!["ziziphus", "--color", "list"],
            vec!["ziziphus", "list", "--color"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(cli.color);
            assert!(!cli.no_color);
        }

        for args in [
            vec!["ziziphus", "--no-color", "list"],
            vec!["ziziphus", "list", "--no-color"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(cli.no_color);
            assert!(!cli.color);
        }
    }

    #[test]
    fn list_accepts_level_alias() {
        let a = Cli::try_parse_from(["ziziphus", "list", "--lvl", "B1"]).unwrap();
        let b = Cli::try_parse_from(["ziziphus", "list", "--level", "B1"]).unwrap();
        assert_eq!(format!("{:?}", a.command), format!("{:?}", b.command));
    }

    #[test]
    fn rejects_conflicting_color_flags() {
        assert!(Cli::try_parse_from(["ziziphus", "--color", "--no-color", "list"]).is_err());

        assert!(Cli::try_parse_from(["ziziphus", "--no-color", "--color", "list"]).is_err());
    }

    #[test]
    fn parses_translation_language_and_operation() {
        let cli = Cli::try_parse_from(["ziziphus", "tr", "book", "-x", "fa", "+کتاب"]).unwrap();

        match cli.command.unwrap() {
            Command::Tr(args) => {
                assert_eq!(args.word, "book");
                assert_eq!(args.lang.as_deref(), Some("fa"));
                assert_eq!(args.ops, vec!["+کتاب"]);
            }
            _ => panic!("expected Tr command"),
        }
    }

    #[test]
    fn parses_translation_removal_operation() {
        let cli = Cli::try_parse_from(["ziziphus", "tr", "book", "-textbook"]).unwrap();

        match cli.command.unwrap() {
            Command::Tr(args) => {
                assert_eq!(args.ops, vec!["-textbook"]);
            }
            _ => panic!("expected Tr command"),
        }
    }

    #[test]
    fn parses_synonym_operations() {
        let cli = Cli::try_parse_from(["ziziphus", "syn", "book", "+volume", "-textbook"]).unwrap();

        match cli.command.unwrap() {
            Command::Syn(args) => {
                assert_eq!(args.ops, vec!["+volume", "-textbook"]);
            }
            _ => panic!("expected Syn command"),
        }
    }

    #[test]
    fn parses_example_addition() {
        let cli = Cli::try_parse_from(["ziziphus", "ex", "book", "I read a book."]).unwrap();

        match cli.command.unwrap() {
            Command::Ex(args) => {
                assert_eq!(args.word, "book");
                assert_eq!(args.texts, vec!["I read a book."]);
                assert_eq!(args.number, None);
                assert!(!args.delete);
            }
            _ => panic!("expected Ex command"),
        }
    }

    #[test]
    fn parses_example_edit() {
        let cli =
            Cli::try_parse_from(["ziziphus", "ex", "book", "--number", "2", "I read a book."])
                .unwrap();

        match cli.command.unwrap() {
            Command::Ex(args) => {
                assert_eq!(args.number, Some(2));
                assert_eq!(args.texts, vec!["I read a book."]);
                assert!(!args.delete);
            }
            _ => panic!("expected Ex command"),
        }
    }

    #[test]
    fn parses_example_deletion() {
        let cli =
            Cli::try_parse_from(["ziziphus", "ex", "book", "--delete", "--number", "2"]).unwrap();

        match cli.command.unwrap() {
            Command::Ex(args) => {
                assert_eq!(args.number, Some(2));
                assert!(args.delete);
                assert!(args.texts.is_empty());
            }
            _ => panic!("expected Ex command"),
        }
    }

    #[test]
    fn rejects_delete_example_without_number() {
        let result = Cli::try_parse_from(["ziziphus", "ex", "book", "--delete"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_delete_example_with_text() {
        let result = Cli::try_parse_from([
            "ziziphus",
            "ex",
            "book",
            "--delete",
            "--number",
            "2",
            "new example",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_example_number() {
        let result =
            Cli::try_parse_from(["ziziphus", "ex", "book", "--number", "zero", "new example"]);

        assert!(result.is_err());
    }
}
