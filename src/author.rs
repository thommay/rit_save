use failure::Error;

#[derive(Default, Debug)]
pub struct Author {
    name: String,
    email: String,
    time: u64,
}

impl Author {
    pub fn new(name: String, email: String, t: std::time::SystemTime) -> Self {
        let time = t
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Author { name, email, time }
    }

    pub fn from(line: &str) -> Result<Self, Error> {
        let mut parts = line.split('<');
        let name = parts.next().map(|x| String::from(x.trim())).expect("failed to get name");

        let line = parts.next().unwrap();
        let mut matches = line.split_whitespace();
        let email = matches.next().expect("failed to get email").trim_matches(|c| c == '<' || c == '>' );
        let time = matches.next().expect("failed to get time").parse::<u64>()?;
        Ok(Self { name, email: String::from(email), time})
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} <{}> {} +0000", self.name, self.email, self.time)
    }
}
