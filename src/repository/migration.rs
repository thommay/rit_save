use crate::database::tree_diff::TreeDifference;
use crate::tree::TreeEntry;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    Create,
    Remove,
    Update,
}

pub type MigrationChanges = HashMap<Action, Vec<(PathBuf, Option<TreeEntry>)>>;

#[derive(Clone, Debug)]
pub struct Migration {
    diff: TreeDifference,
    pub(crate) rmdirs: Vec<PathBuf>,
    pub(crate) mkdirs: Vec<PathBuf>,
    pub(crate) changes: MigrationChanges,
}

impl Migration {
    pub fn new(diff: TreeDifference) -> Migration {
        let changes = [
            (Action::Create, Vec::new()),
            (Action::Remove, Vec::new()),
            (Action::Update, Vec::new()),
        ]
        .iter()
        .cloned()
        .collect();

        Migration {
            diff,
            rmdirs: Vec::new(),
            mkdirs: Vec::new(),
            changes,
        }
    }

    pub fn plan_changes(mut self) -> Migration {
        for (path, (old, new)) in self.diff.clone() {
            let ancestors = path
                .parent()
                .unwrap()
                .ancestors()
                .map(|p| p.to_path_buf())
                .collect::<Vec<_>>();

            let action = if old.is_none() {
                self.mkdirs.extend(ancestors);
                Action::Create
            } else if new.is_none() {
                self.rmdirs.extend(ancestors);
                Action::Remove
            } else {
                self.mkdirs.extend(ancestors);
                Action::Update
            };
            self.changes
                .entry(action)
                .and_modify(|e| e.push((path, new)));
        }
        self
    }
}
