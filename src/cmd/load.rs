//! Load command: run a batch of commands from a `.ziziphus` file.
//!
//! Each non-empty, non-comment line is parsed with `shlex` and executed
//! via [`crate::run::run_internal`]. The leading `ziziphus` token is
//! optional. `load` inside a file is forbidden. Errors on a line are
//! reported but do not stop the batch. Output is suppressed while the
//! file is processed; a summary is printed at the end.

use shlex::split;
use std::path::Path;

use crate::error::ZiziphusError;
use crate::model::{STATE, set_quiet};
use crate::run::run_internal;

use std::path::PathBuf;

/// Restores process-wide settings when batch execution exits.
struct LoadStateGuard {
    quiet: bool,
    color_mode: crate::style::Mode,
    vault: Option<PathBuf>,
}

impl Drop for LoadStateGuard {
    fn drop(&mut self) {
        set_quiet(self.quiet);
        crate::style::set_mode(self.color_mode);

        match &self.vault {
            Some(path) => {
                let value = path.to_string_lossy();
                crate::db::set_vault(Some(value.as_ref()));
            }
            None => crate::db::set_vault(None),
        }
    }
}

/// `ziziphus load <file.ziziphus>`
///
/// Execute commands from a text file. One command per line; `#`
/// starts a comment. Empty lines are skipped.
///
/// # Errors
///
/// - `usage: ziziphus load <file.ziziphus>` — no file
/// - `no such file: <path>` — file does not exist
/// - `line N: parse error` — shlex parse failure
/// - `line N: load inside a file is not allowed` — nested load
pub fn cmd_load(args: &[String], db_override: Option<&str>) -> Result<(), ZiziphusError> {
    if args.is_empty() {
        return Err(ZiziphusError::msg("usage: ziziphus load <file.ziziphus>"));
    }
    let path = Path::new(&args[0]);
    if !path.exists() {
        return Err(ZiziphusError::msg(format!(
            "no such file: {}",
            path.display()
        )));
    }
    let content = std::fs::read_to_string(path)?;
    let mut done = 0;
    let mut bad = 0;
    let previous_quiet = {
        let state = STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.quiet
    };

    let _state_guard = LoadStateGuard {
        quiet: previous_quiet,
        color_mode: crate::style::get_mode(),
        vault: crate::db::get_vault(),
    };

    set_quiet(true);

    for (no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = match split(line) {
            Some(p) => p,
            None => {
                eprintln!("line {}: parse error", no + 1);
                bad += 1;
                continue;
            }
        };
        if parts.is_empty() {
            continue;
        }
        let parts: Vec<String> = parts.into_iter().collect();
        // Optional leading program name ("ziziphus").
        let parts = if parts[0] == "ziziphus" {
            &parts[1..]
        } else {
            &parts[..]
        };
        if parts.is_empty() {
            continue;
        }
        if parts[0] == "load" {
            eprintln!("line {}: load inside a file is not allowed", no + 1);
            bad += 1;
            continue;
        }
        match run_internal(parts.to_vec(), db_override) {
            Ok(()) => done += 1,
            Err(e) => {
                eprintln!("line {}: {}", no + 1, e);
                bad += 1;
            }
        }
    }

    println!("loaded {} commands, {} errors", done, bad);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_continues_after_error_and_restores_output_state() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("batch.ziziphus");

        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "load nested.ziziphus").unwrap();
        writeln!(file, "topics --no-color").unwrap();

        let previous_quiet = {
            let state = STATE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.quiet
        };
        let previous_color = crate::style::get_mode();
        let previous_vault = crate::db::get_vault();

        crate::model::set_quiet(previous_quiet);
        crate::style::set_mode(crate::style::Mode::Always);

        let result = cmd_load(&[file_path.to_string_lossy().into_owned()], None);

        assert!(result.is_ok());

        let current_quiet = {
            let state = STATE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.quiet
        };

        assert_eq!(current_quiet, previous_quiet);
        assert_eq!(crate::style::get_mode(), crate::style::Mode::Always);
        assert_eq!(crate::db::get_vault(), previous_vault);

        crate::style::set_mode(previous_color);
    }
}
