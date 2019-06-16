use crate::index::entry::Entry;
use crate::lockfile::Lockfile;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use fs2::FileExt;
use sha1::Sha1;
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub mod entry;

#[derive(Debug)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
    parents: HashMap<String, Vec<PathBuf>>,
    changed: bool,
    lock: Lockfile,
}

impl Index {
    pub fn new<P: AsRef<Path>>(index: P) -> Result<Self, Error> {
        let lock = Lockfile::new(index)?.try_lock()?;
        Ok(Index {
            entries: BTreeMap::new(),
            parents: HashMap::new(),
            changed: false,
            lock,
        })
    }

    pub fn from<P: AsRef<Path>>(index: P) -> Result<Self, Error> {
        let mut index = Index::new(index)?;
        index.load()?;
        Ok(index)
    }

    pub fn add<P: AsRef<Path> + Copy>(&mut self, path: P, oid: &str, stat: std::fs::Metadata) {
        let entry = Entry::new(path, stat, oid);

        let pth = path.as_ref().to_str().unwrap().to_owned();

        self.discard_conflicts(&entry);

        for dir in entry.parent_directories() {
            let dir = dir.to_str().unwrap().to_string();
            self.parents
                .entry(dir)
                .and_modify(|e| e.push(entry.path.clone()))
                .or_insert(vec![entry.path.clone()]);
        }

        self.entries.insert(pth, entry);
        self.changed = true;
    }

    fn discard_conflicts(&mut self, entry: &Entry) {
        for dir in entry.parent_directories() {
            let key = dir.as_os_str().to_str().unwrap();
            self.remove_entry(key);
        }
        if let Some(children) = self.parents.clone().get(entry.path.to_str().unwrap()) {
            for child in children {
                let key = child.as_os_str().to_str().unwrap();
                self.remove_entry(key);
            }
        }
    }

    fn remove_entry(&mut self, key: &str) {
        if let Some(entry) = self.entries.get(key) {
            for dir in entry.parent_directories() {
                self.parents
                    .entry(dir.to_str().unwrap().into())
                    .and_modify(|f| {
                        if let Ok(index) = f.binary_search(&PathBuf::from(key)) {
                            f.remove(index);
                        }
                    });
            }
        } else {
            return;
        }
        self.entries.remove(key);
    }

    pub fn write_updates(mut self) -> Result<(), Error> {
        if !self.changed {
            return Ok(());
        }

        let mut digest = Sha1::new();
        let mut header = Vec::new();
        write!(&mut header, "DIRC")?;
        header.write_u32::<BigEndian>(2u32)?;
        header.write_u32::<BigEndian>(self.entries.len() as u32)?;
        self.write(&mut digest, header)?;

        for entry in self.entries.values() {
            self.write(&mut digest, entry.pack()?)?;
        }
        self.lock.write_all(&digest.digest().bytes())?;
        self.changed = false;
        self.lock.commit()?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), Error> {
        let index = OpenOptions::new().read(true).open(&self.lock.path);

        let mut index = match index {
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
            Ok(f) => f,
        };
        index.lock_shared()?;

        self.clear();

        let mut digest = Sha1::new();
        let mut header = [0; 12];
        self.read(&mut index, &mut digest, &mut header)?;
        let count = self.parse_header(&mut header)?;

        for _x in 0..count {
            let mut entry = [0; 64];
            self.read(&mut index, &mut digest, &mut entry)?;
            let mut entry = entry.to_vec();
            while entry.last().unwrap() != &0u8 {
                let mut ex = [0; 8];
                self.read(&mut index, &mut digest, &mut ex)?;
                entry.extend_from_slice(&ex);
            }
            let e = Entry::from(&mut entry)?;
            self.entries.insert(e.path.to_str().unwrap().into(), e);
        }

        let mut csum = Vec::new();
        index.read_to_end(&mut csum)?;
        assert_eq!(digest.digest().bytes(), csum.as_slice());
        Ok(())
    }

    fn parse_header(&self, header: &mut [u8]) -> Result<u32, Error> {
        let mut header = std::io::Cursor::new(header);
        let mut sig = [0; 4];
        header.read_exact(&mut sig)?;
        let sig = std::str::from_utf8(&sig)?;
        assert_eq!(sig, "DIRC");
        let version = header.read_u32::<BigEndian>()?;
        assert_eq!(version, 2u32);
        header.read_u32::<BigEndian>().map_err(|e| e.into())
    }

    fn clear(&mut self) {
        self.entries = BTreeMap::new();
        self.parents = HashMap::new();
        self.changed = false;
    }

