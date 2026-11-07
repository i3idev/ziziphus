//! ANSI colour helpers for CLI output.
//!
//! Colours are emitted only when **all** of these hold (in [`Mode::Auto`]):
//!
//! - the target stream is a terminal (stdout, or stderr for errors),
//! - `NO_COLOR` is unset or empty,
//! - `TERM` is not `dumb`.
//!
//! The auto-detection result is computed once, on first use, and then
//! cached. Call [`set_mode`] from `main` (e.g. for `--color` / `--no-color` flags)
//! to override it with [`Mode::Never`] or [`Mode::Always`].
//!
//! Styles do not nest: `bold(red("x"))` loses the bold after the inner
//! reset sequence.

use std::io::IsTerminal;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// How colour output is decided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mode {
    /// Decide from the environment (TTY, `NO_COLOR`, `TERM`).
    Auto = 0,
    /// Never emit colour codes.
    Never = 1,
    /// Always emit colour codes.
    Always = 2,
}

static MODE: AtomicU8 = AtomicU8::new(Mode::Auto as u8);
static AUTO: OnceLock<bool> = OnceLock::new();

/// Set the colour mode.
pub fn set_mode(mode: Mode) {
    MODE.store(mode as u8, Ordering::Relaxed);
}

/// Get the current colour mode.
pub fn get_mode() -> Mode {
    current_mode()
}

/// Pure colour decision, kept free of globals so it is easy to test.
fn decide(mode: Mode, is_tty: bool, no_color: Option<&str>, term: Option<&str>) -> bool {
    match mode {
        Mode::Never => false,
        Mode::Always => true,
        Mode::Auto => is_tty && no_color.is_none_or(str::is_empty) && term != Some("dumb"),
    }
}

fn current_mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        1 => Mode::Never,
        2 => Mode::Always,
        _ => Mode::Auto,
    }
}

static AUTO_ERR: OnceLock<bool> = OnceLock::new();

fn auto_decide(is_tty: bool) -> bool {
    let no_color = std::env::var_os("NO_COLOR").map(|v| v.to_string_lossy().into_owned());
    let term = std::env::var_os("TERM").map(|v| v.to_string_lossy().into_owned());
    decide(Mode::Auto, is_tty, no_color.as_deref(), term.as_deref())
}

/// Whether colours should be emitted on stdout right now.
pub fn enabled() -> bool {
    match current_mode() {
        Mode::Auto => *AUTO.get_or_init(|| auto_decide(std::io::stdout().is_terminal())),
        mode => decide(mode, false, None, None),
    }
}

/// Whether colours should be emitted on stderr right now.
pub fn enabled_stderr() -> bool {
    match current_mode() {
        Mode::Auto => *AUTO_ERR.get_or_init(|| auto_decide(std::io::stderr().is_terminal())),
        mode => decide(mode, false, None, None),
    }
}

fn paint(on: bool, code: &str, s: &str) -> String {
    if on {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

fn wrap(code: &str, s: &str) -> String {
    paint(enabled(), code, s)
}

/// Bold text.
pub fn bold(s: &str) -> String {
    wrap("1", s)
}
/// Dim text.
pub fn dim(s: &str) -> String {
    wrap("2", s)
}
/// Red text.
#[allow(dead_code)]
pub fn red(s: &str) -> String {
    wrap("31", s)
}
/// Green text.
pub fn green(s: &str) -> String {
    wrap("32", s)
}
/// Yellow text.
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
/// Magenta text.
pub fn magenta(s: &str) -> String {
    wrap("35", s)
}
/// Cyan text.
pub fn cyan(s: &str) -> String {
    wrap("36", s)
}

/// Section header (bold).
pub fn header(s: &str) -> String {
    bold(s)
}
/// Key/label (cyan).
pub fn key(s: &str) -> String {
    cyan(s)
}
/// Success marker (green).
pub fn ok(s: &str) -> String {
    green(s)
}
/// Warning marker (yellow).
pub fn warn(s: &str) -> String {
    yellow(s)
}
/// Error marker (red).
pub fn err(s: &str) -> String {
    paint(enabled_stderr(), "31", s)
}
/// Accent (magenta).
pub fn accent(s: &str) -> String {
    magenta(s)
}

#[cfg(test)]
mod decide_tests {
    use super::*;

    #[test]
    fn auto_respects_environment() {
        assert!(decide(Mode::Auto, true, None, Some("xterm")));
        assert!(decide(Mode::Auto, true, Some(""), None));
        assert!(!decide(Mode::Auto, true, Some("1"), None));
        assert!(!decide(Mode::Auto, true, None, Some("dumb")));
        assert!(!decide(Mode::Auto, false, None, None));
    }

    #[test]
    fn explicit_modes_win() {
        assert!(decide(Mode::Always, false, Some("1"), Some("dumb")));
        assert!(!decide(Mode::Never, true, None, None));
    }
}
