use crate::commit::Commit;
use crate::database::{ObjectKind, Storable};
use crate::{commit, database, refs};
use failure::format_err;
use failure::Error;
use lazy_static::lazy_static;
use regex::{Regex, RegexSet};
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum Revision {
    Ref { name: String },
    Parent { rev: Box<Revision> },
    Ancestor { rev: Box<Revision>, n: usize },
}

impl Revision {
    pub fn from(revision: &str) -> Result<Self, Error> {
        if let Some(matches) = PARENT_RE.captures(revision) {
            let rev = Revision::from(&matches[1])?;
            return Ok(Revision::Parent { rev: Box::new(rev) });
        } else if let Some(matches) = ANCESTOR_RE.captures(revision) {
            let rev = Revision::from(&matches[1])?;
            let n: &usize = &matches[2].parse()?;
            return Ok(Revision::Ancestor {
                rev: Box::new(rev),
                n: n.to_owned(),
            });
        } else if !INVALID_NAME.is_match(revision) {
            let name = if revision == "@" { "HEAD" } else { revision };
            return Ok(Revision::Ref {
                name: name.to_owned(),
            });
        }
        Err(format_err!("fatal: Could not parse revision"))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HintedError {
    message: String,
    hints: Vec<String>,
}

impl std::fmt::Display for HintedError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        writeln!(f, "error: {}", self.message)?;
        for line in self.hints.clone() {
            writeln!(f, "hint: {}", line)?;
        }
        Ok(())
    }
}

pub struct RevisionResolver<'a> {
    db: &'a database::Database,
    refs: &'a refs::Refs,
    expr: String,
    pub(crate) errors: Vec<HintedError>,
}

impl<'a> RevisionResolver<'a> {
    pub fn new(db: &'a database::Database, refs: &'a refs::Refs, expr: &str) -> Self {
        Self {
            db,
            refs,
            expr: String::from(expr),
            errors: vec![],
        }
    }

    pub fn resolver(&mut self, kind: ObjectKind) -> Result<String, Error> {
        let rev = Revision::from(self.expr.as_ref())?;
        if let Some(oid) = self.resolv(rev) {
            let (k, _, _) = self.db.read_object(oid.as_ref())?;
            if kind == k {
                return Ok(oid);
            } else {
                let message = format!("object {} is a {}, not a {}", oid, k, kind);
                self.errors.push(HintedError {
                    message,
                    hints: vec![],
                });
            }
        }
        Err(format_err!("Not a valid object name: '{}'", self.expr))
    }

    fn resolv(&mut self, rev: Revision) -> Option<String> {
        match rev {
            Revision::Ref { name } => self.read_ref(name.as_ref()),
            Revision::Parent { rev } => {
                let oid = self.resolv(*rev);
                self.commit_parent(oid)
            }
            Revision::Ancestor { rev, n } => {
                let mut oid = self.resolv(*rev);
                for _ in 1..=n {
                    oid = self.commit_parent(oid);
                }
                oid
            }
        }
    }

    fn commit_parent(&self, rev: Option<String>) -> Option<String> {
        if let Some(rev) = rev {
            if let Ok((kind, _size, data)) = self.db.read_object(rev.as_ref()) {
                if kind.is_commit() {
                    if let Ok(commit) = commit::Commit::try_from(data) {
                        return commit.parent;
                    }
                }
            }
        }
        None
    }

    fn read_ref(&mut self, name: &str) -> Option<String> {
        let oid = self.refs.read_ref(name);
        if oid.is_some() {
            return oid;
        } else if let Ok(mut candidates) = self.db.prefix_match(name) {
            match candidates.len() {
                0 => return None,
                1 => return candidates.first().cloned(),
                _ => self.log_ambiguous_sha1(name, &mut candidates),
            };
        }
        None
    }
    fn log_ambiguous_sha1(&mut self, name: &str, candidates: &mut Vec<String>) {
        candidates.sort();
        let mut objects = candidates
            .iter()
            .map(|oid| {
                if let Ok((kind, _, data)) = self.db.read_object(oid) {
                    if kind.is_commit() {
                        let c = Commit::try_from(data).expect("failed to load commit");

                        let oid = self.db.truncate_oid(c.oid().as_ref());

                        format!(
                            "{} {} {} {}",
                            oid,
                            "commit",
                            c.author.short_date(),
                            c.title_line().expect("commit is fucked")
                        )
                    } else {
                        dbg!(&oid);
                        let oid = self.db.truncate_oid(oid);
                        format!("{} {}", oid, kind)
                    }
                } else {
                    String::new()
                }
            })
            .collect::<Vec<String>>();
        let message = format!("short SHA1 {} is ambiguous", name);
        let mut hints = vec![String::from("The candidates are:")];
        hints.append(&mut objects);
        self.errors.push(HintedError { message, hints });
    }
}

lazy_static! {
    static ref PARENT_RE: Regex = Regex::new(r"^(.+)\^$").unwrap();
    static ref ANCESTOR_RE: Regex = Regex::new(r"^(.+)~(\d+)$").unwrap();
    pub static ref INVALID_NAME: RegexSet = RegexSet::new(&[
        r"^\.",
        r"/\.",
        r"\.\.",
        r"/$",
        r"\.lock$",
        r"@\{",
        r"[\x00-\x20*:?\[\\^=\x7f]"
    ])
    .unwrap();
}

#[cfg(test)]
mod test {
    use super::Revision;

    #[test]
    fn parse_head_parent() {
        let rev = "@^";
        let parsed = Revision::from(rev).unwrap();
        assert_eq!(
            parsed,
            Revision::Parent {
                rev: Box::new(Revision::Ref {
                    name: "HEAD".to_owned()
                })
            }
        )
    }

    #[test]
    fn parse_double_parent() {
        let rev = "master^^";
        let parsed = Revision::from(rev).unwrap();
        assert_eq!(
            parsed,
            Revision::Parent {
                rev: Box::new(Revision::Parent {
                    rev: Box::new(Revision::Ref {
                        name: "master".to_owned()
                    })
                }),
            }
        )
    }

    #[test]
    fn parse_head_ancestor() {
        let rev = "HEAD~42";
        let parsed = Revision::from(rev).unwrap();
        assert_eq!(
            parsed,
            Revision::Ancestor {
                rev: Box::new(Revision::Ref {
                    name: "HEAD".to_owned()
                }),
                n: 42,
            }
        )
    }

    #[test]
    fn parse_hard_ancestor() {
        let rev = "abc123~3^";
        let parsed = Revision::from(rev).unwrap();
        assert_eq!(
            parsed,
            Revision::Parent {
                rev: Box::new(Revision::Ancestor {
                    rev: Box::new(Revision::Ref {
                        name: "abc123".to_owned()
                    }),
                    n: 3,
                }),
            }
        )
    }

}
