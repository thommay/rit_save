use crate::utilities::{decode_hex, is_executable, pack_data};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use std::cmp::{Ord, Ordering};
use std::fs::Metadata;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
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

    pub fn mode(&self) -> String {
        if is_executable(self.mode) {
            "100755".into()
        } else {
            "100644".into()
        }
    }

    pub fn filename(&self) -> &str {
        self.path.file_name().unwrap().to_str().unwrap()
    }

    pub fn metadata(&self) -> Vec<u8> {
        let mode = self.mode();
        let n = self.filename();
        pack_data(mode.as_ref(), n, self.oid.as_ref()).unwrap()
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