    fn read(&self, index: &mut File, digest: &mut Sha1, data: &mut [u8]) -> Result<usize, Error> {
        let res = index.read(data)?;
        digest.update(data);
        Ok(res)
    }

    fn write(&self, digest: &mut Sha1, data: Vec<u8>) -> Result<(), Error> {
        self.lock.write_all(data.as_slice())?;
        digest.update(&data);
        Ok(())
    }
}

impl From<Index> for Vec<Entry> {
    fn from(index: Index) -> Self {
        index.entries.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use lazy_static::lazy_static;
    use std::path::PathBuf;

    lazy_static! {
        static ref TEST_ROOT: PathBuf = {
            let mut path = std::env::current_exe().expect("couldn't read executable name");
            path.pop(); // remove the executable name
            path.pop(); // remove `debug`
            path.pop();

            path
        };

        static ref FILE_STAT: std::fs::Metadata = {
            std::fs::metadata(std::env::current_exe().expect("couldn't read executable name")).unwrap()
        };


        static ref INDEX: PathBuf = {
            TEST_ROOT.to_path_buf().join("index")
        };

        static ref LOCK: PathBuf = {
            TEST_ROOT.to_path_buf().join("index.lock")
        };

        static ref OID: String = {
            sha1::Sha1::from("my test string").hexdigest()
        };
    }

    #[test]
    fn test_add() {
        let mut index = Index::new(INDEX.to_path_buf()).unwrap();
        index.add("alice.txt", &*OID, FILE_STAT.clone());
        assert_eq!(index.entries.len(), 1);
        let mut entries = index.entries.values().map(|x| x.filename());
        std::fs::remove_file(LOCK.to_path_buf()).unwrap();
        assert_eq!(entries.next(), Some("alice.txt"))
    }

    #[test]
    fn test_replace_file_with_dir() {
        let mut index = Index::new(INDEX.to_path_buf()).unwrap();
        index.add("alice.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt", &*OID, FILE_STAT.clone());
        assert_eq!(index.entries.len(), 2);
        index.add("alice.txt/nested.txt", &*OID, FILE_STAT.clone());
        let entry_paths: Vec<Option<&str>> =
            index.entries.values().map(|x| x.path.to_str()).collect();
        std::fs::remove_file(LOCK.to_path_buf()).unwrap();
        assert_eq!(
            vec![Some("alice.txt/nested.txt"), Some("bob.txt")],
            entry_paths
        )
    }

    #[test]
    fn test_replace_deep_file_with_dir() {
        let mut index = Index::new(INDEX.to_path_buf()).unwrap();
        index.add("alice.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt/deep", &*OID, FILE_STAT.clone());
        assert_eq!(index.entries.len(), 2);
        index.add("alice.txt/nested.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt/deep/nested.txt", &*OID, FILE_STAT.clone());
        let entry_paths: Vec<Option<&str>> =
            index.entries.values().map(|x| x.path.to_str()).collect();
        std::fs::remove_file(LOCK.to_path_buf()).unwrap();
        assert_eq!(
            vec![
                Some("alice.txt/nested.txt"),
                Some("bob.txt/deep/nested.txt")
            ],
            entry_paths
        )
    }

    #[test]
    fn test_replace_dir_with_file() {
        let mut index = Index::new(INDEX.to_path_buf()).unwrap();
        index.add("alice.txt/nested.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt", &*OID, FILE_STAT.clone());
        assert_eq!(index.entries.len(), 2);
        index.add("alice.txt", &*OID, FILE_STAT.clone());
        let entry_paths: Vec<Option<&str>> =
            index.entries.values().map(|x| x.path.to_str()).collect();
        std::fs::remove_file(LOCK.to_path_buf()).unwrap();
        assert_eq!(vec![Some("alice.txt"), Some("bob.txt")], entry_paths)
    }

    #[test]
    fn test_replace_dir_with_file_recursively() {
        let mut index = Index::new(INDEX.to_path_buf()).unwrap();
        index.add("alice.txt/deep/nested.txt", &*OID, FILE_STAT.clone());
        index.add("bob.txt", &*OID, FILE_STAT.clone());
        assert_eq!(index.entries.len(), 2);
        index.add("alice.txt", &*OID, FILE_STAT.clone());
        let entry_paths: Vec<Option<&str>> =
            index.entries.values().map(|x| x.path.to_str()).collect();
        std::fs::remove_file(LOCK.to_path_buf()).unwrap();
        assert_eq!(vec![Some("alice.txt"), Some("bob.txt")], entry_paths)
    }

}
