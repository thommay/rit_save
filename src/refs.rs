use crate::lockfile::Lockfile;
use failure::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;

pub struct Refs {
    path: PathBuf,
}

impl Refs {
    pub fn new(path: PathBuf) -> Self {
        Refs { path }
    }

    pub fn get_head(&self) -> Option<String> {
        if let Ok(mut fh) = OpenOptions::new().read(true).open(self.head_path()) {
            let mut ret = String::new();
            fh.read_to_string(&mut ret).unwrap();
            Some(ret)
        } else {
            None
        }
    }

    pub fn update_head(&self, oid: &str) -> Result<(), Error> {
        Lockfile::new(&self.head_path())?
            .try_lock()?
            .write(oid)?
            .write("\n")?
            .commit()
    }

    fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }
}
