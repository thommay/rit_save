use crate::index::entry::Entry;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use fs2::FileExt;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub mod entry;

#[derive(Debug, Clone)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
    index: PathBuf,
    changed: bool,
}

impl Index {
    pub fn new(index: PathBuf) -> Self {
        Index {
            entries: BTreeMap::new(),
            index,
            changed: false,
        }
    }

    pub fn from(index: PathBuf) -> Result<Self, Error> {
        let mut index = Index::new(index);
        index.load()?;
        Ok(index)
    }

    pub fn add(&mut self, path: &Path, oid: &str, stat: std::fs::Metadata) {
        let entry = Entry::new(path, stat, oid);
        self.entries.insert(path.to_str().unwrap().into(), entry);
        self.changed = true;
    }

    pub fn write_updates(&mut self) -> Result<(), Error> {
        if !self.changed {
            return Ok(());
        }
        let mut index = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.index)?;
        index.try_lock_exclusive()?;

        let mut digest = Sha1::new();
        let mut header = Vec::new();
        write!(&mut header, "DIRC")?;
        header.write_u32::<BigEndian>(2u32)?;
        header.write_u32::<BigEndian>(self.entries.len() as u32)?;
        self.write(&mut index, &mut digest, header)?;

        for (_name, entry) in &self.entries {
            self.write(&mut index, &mut digest, entry.pack()?)?;
        }
        index.write(&digest.digest().bytes())?;
        self.changed = false;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), Error> {
        let index = OpenOptions::new().read(true).open(&self.index);

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
        self.changed = false;
    }

    fn read(&self, index: &mut File, digest: &mut Sha1, data: &mut [u8]) -> Result<usize, Error> {
        let res = index.read(data)?;
        digest.update(data);
        Ok(res)
    }

    fn write(&self, index: &mut File, digest: &mut Sha1, data: Vec<u8>) -> Result<(), Error> {
        index.write(data.as_slice())?;
        digest.update(&data);
        Ok(())
    }
}

impl From<Index> for Vec<Entry> {
    fn from(index: Index) -> Self {
        index.entries.values().cloned().collect()
    }
}
