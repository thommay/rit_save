use crate::utilities::decode_hex;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use fs2::FileExt;
use sha1::Sha1;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub struct Index {
    entries: BTreeMap<String, Entry>,
    index: PathBuf,
    changed: bool,
}

impl Index {
    pub fn new(index: PathBuf) -> Result<Self, Error> {
        Ok(Index {
            entries: BTreeMap::new(),
            index,
            changed: false,
        })
    }

    pub fn add(&mut self, path: &Path, oid: &str, stat: std::fs::Metadata) {
        let entry = Entry::new(path, stat, oid);
        self.entries.insert(path.to_str().unwrap().into(), entry);
        self.changed = true;
    }

    pub fn write_updates(&mut self) -> Result<(), Error> {
        if !self.changed { return Ok(()) }
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

    pub fn load_for_update(&mut self) -> Result<(), Error> {
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

#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    oid: String,
    flags: u16,
    ctime: u32,
    ctime_ns: u32,
    mtime: u32,
    mtime_ns: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
}

impl Entry {
    pub fn new(path: &Path, stat: Metadata, oid: &str) -> Self {
        let path = path.to_path_buf();
        let pathlength = path.to_str().unwrap().len();
        let flags: u16 = if pathlength > 0xFFF {
            0xFFF
        } else {
            pathlength as u16
        };
        let oid = String::from(oid);
        let ctime: u32 = stat.ctime() as u32;
        let ctime_ns: u32 = stat.ctime_nsec() as u32;
        let mtime: u32 = stat.mtime() as u32;
        let mtime_ns: u32 = stat.mtime_nsec() as u32;
        let dev: u32 = stat.dev() as u32;
        let ino: u32 = stat.ino() as u32;
        let mode: u32 = stat.mode() as u32;
        let uid: u32 = stat.uid() as u32;
        let gid: u32 = stat.gid() as u32;
        let size: u32 = stat.size() as u32;

        Entry {
            path,
            oid,
            flags,
            ctime,
            ctime_ns,
            mtime,
            mtime_ns,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
        }
    }

    pub fn from(entry: &mut Vec<u8>) -> Result<Self, Error> {
        let mut entry = std::io::Cursor::new(entry);
        let ctime = entry.read_u32::<BigEndian>()?;
        let ctime_ns = entry.read_u32::<BigEndian>()?;
        let mtime = entry.read_u32::<BigEndian>()?;
        let mtime_ns = entry.read_u32::<BigEndian>()?;
        let dev = entry.read_u32::<BigEndian>()?;
        let ino = entry.read_u32::<BigEndian>()?;
        let mode = entry.read_u32::<BigEndian>()?;
        let uid = entry.read_u32::<BigEndian>()?;
        let gid = entry.read_u32::<BigEndian>()?;
        let size = entry.read_u32::<BigEndian>()?;
        let mut oid = [0; 20];
        entry.read_exact(&mut oid)?;
        let oid = hex::encode(oid);
        let flags = entry.read_u16::<BigEndian>()?;
        let mut path = String::new();
        entry.read_to_string(&mut path)?;
        let path = path.trim_end_matches('\0').into();
        Ok(Entry {
            path,
            oid,
            flags,
            ctime,
            ctime_ns,
            mtime,
            mtime_ns,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
        })
    }

    pub fn pack(&self) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();
        data.write_u32::<BigEndian>(self.ctime)?;
        data.write_u32::<BigEndian>(self.ctime_ns)?;
        data.write_u32::<BigEndian>(self.mtime)?;
        data.write_u32::<BigEndian>(self.mtime_ns)?;
        data.write_u32::<BigEndian>(self.dev)?;
        data.write_u32::<BigEndian>(self.ino)?;
        data.write_u32::<BigEndian>(self.mode)?;
        data.write_u32::<BigEndian>(self.uid)?;
        data.write_u32::<BigEndian>(self.gid)?;
        data.write_u32::<BigEndian>(self.size)?;
        let b = decode_hex(self.oid.as_ref())?;
        for s in b {
            data.write_u8(s)?;
        }
        data.write_u16::<BigEndian>(self.flags as u16)?;
        write!(&mut data, "{}\0", self.path.to_str().unwrap())?;
        while &data.len() % 8 != 0 {
            write!(&mut data, "\0")?;
        }
        Ok(data)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.path == other.path
    }
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
