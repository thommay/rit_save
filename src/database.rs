use crate::database::tree_diff::{TreeDiff, TreeDifference};
use failure::format_err;
use failure::Error;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::convert::TryFrom;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

pub mod marker;
pub mod tree_diff;

//macro_rules! parsed_kind {
//    ($knd:ty: $($k:ty => $s:ident),+) => {
//        #[derive(Clone, Debug)]
//        pub enum $knd {}
//    };
//}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectKind {
    Commit,
    Tree,
    Blob,
}

impl std::fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match *self {
                ObjectKind::Commit => "commit",
                ObjectKind::Tree => "tree",
                ObjectKind::Blob => "blob",
            }
        )
    }
}

impl ObjectKind {
    pub fn parse(k: &str) -> Self {
        match k {
            "commit" => ObjectKind::Commit,
            "tree" => ObjectKind::Tree,
            _ => ObjectKind::Blob,
        }
    }

    pub fn is_commit(&self) -> bool {
        match *self {
            ObjectKind::Commit => true,
            _ => false,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn read_object(&self, oid: &str) -> Result<(ObjectKind, u64, Vec<u8>), Error> {
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
        let kind = ObjectKind::parse(tp);

        let mut size = vec![];
        cursor.read_until(b'\0', &mut size)?;
        let size = std::str::from_utf8(size.as_ref())?
            .trim_end_matches('\0')
            .parse::<u64>()?;

        let mut out = vec![];
        cursor.read_to_end(&mut out)?;

        Ok((kind, size, out))
    }

    pub fn store<T>(&self, blob: T) -> Result<(), Error>
    where
        T: Storable,
    {
        let content = blob.serialize();
        let oid = blob.oid();
        self.write(oid, content)
    }

    pub fn truncate_oid(&self, oid: &str) -> String {
        oid.get(0..=6)
            .map(String::from)
            .unwrap_or_else(|| String::from(oid))
    }

    pub fn prefix_match(&self, name: &str) -> Result<Vec<String>, Error> {
        if let Ok((dir, _)) = self.object_path(name) {
            let prefix = &dir.file_name().unwrap().to_str().unwrap();
            let entries = std::fs::read_dir(&dir)?
                .map(|f| {
                    let entry = f.unwrap();
                    let n = entry.file_name();
                    let n = n.to_str().expect("failed to get name");
                    format!("{}{}", prefix, n)
                })
                .collect::<Vec<String>>();

            let set = entries
                .iter()
                .filter(|&e| e.starts_with(name))
                .map(String::from)
                .collect::<Vec<String>>();
            return Ok(set);
        }
        Ok(vec![])
    }

    pub fn tree_diff(&self, a: Option<String>, b: Option<String>) -> TreeDifference {
        let mut td = TreeDiff::new(self);
        td.compare_oids(&a, &b, Some(&self.path));
        td.changes
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
}

pub trait Storable {
    fn serialize(&self) -> Vec<u8>;
    fn oid(&self) -> String {
        sha1::Sha1::from(&self.serialize()).hexdigest()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Blob {
    pub(crate) data: String,
}

impl Blob {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    fn content(&self) -> String {
        self.data.to_string()
    }
}

impl TryFrom<Vec<u8>> for Blob {
    type Error = failure::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let data = String::from_utf8(value)?;
        Ok(Self { data })
    }
}

impl Storable for Blob {
    fn serialize(&self) -> Vec<u8> {
        let s = self.content();
        format!("blob {}\0{}", s.len(), s).into()
    }
}
