use crate::author::Author;
use crate::database::Storable;
use std::fmt::Write;

#[derive(Debug)]
pub struct Commit {
    parent: Option<String>,
    oid: String,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(parent: Option<String>, oid: &str, author: Author, message: &str) -> Self {
        let oid = String::from(oid);
        let message = String::from(message);
        Self {
            parent,
            oid,
            author,
            message,
        }
    }
}

impl Storable for Commit {
    fn serialize(&self) -> Vec<u8> {
        let mut content = format!("tree {}\n", self.oid);
        match &self.parent {
            Some(p) => write!(&mut content, "parent {}", p).unwrap(),
            None => {}
        }
        write!(
            &mut content,
            "author {}\ncommitter {}\n\n{}",
            self.author, self.author, self.message
        )
        .unwrap();
        format!("commit {}\0{}", content.len(), content).into()
    }
}
