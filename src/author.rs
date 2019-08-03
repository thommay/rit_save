use chrono::prelude::*;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct Author {
    name: String,
    email: String,
    time: DateTime<Utc>,
}

impl Author {
    pub fn new(name: String, email: String, time: DateTime<Utc>) -> Self {
        Author { name, email, time }
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
        let time = matches.next().expect("failed to get time").parse::<i64>()?;
        let time = Utc.timestamp(time, 0);
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
            "{} <{}> {} +0000",
            self.name,
            self.email,
            self.time.timestamp()
        )
    }
}
