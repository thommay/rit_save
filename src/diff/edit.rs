use colored::Colorize;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum EditKind {
    Insert,
    Delete,
    Equals,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub content: String,
    pub number: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub kind: EditKind,
    pub a: Option<Line>,
    pub b: Option<Line>,
}

impl fmt::Display for Edit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            EditKind::Equals => {
                if let Some(ref l) = self.a {
                    write!(f, " {}", l.content)?;
                }
            }
            EditKind::Insert => {
                if let Some(ref l) = self.b {
                    write!(f, "{}", format!("+ {}", l.content).green())?;
                }
            }
            EditKind::Delete => {
                if let Some(ref l) = self.a {
                    write!(f, "{}", format!("- {}", l.content).red())?;
                }
            }
        }
        Ok(())
    }
}

impl Edit {
    pub fn insert(a: Option<Line>, b: Option<Line>) -> Self {
        Edit {
            kind: EditKind::Insert,
            a,
            b,
        }
    }
    pub fn delete(a: Option<Line>, b: Option<Line>) -> Self {
        Edit {
            kind: EditKind::Delete,
            a,
            b,
        }
    }
    pub fn equals(a: Option<Line>, b: Option<Line>) -> Self {
        Edit {
            kind: EditKind::Equals,
            a,
            b,
        }
    }

    pub fn is_equals(&self) -> bool {
        match self.kind {
            EditKind::Equals => true,
            _ => false,
        }
    }
}
