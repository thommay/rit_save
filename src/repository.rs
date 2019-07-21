use crate::database::marker::{Kind, Marker};
use crate::database::{Blob, Storable};
use crate::index::entry::Entry;
use crate::tree::TreeEntry;
use crate::{commit, database, index, refs, tree, workspace, BoxResult};
use failure::Error;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Changed {
    Index,
    Workspace,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Deleted,
    Modified,
    Added,
    None,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

pub struct Repository {
    pub workspace: workspace::Workspace,
    pub index: index::Index,
    pub database: database::Database,
    refs: refs::Refs,
    pub index_changes: BTreeMap<String, Status>,
    pub workspace_changes: BTreeMap<String, Status>,
    pub changed: Vec<String>,
    pub untracked: Vec<String>,
    pub stats: BTreeMap<PathBuf, Metadata>,
    pub tree: BTreeMap<PathBuf, Marker>,
}

impl Repository {
    pub fn new<P: AsRef<Path>>(root: P) -> BoxResult<Self> {
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

        Ok(Repository {
            workspace,
            index,
            database,
            refs,
            untracked,
            changed,
            index_changes,
            workspace_changes,
            stats,
            tree,
        })
    }

    pub fn status(&mut self) -> BoxResult<()> {
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
        Ok(())
    }

    pub fn commit_changes(self) -> Result<(), Error> {
        self.index.write_updates()
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
}
