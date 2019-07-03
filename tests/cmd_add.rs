use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempdir::TempDir;

use rit::BoxResult;

fn prepare_repo() -> BoxResult<TempDir> {
    let tmp = TempDir::new("rit")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("init").arg(tmp.path()).assert().success();
    Ok(tmp)
}

#[test]
fn add_regular_file() -> BoxResult<()> {
    let repo = prepare_repo()?;
    let new_file = repo.path().join("hello.txt");
    let mut hello = File::create(new_file)?;
    writeln!(hello, "hello")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("add")
        .arg("hello.txt")
        .assert()
        .success();
    Ok(())
}

#[test]
fn add_missing_file() -> BoxResult<()> {
    let repo = prepare_repo()?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("add")
        .arg("derp.txt")
        .assert()
        .code(predicate::eq(128))
        .stderr(predicate::str::contains("pathspec 'derp.txt'"));
    Ok(())
}
