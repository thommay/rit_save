use crate::database::marker::{Kind, Marker};
use crate::database::Storable;
use crate::index::entry::Entry;
use crate::utilities::pack_data;
use indexmap::IndexMap;
use std::convert::TryFrom;
use std::io::{BufRead, Read, Write};
use std::path::Component;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub entries: IndexMap<String, TreeEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeEntry {
    Entry(Entry),
    Tree(Tree),
    Marker(Marker),
}
impl TreeEntry {
    pub fn kind(&self) -> Kind {
        match self {
            TreeEntry::Entry(_) => Kind::Entry,
            TreeEntry::Tree(_) => Kind::Tree,
            TreeEntry::Marker(m) => m.kind(),
        }
    }

    pub fn is_tree(&self) -> bool {
        self.kind() == Kind::Tree
    }

    pub fn oid(&self) -> String {
        match self {
            TreeEntry::Tree(t) => t.oid(),
            TreeEntry::Entry(e) => e.oid().to_owned(),
            TreeEntry::Marker(m) => m.clone().oid,
        }
    }
}

impl Tree {
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub fn build(entries: Vec<Entry>) -> Self {
        let mut root = Self::new();
        for entry in entries {
            let mut parts: Vec<Component> = entry.path.components().collect();
            let name = parts.pop().unwrap().as_os_str().to_str().unwrap();
            root.add_entry(parts, name, entry.clone());
        }
        root
    }

    pub fn get_entry(&self, key: &str) -> Option<&TreeEntry> {
        self.entries.get(key)
    }
    fn add_entry(&mut self, parts: Vec<Component>, name: &str, entry: Entry) {
        if let Some((first, rest)) = parts.split_first() {
            if first == &Component::CurDir && rest.is_empty() {
                self.entries
                    .insert(String::from(name), TreeEntry::Entry(entry));
            } else if first == &Component::CurDir {
                self.add_entry(rest.to_vec(), name, entry);
            } else {
                let first = first.as_os_str().to_str().unwrap();
                if let TreeEntry::Tree(ref mut tree) =
                    self.entries
                        .entry(first.into())
                        .or_insert(TreeEntry::Tree(Tree {
                            entries: IndexMap::new(),
                        }))
                {
                    tree.add_entry(rest.to_vec(), name, entry);
                }
            }
        } else {
            self.entries
                .insert(String::from(name), TreeEntry::Entry(entry));
        }
    }

    pub fn mode(&self) -> String {
        "40000".into()
    }

    pub fn kind(&self) -> Kind {
        Kind::Tree
    }

    pub fn traverse<T>(&self, f: &T)
    where
        T: Fn(Tree),
    {
        for entry in self.entries.values() {
            if let TreeEntry::Tree(tree) = entry {
                tree.traverse(f);
            }
        }
        f(self.clone());
    }
}

impl TryFrom<Vec<u8>> for Tree {
    type Error = failure::Error;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        let mut data = std::io::Cursor::new(data);
        let len = data.get_ref().len();
        let mut entries = IndexMap::new();
        while (data.position() as usize) < len - 1 {
            let mut mode = vec![];
            data.read_until(b' ', &mut mode)?;
            let mode = String::from_utf8(mode)?;
            let mode = mode.trim_end_matches(' ');

            let mut name = vec![];
            data.read_until(b'\0', &mut name)?;
            let name = String::from_utf8(name)?;
            let name = name.trim_end_matches('\0');

            let mut oid = [0; 20];
            data.read_exact(&mut oid)?;
            let oid = hex::encode(oid);

            let marker = Marker::new(name, oid, mode);
            entries.insert(String::from(name), TreeEntry::Marker(marker));
        }
        Ok(Tree { entries })
    }
}

impl Storable for Tree {
    fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for (name, entry) in &self.entries {
            let ret = match entry {
                TreeEntry::Tree(t) => {
                    let mode = t.mode();
                    let oid = t.oid();
                    pack_data(mode.as_ref(), name.as_ref(), oid.as_ref()).unwrap()
                }
                TreeEntry::Entry(e) => e.metadata(),
                TreeEntry::Marker(m) => m.metadata(),
            };
            data.write_all(&ret).unwrap();
        }
        let mut ret: Vec<u8> = format!("tree {}\0", data.len()).into();

        ret.write_all(&data).unwrap();
        ret
    }
}
