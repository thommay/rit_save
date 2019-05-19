use crate::database::Storable;
use crate::index::entry::Entry;
use crate::utilities::pack_data;
use indexmap::IndexMap;
use std::io::Write;
use std::path::Component;

#[derive(Clone, Debug)]
pub struct Tree {
    pub entries: IndexMap<String, TreeEntry>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum TreeEntry {
    Entry(Entry),
    Tree(Tree),
}

impl Tree {
    pub fn build(entries: Vec<Entry>, name: &str) -> Self {
        let mut root = Tree {
            entries: IndexMap::new(),
            name: String::from(name),
        };
        for entry in entries {
            let mut parts: Vec<Component> = entry.path.components().collect();
            let name = parts.pop().unwrap().as_os_str().to_str().unwrap();
            root.add_entry(parts, name, entry.clone());
        }
        root
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
                            name: first.into(),
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
            };
            data.write_all(&ret).unwrap();
        }
        let mut ret: Vec<u8> = format!("tree {}\0", data.len()).into();

        ret.write_all(&data).unwrap();
        ret
    }
}
