use crate::utilities::pack_data;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Entry {
    executable: bool,
    pub name: PathBuf,
    oid: String,
}

impl Entry {
    pub fn new(name: PathBuf, oid: String, executable: bool) -> Self {
        Entry {
            executable,
            name,
            oid,
        }
    }

    pub fn mode(&self) -> String {
        if self.executable {
            "100755".into()
        } else {
            "100644".into()
        }
    }

    pub fn filename(&self) -> &str {
        self.name.file_name().unwrap().to_str().unwrap()
    }

    pub fn metadata(&self) -> Vec<u8> {
        let mode = self.mode();
        let n = self.filename();
        pack_data(mode.as_ref(), n, self.oid.as_ref()).unwrap()
    }
}
