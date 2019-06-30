use crate::utilities::pack_data;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Marker {
    pub name: PathBuf,
    pub mode: String,
    pub oid: String,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Entry,
    Tree,
}

impl Marker {
    pub fn new<N: AsRef<Path>, O: Into<String>, M: Into<String>>(name: N, oid: O, mode: M) -> Self {
        Self {
            name: name.as_ref().to_path_buf(),
            oid: oid.into(),
            mode: mode.into(),
        }
    }

    pub fn kind(&self) -> Kind {
        if &self.mode == "40000" {
            Kind::Tree
        } else {
            Kind::Entry
        }
    }

    pub fn filename(&self) -> &str {
        self.name.file_name().unwrap().to_str().unwrap()
    }

    pub fn metadata(&self) -> Vec<u8> {
        let mode = self.mode.as_ref();
        let name = self.filename();
        pack_data(mode, name, self.oid.as_ref()).expect("failed to pack marker")
    }
}
