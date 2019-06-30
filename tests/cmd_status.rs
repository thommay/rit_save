use assert_cmd::prelude::*;
use std::process::Command;

mod helpers;
use helpers::*;

#[test]
fn quiet_when_nothing() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#" M 1.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_added_files() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    write_file(&repo, "a/4.txt", "hello", true)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"A  a/4.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_added_file_in_untracked_dirs() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    write_file(&repo, "d/e/4.txt", "hello", true)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"A  d/e/4.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_tracked_files_with_changed_mode() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    make_executable(&repo, "a/2.txt")?;
    add_file(&repo, "a/2.txt")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"M  a/2.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_tracked_files_with_changed_content() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    write_file(&repo, "a/2.txt", "changed", true)?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"M  a/2.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_deleted_tracked_files() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    delete(&repo, "a/2.txt")?;
    delete(&repo, ".git/index")?;
    add_file(&repo, ".")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"D  a/2.txt
"#,
        );
    Ok(())
}

#[test]
fn reports_all_deleted_tracked_files_in_directories() -> Result<(), Box<std::error::Error>> {
    let repo = prepare_repo()?;
    prepare_commits(&repo, vec!["1.txt", "a/2.txt", "a/b/3.txt"])?;
    delete(&repo, "a")?;
    delete(&repo, ".git/index")?;
    add_file(&repo, ".")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(repo.path())
        .arg("status")
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"D  a/2.txt
D  a/b/3.txt
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
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
        .arg("--porcelain")
        .assert()
        .success()
        .stdout(
            r#"?? outer/
"#,
        );
    Ok(())
}
