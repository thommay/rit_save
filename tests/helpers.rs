use assert_cmd::prelude::*;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempdir::TempDir;

pub fn prepare_repo() -> Result<TempDir, std::io::Error> {
    let tmp = TempDir::new("rit")?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir(tmp.path())
        .arg("init")
        .arg(tmp.path())
        .assert()
        .success();
    Ok(tmp)
}

pub fn write_file(
    repo: &TempDir,
    path: &str,
    content: &str,
    add: bool,
) -> Result<(), std::io::Error> {
    let new_file = repo.path().join(path);
    let dn = new_file.parent().unwrap();
    std::fs::create_dir_all(dn)?;
    let mut f = File::create(new_file)?;
    f.write_all(content.as_bytes())?;
    if add {
        add_file(repo, path)?;
    }
    Ok(())
}

pub fn add_file(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir(repo.path())
        .arg("add")
        .arg(path)
        .assert()
        .success();
    Ok(())
}

pub fn mkdir(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    std::fs::create_dir_all(dn)?;
    Ok(())
}

pub fn make_executable(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    let perms = std::fs::Permissions::from_mode(0o0755);
    std::fs::set_permissions(dn, perms)?;
    Ok(())
}

pub fn delete(repo: &TempDir, path: &str) -> Result<(), std::io::Error> {
    let dn = repo.path().join(path);
    let m = std::fs::metadata(&dn)?;
    if m.is_dir() {
        std::fs::remove_dir_all(dn)?;
    } else {
        std::fs::remove_file(dn)?;
    }
    Ok(())
}

pub fn commit(repo: &TempDir, message: &str) -> Result<(), std::io::Error> {
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

pub fn prepare_commits(repo: &TempDir, files: Vec<&str>) -> Result<(), std::io::Error> {
    for file in files {
        write_file(repo, file, file, true)?;
    }
    commit(repo, "commit")
}
