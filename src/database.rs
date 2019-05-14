use std::path::PathBuf;
use failure::Error;
use std::fs::OpenOptions;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

#[derive(Default,Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn store<T>(&self, blob: T) -> Result<(), Error> where T: Storable {
        let content = blob.serialize();
        let oid = blob.oid();
        self.write(oid, content)
    }

    fn write(&self, oid: String, content: Vec<u8>) -> Result<(), Error> {
        let oid = oid.as_bytes();
        let (shard, filename) = oid.split_at(2);
        let dir = self.path.join(std::str::from_utf8(shard)?);
        let path = dir.join(std::str::from_utf8(filename)?);
        if path.exists() {
            return Ok(());
        }

        let tmpnam = dir.join(uuid::Uuid::new_v4().to_simple().to_string());
        std::fs::create_dir_all(dir)?;

        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmpnam)?;

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&content)?;
        file.write(e.finish()?.as_ref())?;

        drop(file);
        std::fs::rename(tmpnam,path)?;

        Ok(())
    }
}

pub trait Storable {
    fn serialize(&self) -> Vec<u8>;
    fn oid(&self) -> String { sha1::Sha1::from( &self.serialize()).hexdigest() }
}

#[derive(Clone,Debug,Default)]
pub struct Blob {
    data: String,
}

impl Blob {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    fn content(&self) -> String {self.data.to_string()}
}

impl Storable for Blob {
    fn serialize(&self) -> Vec<u8> {
        let s = self.content();
        format!("blob {}\0{}", s.len(), s).into()
    }
}