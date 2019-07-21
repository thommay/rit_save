use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Edit {
    Insert(String),
    Delete(String),
    Equals(String),
}

impl fmt::Display for Edit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Edit::Equals(e) => format!(" {}", e),
                Edit::Insert(e) => format!("+ {}", e),
                Edit::Delete(e) => format!("- {}", e),
            }
        )
    }
}
