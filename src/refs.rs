use crate::lockfile::Lockfile;
use failure::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct Refs {
    path: PathBuf,
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
        let lock = Lockfile::new(&self.head_path())?.try_lock()?;
        lock.write_all(oid.as_bytes())?;
        lock.write_all("\n".as_bytes())?;
        lock.commit()
    }

    fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }
}
