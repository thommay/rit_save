use failure::Error;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::fs::OpenOptions;
use std::io::{Write, BufReader, Read, BufRead};
use std::path::{Path, PathBuf};
use flate2::bufread::ZlibDecoder;
use failure::format_err;

pub mod marker;

#[derive(Default, Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn store<T>(&self, blob: T) -> Result<(), Error>
    where
        T: Storable,
    {
        let content = blob.serialize();
        let oid = blob.oid();
        self.write(oid, content)
    }

    fn write(&self, oid: String, content: Vec<u8>) -> Result<(), Error> {
        let (dir, path) = self.object_path(oid.as_ref())?;
        if path.exists() {
            return Ok(());
        }

        let tmpnam = dir.join(uuid::Uuid::new_v4().to_simple().to_string());
        std::fs::create_dir_all(dir)?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmpnam)?;

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&content)?;
        file.write_all(e.finish()?.as_ref())?;

        drop(file);
        std::fs::rename(tmpnam, path)?;

        Ok(())
    }

    fn object_path(&self, oid: &str) -> Result<(PathBuf, PathBuf), Error> {
        let oid = oid.as_bytes();
        let (shard, filename) = oid.split_at(2);
        let dir = self.path.join(std::str::from_utf8(shard)?);
        let path = dir.join(std::str::from_utf8(filename)?);
        Ok((dir, path))
    }

    pub fn read_object(&self, oid: &str) -> Result<(String, u64, Vec<u8>), Error> {
        let (_, path) = self.object_path(oid)?;
        if !path.exists() {
            return Err(format_err!("object {} does not exist", oid));
        }
        let mut out = Vec::new();
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(file);
        let mut z = ZlibDecoder::new(reader);
        z.read_to_end(&mut out)?;
        let mut cursor = std::io::Cursor::new(out);

        let mut tp = vec![];
        cursor.read_until(b' ', &mut tp)?;
        let tp = String::from_utf8(tp)?;
        let tp = tp.trim_end_matches(' ');

        let mut size = vec![];
        cursor.read_until(b'\0', &mut size)?;
        let size = std::str::from_utf8(size.as_ref())?.trim_end_matches('\0').parse::<u64>()?;

        let mut out = vec![];
        cursor.read_to_end(&mut out)?;

        Ok((tp.to_string(), size, out))
    }
}

pub trait Storable {
    fn serialize(&self) -> Vec<u8>;
    fn oid(&self) -> String {
        sha1::Sha1::from(&self.serialize()).hexdigest()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Blob {
    data: String,
}

impl Blob {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    fn content(&self) -> String {
        self.data.to_string()
    }
}

impl Storable for Blob {
    fn serialize(&self) -> Vec<u8> {
        let s = self.content();
        format!("blob {}\0{}", s.len(), s).into()
    }
}
