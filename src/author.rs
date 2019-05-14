#[derive(Default,Debug)]
pub struct Author{
    name: String,
    email: String,
    time: u64,
}

impl Author {
    pub fn new(name: String, email: String, t: std::time::SystemTime) -> Self {
        let time = t.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
        Author { name, email, time }
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} <{}> {} +0000", self.name, self.email, self.time)
    }
}