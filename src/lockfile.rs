use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::format_err;
use failure::Error;

#[derive(Debug, Default)]
pub struct Lockfile {
    pub path: PathBuf,
    lock: PathBuf,
    file: RefCell<Option<File>>,
}

impl Lockfile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
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
            .open(&self.lock).expect("Failed to get lock file");
        self.file = RefCell::new(Some(file));
        Ok(self)
    }

    pub fn write_all(&self, content: &[u8]) -> Result<&Self, Error> {
        if let Some(ref mut file) = *self.file.borrow_mut() {
            file.write_all(content)?;
        } else {
            return Err(format_err!(
                "Unable to get reference to file; did you already lock it?"
            ));
        }
        Ok(self)
    }

    pub fn release(self) -> Result<(), Error> {
        let file = self.file.into_inner().unwrap();
        drop(file);
        std::fs::remove_file(self.lock)?;
        Ok(())
    }

    pub fn commit(self) -> Result<(), Error> {
        let file = self.file.into_inner().unwrap();
        drop(file);
        std::fs::rename(self.lock, self.path)?;
        Ok(())
    }
}
