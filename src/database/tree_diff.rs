use crate::commit::Commit;
use crate::database::{Database, ObjectKind, Storable};
use crate::tree::{Tree, TreeEntry};
use failure::format_err;
use failure::Error;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

pub type TreeDifference = HashMap<PathBuf, (Option<TreeEntry>, Option<TreeEntry>)>;

#[derive(Debug)]
pub struct TreeDiff<'a> {
    pub changes: TreeDifference,
    db: &'a Database,
}

impl<'a> TreeDiff<'a> {
    pub fn new(db: &'a Database) -> Self {
        TreeDiff {
            changes: HashMap::new(),
            db,
        }
    }

    pub fn compare_oids<P: AsRef<Path>>(
        &mut self,
        a: &Option<String>,
        b: &Option<String>,
        prefix: Option<P>,
    ) {
        if a == b {
            return;
        }
        let tree_a = if let Ok(tree) = self.oid_to_tree(a) {
            tree
        } else {
            Tree::new()
        };

        let tree_b = if let Ok(tree) = self.oid_to_tree(b) {
            tree
        } else {
            Tree::new()
        };

        let prefix = if let Some(prefix) = prefix {
            prefix.as_ref().to_path_buf()
        } else {
            PathBuf::new()
        };

        self.detect_deletions(tree_a.clone(), tree_b.clone(), prefix.clone());
        self.detect_additions(tree_a, tree_b, prefix);
    }

    fn detect_deletions(&mut self, a: Tree, b: Tree, prefix: PathBuf) {
        for (name, entry) in a.entries {
            let other = b.get_entry(name.as_ref()).cloned();

            let a_oid = TreeDiff::get_tree_oid(&entry);
            let b_oid = if let Some(other) = other.clone() {
                if &entry == &other {
                    continue;
                }
                TreeDiff::get_tree_oid(&other)
            } else {
                None
            };
            let path = prefix.join(&name);
            self.compare_oids(&a_oid, &b_oid, Some(path.clone()));

            let changes = if a_oid.is_none() && b_oid.is_none() {
                (Some(entry), other)
            } else if a_oid.is_none() {
                (Some(entry), None)
            } else if b_oid.is_none() {
                (None, other)
            } else {
                continue;
            };
            self.changes.insert(path, changes);
        }
    }

    fn detect_additions(&mut self, a: Tree, b: Tree, prefix: PathBuf) {
        for (name, entry) in b.entries {
            let other = a.get_entry(name.as_ref()).cloned();
            if other.is_some() {
                continue;
            }

            let path = prefix.join(&name);
            if entry.is_tree() {
                let oid = TreeDiff::get_tree_oid(&entry);
                self.compare_oids(&None, &oid, Some(path.clone()));
            } else {
                self.changes.insert(path, (None, Some(entry)));
            }
        }
    }

    fn get_tree_oid(t: &TreeEntry) -> Option<String> {
        match t {
            TreeEntry::Tree(t) => Some(t.oid()),
            TreeEntry::Entry(_) => None,
            TreeEntry::Marker(m) => {
                if m.is_tree() {
                    Some(m.clone().oid)
                } else {
                    None
                }
            }
        }
    }

    fn oid_to_tree(&self, oid: &Option<String>) -> Result<Tree, Error> {
        if let Some(oid) = oid {
            let (kind, _, data) = self.db.read_object(oid.as_ref())?;
            match kind {
                ObjectKind::Commit => {
                    let c = Commit::try_from(data)?;
                    self.oid_to_tree(&Some(c.tree))
                }
                ObjectKind::Tree => Tree::try_from(data),
                _ => unreachable!(),
            }
        } else {
            Err(format_err!("no oid sent"))
        }
    }
}
