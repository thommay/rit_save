use chrono::prelude::*;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct Author {
    name: String,
    email: String,
    time: DateTime<Local>,
}

impl Author {
    pub fn new(name: String, email: String, time: DateTime<Local>) -> Self {
        Author { name, email, time }
    }

    pub fn short_date(&self) -> String {
        self.time.format("%Y-%m-%d").to_string()
    }
}

impl TryFrom<&str> for Author {
    type Error = failure::Error;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        let mut parts = line.split('<');
        let name = parts
            .next()
            .map(|x| String::from(x.trim()))
            .expect("failed to get name");

        let line = parts.next().unwrap();
        let mut matches = line.split_whitespace();
        let email = matches
            .next()
            .expect("failed to get email")
            .trim_matches(|c| c == '<' || c == '>');

        let time = matches.collect::<Vec<&str>>().join(" ");
        let time = Local
            .datetime_from_str(time.as_ref(), "%s %z")
            .unwrap_or_else(|_| Local::now());
        Ok(Self {
            name,
            email: String::from(email),
            time,
        })
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} <{}> {}",
            self.name,
            self.email,
            self.time.format("%s %z")
        )
    }
}
