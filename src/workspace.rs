use failure::Error;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

const IGNORED: [&str; 6] = [".", "..", ".git", "target", ".idea", "cmake-build-debug"];

pub struct Workspace {
    pub path: PathBuf,
}

impl Workspace {
    pub fn new(path: PathBuf) -> Self {
        Workspace { path }
    }

    pub fn list_files(&self, path: Option<PathBuf>) -> io::Result<Vec<PathBuf>> {
        let path = match path {
            Some(ref p) => p,
            None => &self.path,
        };

        if path.is_dir() {
            visit_dirs(&path)
        } else {
            Ok(vec![path.to_path_buf()])
        }
    }

    pub fn read_file(&self, path: &Path) -> Result<String, Error> {
        let path = self.path.join(path);
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
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

        if IGNORED.into_iter().any(|&x| p.starts_with(x)) {
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
