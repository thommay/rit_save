use crate::database::marker::{Kind, Marker};
use crate::database::{Blob, Storable};
use crate::index::entry::Entry;
use crate::index::Index;
use crate::tree::TreeEntry;
use crate::workspace::Workspace;
use crate::{commit, database, index, refs, tree, workspace, BoxResult};
use clap::ArgMatches;
use colored::*;
use core::fmt;
use failure::Error;
use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
enum Changed {
    Index,
    Workspace,
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Deleted,
    Modified,
    Added,
    None,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Deleted => "D",
                Status::Modified => "M",
                Status::Added => "A",
                Status::None => " ",
            }
        )
    }
}

pub struct CmdStatus {
    workspace: Workspace,
    index: Index,
    database: database::Database,
    refs: refs::Refs,
    index_changes: BTreeMap<String, Status>,
    workspace_changes: BTreeMap<String, Status>,
    changed: Vec<String>,
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
        let changed = vec![];
        let index_changes = BTreeMap::new();
        let workspace_changes = BTreeMap::new();
        let stats = BTreeMap::new();
        let tree = BTreeMap::new();
        Ok(CmdStatus {
            workspace,
            index,
            database,
            refs,
            changed,
            index_changes,
            workspace_changes,
            stats,
            untracked,
            tree,
        })
    }

    pub fn exec(mut self, matches: &ArgMatches) -> BoxResult<()> {
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
                self.check_deleted_tree_files();
            }
        }

        self.untracked.sort();
        self.untracked.dedup();

        if matches.is_present("porcelain") {
            self.print_porcelain();
        } else {
            self.print_long_format();
        }
        self.index.write_updates()?;
        Ok(())
    }

    fn print_long_format(&self) {
        let index = self.index_changes.clone();
        let workspace = self.workspace_changes.clone();
        let untracked = self.untracked.clone();

        print_changes("Changes to be committed", index, "green");
        print_changes("Changes not staged for commit", workspace, "red");

        if !untracked.is_empty() {
            println!("Untracked files");
            println!();
            for file in untracked {
                println!("\t{}", file.red());
            }
            println!();
        }
        self.print_status();
    }

    fn print_status(&self) {
        if !self.index_changes.is_empty() {
            return;
        }
        if !self.workspace_changes.is_empty() {
            println!("no changes added to commit");
        } else if !self.untracked.is_empty() {
            println!("nothing added to commit but untracked files present");
        } else {
            println!("nothing to commit, working tree clean");
        }
    }

    fn print_porcelain(&self) {
        let mut changed = self.changed.clone();
        changed.sort();
        changed.dedup();

        let untracked = self.untracked.clone();

        for file in changed {
            println!("{} {}", self.status_for(&file), file);
        }

        for file in untracked {
            println!("?? {}", file);
        }
    }

    fn record_change(&mut self, name: String, target: Changed, status: Status) {
        self.changed.push(name.clone());
        if target == Changed::Workspace {
            self.workspace_changes.insert(name, status);
        } else {
            self.index_changes.insert(name, status);
        }
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

    fn check_deleted_tree_files(&mut self) {
        let mut deleted = vec![];
        for path in self.tree.keys() {
            let p = path.to_str().unwrap();
            if !self.index.has_entry(&p) {
                deleted.push(p.to_string().clone());
            }
        }
        for file in deleted {
            self.record_change(file, Changed::Index, Status::Deleted);
        }
    }

    fn check_index_against_tree(&mut self, entry: &Entry) -> BoxResult<()> {
        let name = entry.path.to_str().unwrap().to_string();
        let item = self.tree.get(&entry.path);
        if let Some(item) = item {
            if item.oid != entry.oid || item.mode != entry.mode() {
                self.record_change(name, Changed::Index, Status::Modified);
            }
        } else {
            self.record_change(name, Changed::Index, Status::Added);
        }
        Ok(())
    }

    fn check_index_against_workspace(&mut self, entry: &Entry) -> BoxResult<()> {
        let name = entry.path.to_str().unwrap().to_string();
        let stat = self.stats.get(&entry.path);
        if stat.is_none() {
            self.record_change(name, Changed::Workspace, Status::Deleted);
            return Ok(());
        }
        if entry.stat_match(stat) {
            if entry.stat_times_match(stat) {
                return Ok(());
            }
            let data = self.workspace.read_file(&entry.path)?;
            let blob = Blob::new(data);
            if entry.oid == blob.oid() {
                self.index
                    .add(&entry.path, blob.oid().as_ref(), stat.unwrap().clone());
                return Ok(());
            }
        }
        self.record_change(name, Changed::Workspace, Status::Modified);
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

    fn read_tree(&mut self, oid: &str, path: PathBuf) -> BoxResult<()> {
        let (kind, _size, data) = self.database.read_object(oid)?;
        if kind == "tree" {
            let tree = tree::Tree::from(data)?;
            for (name, entry) in tree.entries {
                let p = path.join(name);
                if let TreeEntry::Marker(entry) = entry {
                    match entry.kind() {
                        Kind::Entry => {
                            self.tree.insert(p, entry);
                        }
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

    fn status_for(&self, file: &str) -> String {
        format!(
            "{}{}",
            self.index_changes.get(file).unwrap_or(&Status::None),
            self.workspace_changes.get(file).unwrap_or(&Status::None)
        )
    }
}

fn long_format(status: Status) -> String {
    match status {
        Status::Deleted => String::from("deleted:"),
        Status::Modified => String::from("modified:"),
        Status::Added => String::from("new file:"),
        Status::None => String::new(),
    }
}

fn print_changes(msg: &str, index: BTreeMap<String, Status>, colour: &str) {
    if !index.is_empty() {
        println!("{}", msg);
        println!();
        for (path, status) in index {
            let item = format!("{:12}{}", long_format(status), path).color(colour);
            println!("\t{}", item);
        }
        println!();
    }
}
