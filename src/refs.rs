use crate::lockfile::Lockfile;
use failure::format_err;
use failure::Error;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Refs {
    path: PathBuf,
}

pub enum BranchName {
    Ok,
    InvalidName,
    AlreadyExists,
}

impl Refs {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Refs {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn get_head(&self) -> Option<String> {
        if let Ok(mut fh) = OpenOptions::new().read(true).open(self.head_path()) {
            let mut ret = String::new();
            fh.read_to_string(&mut ret).unwrap();
            let ret = ret.trim_end_matches('\n').to_owned();
            Some(ret)
        } else {
            None
        }
    }

    pub fn update_head(&self, oid: &str) -> Result<(), Error> {
        self.update_ref_file(self.path.join("HEAD"), oid)
    }

    pub fn update_ref_file<P: AsRef<Path>>(&self, path: P, oid: &str) -> Result<(), Error> {
        let lock = Lockfile::new(path)?.try_lock()?;
        lock.write_all(oid.as_bytes())?;
        lock.write_all("\n".as_bytes())?;
        lock.commit()
    }

    pub fn create_branch(&self, name: &str, start: Option<String>) -> Result<(), Error> {
        match self.validate_branch_name(name)? {
            BranchName::InvalidName => {
                return Err(format_err!("'{}' is not a valid branch name.", name))
            }
            BranchName::AlreadyExists => {
                return Err(format_err!("A branch named '{}' already exists.", name))
            }
            _ => {}
        }
        if let Some(head) = start {
            std::fs::create_dir_all(self.heads_path())?;
            let path = self.heads_path().join(name);
            self.update_ref_file(path, &head)?;
            Ok(())
        } else {
            Err(format_err!(
                "failed to get reference for HEAD to branch off"
            ))
        }
    }

    pub fn read_ref(&self, name: &str) -> Option<String> {
        if let Some(path) = self.path_for_name(name) {
            return self.read_ref_file(path);
        }
        None
    }

    fn path_for_name(&self, name: &str) -> Option<PathBuf> {
        let refs = &self.refs_path();
        let heads = &self.heads_path();
        let prefixes = vec![&self.path, refs, heads];
        prefixes
            .iter()
            .find(|&p| {
                let pth = p.join(name);
                std::fs::metadata(pth).map(|m| m.is_file()).unwrap_or(false)
            })
            .map(|p| p.join(name))
    }

    fn read_ref_file(&self, path: PathBuf) -> Option<String> {
        let mut cnt = String::new();
        if let Ok(mut fh) = File::open(path) {
            fh.read_to_string(&mut cnt)
                .expect("fatal: Could not read reference");
            let cnt = cnt.trim().to_owned();
            return Some(cnt);
        }
        None
    }

    fn validate_branch_name(&self, name: &str) -> Result<BranchName, Error> {
        if crate::revision::INVALID_NAME.is_match(name) {
            return Ok(BranchName::InvalidName);
        }

        if let Ok(stat) = std::fs::metadata(self.heads_path().join(name)) {
            if stat.is_file() {
                return Ok(BranchName::AlreadyExists);
            }
        }
        Ok(BranchName::Ok)
    }

    fn refs_path(&self) -> PathBuf {
        self.path.join("refs")
    }

    fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }

    fn heads_path(&self) -> PathBuf {
        self.refs_path().join("heads")
    }
}
