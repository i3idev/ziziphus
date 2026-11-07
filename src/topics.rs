//! Topic-based help for the CLI.
//!
//! `Ziziphus topics` prints extended help on specific topics that go beyond
//! the built-in CLI usage summary. Topics are defined as string
//! constants in this module and printed by [`print_topic`].
//!
//! Available topics:
//!
//! - `examples` — End-to-end walkthroughs

use crate::error::ZiziphusError;

/// List of available help topics.
///
/// Each entry is `(name, short description)`.
pub const TOPICS: &[(&str, &str)] = &[("examples", "End-to-end walkthroughs")];

/// Print the list of available topics.
pub fn list_topics() {
    use crate::style;
    println!("{}", style::header("ziziphus topics — extended help"));
    println!();
    println!("Usage: {} <TOPIC>", style::key("ziziphus topics"));
    println!();
    println!("{}", style::key("Available topics:"));
    for (name, desc) in TOPICS {
        // Pad *before* coloring, so ANSI codes don't break the alignment.
        let padded = format!("{:<12}", name);
        println!("  {} {}", style::accent(&padded), desc);
    }
    println!();
    println!(
        "Run `{}` for details.",
        style::key("ziziphus topics <TOPIC>")
    );
}

/// Print a specific topic, or list all if name is `None`.
///
/// # Errors
///
/// Returns an error for unknown topic names.
pub fn print_topic(name: Option<&str>) -> Result<(), ZiziphusError> {
    let Some(name) = name else {
        list_topics();
        return Ok(());
    };
    let body = match name {
        "examples" => EXAMPLES,
        other => {
            return Err(ZiziphusError::msg(format!(
                "unknown topic '{}'. run `ziziphus topics` to list",
                other
            )));
        }
    };
    print!("{}", body);
    Ok(())
}

const EXAMPLES: &str = r#"ziziphus topics — End-to-end walkthroughs

SETUP
  ziziphus init
  ziziphus db new demo
  ziziphus add book کتاب -p n -l A1

WORD MANAGEMENT
  ziziphus show book
  ziziphus tr book +کتاب
  ziziphus ex book "I read a book."
  ziziphus find book
  ziziphus list

GRAMMAR
  ziziphus gram add present-simple
  ziziphus gram list
  ziziphus gram show present-simple

DETAILED GUIDE
  docs/tutorials.md — complete end-to-end workflows
  docs/command-reference.md — full command reference
"#;
