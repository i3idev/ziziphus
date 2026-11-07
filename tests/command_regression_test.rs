//! Regression tests for example and grammar error commands.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn ziziphus() -> Command {
    Command::cargo_bin("ziziphus").unwrap()
}

fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempdir().unwrap();
    let home = root.path().join("home");

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .arg("init")
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["db", "new", "demo"])
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["db", "use", "demo"])
        .assert()
        .success();

    (root, home)
}

#[test]
fn ex_dash_returns_error_without_panicking() {
    let (_root, home) = setup();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["add", "example-word", "معنی", "-p", "n"])
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["ex", "example-word", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("panicked").not());
}

#[test]
fn gram_err_number_edits_existing_error() {
    let (_root, home) = setup();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["gram", "add", "test-rule", "--rule", "rule"])
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["gram", "err", "test-rule", "original error"])
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["gram", "err", "test-rule", "--number", "1", "updated error"])
        .assert()
        .success();

    ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["gram", "err", "test-rule"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated error"))
        .stdout(predicate::str::contains("original error").not());
}
