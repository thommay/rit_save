use crate::database::{Blob, Storable};
use crate::index::Index;
use crate::workspace::Workspace;
use crate::{index, workspace, BoxResult};
use clap::ArgMatches;
use core::fmt;
use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

enum Status {
    Deleted,
    Modified,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Deleted => "D",
                Status::Modified => "M",
            }
        )
    }
}

fn trackable_file(
    workspace: &Workspace,
    index: &Index,
    path: &Path,
    stat: std::fs::Metadata,
) -> bool {
    if stat.is_file() {
        return !index.has_entry(path.to_str().unwrap());
    }
    if !stat.is_dir() {
        return false;
    }

    let items = workspace.list_dir(Some(path.to_path_buf())).unwrap();

    for (path, stat) in items {
        if !stat.is_file() && !stat.is_dir() {
            continue;
        }
        if trackable_file(&workspace, &index, path.as_path(), stat) {
            return true;
        }
    }
    false
}

fn scan_workspace(
    workspace: &Workspace,
    index: &Index,
    path: Option<PathBuf>,
) -> BoxResult<(Vec<String>, BTreeMap<PathBuf, Metadata>)> {
    let mut untracked = Vec::new();
    let mut stats = BTreeMap::new();
    for (file, stat) in workspace.list_dir(path)? {
        if index.has_entry(file.to_str().unwrap()) {
            if stat.is_dir() {
                let (mut u, s) = scan_workspace(workspace, index, Some(file))?;
                untracked.append(&mut u);
                stats.extend(s);
            } else {
                stats.insert(file, stat);
            }
        } else if trackable_file(workspace, index, file.as_path(), stat.clone()) {
            let mut file = file.to_str().unwrap().to_owned();
            if stat.is_dir() {
                file.push('/');
            }
            untracked.push(file);
        }
    }
    Ok((untracked, stats))
}

fn detect_changes(
    workspace: Workspace,
    index: &mut Index,
    stats: BTreeMap<PathBuf, Metadata>,
) -> BoxResult<BTreeMap<String, Status>> {
    let mut changed = BTreeMap::new();
    for entry in index.entries() {
        let name = entry.path.to_str().unwrap().to_string();
        let stat = stats.get(&entry.path);
        if stat.is_none() {
            changed.insert(name, Status::Deleted);
            continue;
        }
        if entry.stat_match(stat) {
            if entry.stat_times_match(stat) {
                continue;
            }
            let data = workspace.read_file(&entry.path)?;
            let blob = Blob::new(data);
            if entry.oid == blob.oid() {
                index.add(&entry.path, blob.oid().as_ref(), stat.unwrap().clone());
                continue;
            }
        }
        changed.insert(name, Status::Modified);
    }
    Ok(changed)
}

pub fn exec(_matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let workspace = workspace::Workspace::new(root);
    let mut index = index::Index::from(root.join(".git/index"))?;

    let (mut untracked, stats) = scan_workspace(&workspace, &index, None)?;

    for (file, status) in detect_changes(workspace, &mut index, stats)? {
        println!(" {} {}", status, file);
    }

    untracked.sort();
    for file in untracked {
        println!("?? {}", file);
    }
    index.write_updates()?;
    Ok(())
}
