//! Integration tests for CLI command dispatch.
//!
//! These tests verify that the `ziziphus` binary correctly routes commands.

use assert_cmd::Command;
use predicates::prelude::*;
/// Existing word commands still work (e.g., `ziziphus add` without args gives usage).
#[test]
fn test_existing_commands_still_work() {
    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.arg("add");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("WORD").or(predicate::str::contains("usage")));
}

/// `gxx help` should still work and show usage.
#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.arg("help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ziziphus - A CLI tool"));
}

/// `gxx --help` should still work and show usage.
#[test]
fn test_help_long_alias() {
    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ziziphus - A CLI tool"));
}
/// Verify that `gxx` with no args prints usage (exit code 0).
#[test]
fn test_no_args_prints_usage() {
    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.assert().success().stdout(predicate::str::contains(
        "ziziphus - word and grammar notebook",
    ));
}
/// `ziziphus init` should work (database initialization).
#[test]
fn test_init_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let home = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.env("ZIZIPHUS_HOME", home);
    cmd.arg("init");
    cmd.assert().success();
}

/// `ziziphus db list` should work after init.
#[test]
fn test_db_list_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let home = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.env("ZIZIPHUS_HOME", home);
    cmd.arg("init");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ziziphus").unwrap();
    cmd.env("ZIZIPHUS_HOME", home);
    cmd.arg("db").arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no databases").or(predicate::str::contains("Databases")));
}
