use assert_cmd::prelude::*;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
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

fn mkdir(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    std::fs::create_dir_all(dn)?;
    Ok(())
}

fn make_executable(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    let perms = std::fs::Permissions::from_mode(0o0755);
    std::fs::set_permissions(dn, perms)?;
    Ok(())
}

fn delete(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    let m = std::fs::metadata(&dn)?;
    if m.is_dir() {
        std::fs::remove_dir_all(dn)?;
    } else {
        std::fs::remove_file(dn)?;
    }
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

fn prepare_commits(repo: &TempDir, files: Vec<&str>) -> Result<(), std::io::Error> {
    for file in files {
        write_file(repo, file, file, true)?;
    }
    commit(repo, "commit")
}

#[test]
fn quiet_when_nothing() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout("");
    Ok(())
}

#[test]
fn reports_deleted_files() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    delete(&repo, "1.txt")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#" D 1.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_files_in_deleted_dir() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    delete(&repo, "a")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#" D a/2.txt
 D a/b/3.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_files_with_modified_contents() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    write_file(&repo, "1.txt", "changed", false)?;
    write_file(&repo, "a/2.txt", "modified", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#" M 1.txt
 M a/2.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_files_with_modified_contents_but_same_size() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    write_file(&repo, "1.txt", "hello", false)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#" M 1.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_files_with_modified_mode() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    make_executable(&repo, "1.txt")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            r#" M 1.txt
"#,
        );
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
