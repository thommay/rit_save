use crate::database::{Blob, Storable};
use crate::index::Index;
use crate::workspace::Workspace;
use crate::{index, workspace, BoxResult, database, refs, commit, tree};
use clap::ArgMatches;
use core::fmt;
use failure::Error;
use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use crate::database::marker::{Marker, Kind};
use crate::tree::TreeEntry;
use crate::index::entry::Entry;

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Deleted,
    Modified,
    IndexModified,
    IndexAdded,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Deleted => "D",
                Status::Modified | Status::IndexModified => "M",
                Status::IndexAdded => "A",
            }
        )
    }
}


pub struct CmdStatus {
    workspace: Workspace,
    index: Index,
    database: database::Database,
    refs: refs::Refs,
    changes: BTreeMap<String, Vec<Status>>,
    untracked: Vec<String>,
    stats: BTreeMap<PathBuf, Metadata>,
    tree: BTreeMap<PathBuf, Marker>,
}

impl CmdStatus {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, Error> {
        let root = root.as_ref();
        let workspace = workspace::Workspace::new(root);
        let index = index::Index::from(root.join(".git/index"))?;
        let database = database::Database::new(root.join(".git/objects"));

        let refs = refs::Refs::new(root.join(".git"));

        let untracked = vec![];
        let changes = BTreeMap::new();
        let stats = BTreeMap::new();
        let tree = BTreeMap::new();
        Ok(CmdStatus { workspace, index, database, refs, changes, stats, untracked, tree })
    }

    pub fn exec(mut self, _matches: &ArgMatches) -> BoxResult<()> {
        self.scan_workspace(None)?;

        let mut has_tree = false;
        if let Some(head) = self.refs.get_head() {
            has_tree = true;
            self.read_tree(head.as_ref(), "".into())?;
        }


        for entry in self.index.entries() {
            self.check_index_against_workspace(&entry)?;
            if has_tree {
                self.check_index_against_tree(&entry)?;
            }
        }

        for (file, status) in  self.changes {
            println!("{} {}", status_for(status), file);
        }

        self.untracked.sort();
        for file in self.untracked {
            println!("?? {}", file);
        }
        self.index.write_updates()?;
        Ok(())
    }

    fn record_change(&mut self, name: String, status: Status) {
        self.changes
            .entry(name)
            .and_modify(|e| e.push(status))
            .or_insert_with(|| vec![status]);
    }

    fn scan_workspace(&mut self, path: Option<PathBuf>) -> BoxResult<()> {
        for (file, stat) in self.workspace.list_dir(path)? {
            if self.index.has_entry(file.to_str().unwrap()) {
                if stat.is_dir() {
                    self.scan_workspace(Some(file))?;
                } else {
                    self.stats.insert(file, stat);
                }
            } else if self.trackable_file(file.as_path(), stat.clone()) {
                let mut file = file.to_str().unwrap().to_owned();
                if stat.is_dir() {
                    file.push('/');
                }
                self.untracked.push(file);
            }
        }
        Ok(())
    }

    fn check_index_against_tree(&mut self, entry: &Entry) -> BoxResult<()> {
        let name = entry.path.to_str().unwrap().to_string();
        let item = self.tree.get(&entry.path);
        if let Some(item) = item {
            if item.oid != entry.oid || item.mode != entry.mode() {
                self.record_change(name, Status::IndexModified);
            }
        } else {
            self.record_change(name, Status::IndexAdded);
        }
        Ok(())
    }

    fn check_index_against_workspace(&mut self, entry: &Entry) -> BoxResult<()> {
        let name = entry.path.to_str().unwrap().to_string();
        let stat = self.stats.get(&entry.path);
        if stat.is_none() {
            self.record_change(name, Status::Deleted);
            return Ok(());
        }
        if entry.stat_match(stat) {
            if entry.stat_times_match(stat) {
                return Ok(());
            }
            let data = self.workspace.read_file(&entry.path)?;
            let blob = Blob::new(data);
            if entry.oid == blob.oid() {
                self.index.add(&entry.path, blob.oid().as_ref(), stat.unwrap().clone());
                return Ok(());
            }
        }
        self.record_change(name, Status::Modified);
        Ok(())
    }

    fn trackable_file(&self, path: &Path, stat: std::fs::Metadata) -> bool {
        if stat.is_file() {
            return !self.index.has_entry(path.to_str().unwrap());
        }
        if !stat.is_dir() {
            return false;
        }

        let items = self.workspace.list_dir(Some(path.to_path_buf())).unwrap();

        for (path, stat) in items {
            if !stat.is_file() && !stat.is_dir() {
                continue;
            }
            if self.trackable_file(path.as_path(), stat) {
                return true;
            }
        }
        false
    }

    fn read_tree(&mut self, oid: &str, path: PathBuf) -> BoxResult<()>{
        let (kind, _size, data) = self.database.read_object(oid)?;
        if kind == "tree" {
            let tree = tree::Tree::from(data)?;
            for (name, entry) in tree.entries {
                let p = path.join(name);
                if let TreeEntry::Marker(entry) = entry {
                    match entry.kind() {
                        Kind::Entry => { self.tree.insert(p, entry); },
                        Kind::Tree => self.read_tree(entry.oid.as_ref(), p)?,
                    };
                }
            }
        } else if kind == "commit" {
            let commit = commit::Commit::from(data)?;
            let tree = commit.tree;
            self.read_tree(tree.as_ref(), path)?;
        }
        Ok(())
    }
}

fn status_for(status: Vec<Status>) -> String {
    let left = if status.iter().any(|&n| n == Status::IndexAdded) {
        "A"
    } else if status.iter().any(|&n| n == Status::IndexModified) {
        "M"
    } else {
        " "
    };
    let right = if status.iter().any(|&n| n == Status::Deleted) {
        "D"
    } else if status.iter().any(|&n| n == Status::Modified) {
        "M"
    } else {
        " "
    };
    format!("{}{}", left, right)
}

