use crate::database::Database;
use crate::database::{Blob, Storable};
use crate::repository::migration::{Action, Migration, MigrationChanges};
use crate::tree::TreeEntry;
use failure::Error;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::{File, Metadata, OpenOptions, Permissions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const IGNORED: [&str; 6] = [".", "..", ".git", "target", ".idea", "cmake-build-debug"];

#[derive(Clone, Debug)]
pub struct Workspace {
    pub path: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Workspace {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn list_dir(&self, path: Option<PathBuf>) -> io::Result<BTreeMap<PathBuf, Metadata>> {
        let path = match path {
            Some(ref p) => p,
            None => &self.path,
        };

        let mut stats = BTreeMap::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?.path();

            let p = if entry.starts_with(".") {
                entry.strip_prefix("./").unwrap()
            } else {
                &entry
            };

            if IGNORED.iter().any(|&x| p.starts_with(x)) {
                continue;
            }
            let stat = std::fs::metadata(&p)?;
            stats.insert(p.to_path_buf(), stat);
        }

        Ok(stats)
    }

    pub fn list_files(&self, path: Option<PathBuf>) -> io::Result<Vec<PathBuf>> {
        let path = match path {
            Some(ref p) => p,
            None => &self.path,
        };

        if !path.exists() {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        if path.is_dir() {
            visit_dirs(&path)
        } else {
            Ok(vec![path.to_path_buf()])
        }
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, Error> {
        let path = self.workspace_path(path);
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn create_dir(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let path = self.workspace_path(path);

        if path.metadata()?.is_file() {
            std::fs::remove_dir(&path)?;
        }
        std::fs::create_dir(&path)
    }

    fn remove_dir(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let path = self.workspace_path(path);
        std::fs::remove_dir(path)
    }

    fn workspace_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let path = path.as_ref().to_path_buf();
        if path.is_relative() {
            self.path.join(path)
        } else {
            path
        }
    }
    pub fn apply_migration(&self, migration: Migration, db: &Database) -> Result<(), Error> {
        self.apply_change_list(&migration.changes, Action::Remove, db)?;
        let mut remove = migration.rmdirs;
        let mut make = migration.mkdirs;
        remove.sort();
        for r in remove.iter().rev() {
            self.remove_dir(r)?;
        }
        make.sort();
        for m in make {
            self.create_dir(&m)?;
        }
        self.apply_change_list(&migration.changes, Action::Create, db)?;
        self.apply_change_list(&migration.changes, Action::Update, db)?;

        Ok(())
    }

    fn apply_change_list(
        &self,
        changes: &MigrationChanges,
        action: Action,
        db: &Database,
    ) -> Result<(), Error> {
        let list = match changes.get(&action) {
            None => return Ok(()),
            Some(l) => l,
        };
        for (path, entry) in list {
            let path = self.workspace_path(&path);
            std::fs::remove_file(&path)?;
            if action == Action::Remove {
                continue;
            }

            let entry = entry.clone().unwrap();
            let (oid, mode) = match entry {
                TreeEntry::Entry(e) => (e.oid().to_owned(), e.mode()),
                TreeEntry::Tree(t) => (t.oid(), t.mode()),
                TreeEntry::Marker(m) => (m.oid, m.mode),
            };
            let (_, _, data) = db.read_object(oid.as_str())?;
            let blob = Blob::try_from(data)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(blob.data.as_bytes())?;
            let mode = mode.parse::<u32>()?;
            let perms = Permissions::from_mode(mode);
            file.set_permissions(perms)?;
        }
        Ok(())
    }
}

fn visit_dirs(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?.path();

        let p = if entry.starts_with(".") {
            entry.strip_prefix("./").unwrap()
        } else {
            &entry
        };

        if IGNORED.iter().any(|&x| p.starts_with(x)) {
            continue;
        }
        if p.is_dir() {
            let mut sub = visit_dirs(&p)?;
            entries.append(&mut sub);
        } else {
            entries.push(p.to_path_buf())
        }
    }
    Ok(entries)
}
