use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::format_err;
use failure::Error;

#[derive(Debug, Default)]
pub struct Lockfile {
    path: PathBuf,
    lock: PathBuf,
    file: RefCell<Option<File>>,
}

impl Lockfile {
    pub fn new(path: &Path) -> Result<Self, Error> {
        let path = path.to_path_buf();
        if let Some(name) = path.file_name() {
            let mut name = name.to_os_string();
            name.push(".lock");
            let lock = path.with_file_name(name);
            Ok(Self {
                path,
                lock,
                file: RefCell::new(None),
            })
        } else {
            Err(format_err!("Path did not have a file name!"))
        }
    }

    pub fn try_lock(mut self) -> Result<Self, Error> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock)?;
        self.file = RefCell::new(Some(file));
        Ok(self)
    }

    pub fn write(self, content: &str) -> Result<Self, Error> {
        if let Some(ref mut file) = *self.file.borrow_mut() {
            file.write(content.as_bytes())?;
        } else {
            return Err(format_err!(
                "Unable to get reference to file; did you already lock it?"
            ));
        }
        Ok(self)
    }

    pub fn commit(self) -> Result<(), Error> {
        let file = self.file.into_inner().unwrap();
        drop(file);
        std::fs::rename(self.lock, self.path)?;
        Ok(())
    }
}
