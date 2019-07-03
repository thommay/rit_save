use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Edit {
    Insert,
    Delete,
    Equals,
}

impl fmt::Display for Edit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Edit::Equals => " ",
                Edit::Insert => "+",
                Edit::Delete => "-",
            }
        )
    }
}
