use crate::author::Author;
use crate::database::Storable;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Write;

#[derive(Debug)]
pub struct Commit {
    pub parent: Option<String>,
    pub tree: String,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(parent: Option<String>, tree: &str, author: Author, message: &str) -> Self {
        let tree = String::from(tree);
        let message = String::from(message);
        Self {
            parent,
            tree,
            author,
            message,
        }
    }
    pub fn title_line(&self) -> Option<String> {
        self.message.lines().nth(0).map(String::from)
    }
}

impl TryFrom<Vec<u8>> for Commit {
    type Error = failure::Error;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        let mut headers = HashMap::new();
        let data = String::from_utf8(data)?;
        let mut data = data.lines();
        loop {
            let line = data.next();
            if let Some(line) = line {
                let line = line.trim();
                if line == "" {
                    break;
                }
                let mut matches = line.split_whitespace();
                let key = matches.next().unwrap();
                let val = matches.collect::<Vec<&str>>().join(" ");
                headers.insert(key, val);
            }
        }
        let message = data.collect::<Vec<&str>>().join("\n");
        let parent = headers.get("parent").and_then(|x| Some(x.to_string()));
        let tree = headers
            .get("tree")
            .expect("failed to read tree from commit")
            .to_string();
        let author = headers
            .get("author")
            .map(|x| Author::from(x))
            .expect("failed to read author from commit")
            .unwrap();
        Ok(Self {
            parent,
            tree,
            author,
            message,
        })
    }
}

impl Storable for Commit {
    fn serialize(&self) -> Vec<u8> {
        let mut content = format!("tree {}\n", self.tree);
        match &self.parent {
            Some(p) => writeln!(&mut content, "parent {}", p).unwrap(),
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
