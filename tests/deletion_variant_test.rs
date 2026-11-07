//! Regression tests for deletion with normalised (Arabic/Persian variant)
//! names.

use assert_cmd::Command;
use tempfile::tempdir;

fn ziziphus() -> Command {
    Command::cargo_bin("ziziphus").unwrap()
}

#[test]
fn delete_word_using_arabic_variant_removes_record() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["db", "new", "demo"])
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["db", "use", "demo"])
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["add", "کتاب", "book", "-p", "n"])
        .assert()
        .success();

    // Delete using the Arabic-kaf variant.
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["del", "كتاب", "--yes"])
        .assert()
        .success();

    let out = ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("کتاب"),
        "entry should be deleted: {stdout}"
    );
}

#[test]
fn delete_grammar_using_arabic_variant_removes_record() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["db", "new", "demo"])
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["db", "use", "demo"])
        .assert()
        .success();
    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["gram", "add", "کتاب", "--rule", "r"])
        .assert()
        .success();

    let mut cmd = ziziphus();
    cmd.env("ZIZIPHUS_HOME", &home)
        .args(["gram", "del", "كتاب"])
        .write_stdin("كتاب\n")
        .assert()
        .success();

    let out = ziziphus()
        .env("ZIZIPHUS_HOME", &home)
        .args(["gram", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("کتاب"),
        "entry should be deleted: {stdout}"
    );
}
