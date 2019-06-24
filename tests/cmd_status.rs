use assert_cmd::prelude::*;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempdir::TempDir;

fn prepare_repo() -> Result<TempDir, std::io::Error> {
    let tmp = TempDir::new("rit")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir(tmp.path())
        .arg("init")
        .arg(tmp.path())
        .assert()
        .success();
    Ok(tmp)
}

fn write_file(repo: &TempDir, path: &str, content: &str, add: bool) -> Result<(), std::io::Error> {
    let new_file = repo.path().join(path);
    let dn = new_file.parent().unwrap();
    std::fs::create_dir_all(dn)?;
    let mut f = File::create(new_file)?;
    f.write_all(content.as_bytes())?;
    if add {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.current_dir(repo.path())
            .arg("add")
            .arg(path)
            .assert()
            .success();
    }
    Ok(())
}

fn mkdir(repo: &TempDir, path: &str)  -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    std::fs::create_dir_all(dn)?;
    Ok(())
}

fn commit(repo: &TempDir, message: &str) -> Result<(), std::io::Error> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_AUTHOR_NAME", "A. U. Thor")
        .current_dir(repo.path())
        .arg("commit")
        .arg("-m")
        .arg(message)
        .assert()
        .success();
    Ok(())
}

#[test]
fn untracked_files() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    write_file(&repo, "file.txt", "hello", false)?;
    write_file(&repo, "another.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#"?? another.txt
?? file.txt
"#,
        );
    Ok(())
}

#[test]
fn untracked_files_not_indexed() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    write_file(&repo, "committed.txt", "hello", true)?;
    commit(&repo, "commit message")?;
    write_file(&repo, "file.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#"?? file.txt
"#,
        );
    Ok(())
}

#[test]
fn lists_untracked_directories() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    write_file(&repo, "file.txt", "hello", false)?;
    write_file(&repo, "dir/another.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#"?? dir/
?? file.txt
"#,
        );
    Ok(())
}

#[test]
fn lists_untracked_files_in_tracked_directories() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    write_file(&repo, "a/b/inner.txt", "hello", true)?;
    commit(&repo, "commit")?;
    write_file(&repo, "a/outer.txt", "hello", false)?;
    write_file(&repo, "a/b/c/file.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#"?? a/b/c/
?? a/outer.txt
"#,
        );
    Ok(())
}

#[test]
fn does_not_list_empty_untracked_dirs() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    mkdir(&repo, "outer")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout("");
    Ok(())
}

#[test]
fn lists_untracked_dirs_that_contain_files() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    write_file(&repo, "outer/inner/file.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#"?? outer/
"#,
        );
    Ok(())
}
