use crate::author::Author;
use crate::commit::Commit;
use crate::database::{Database, Storable};
use crate::index::Index;
use crate::refs::Refs;
use crate::tree::Tree;
use crate::BoxResult;
use chrono::Utc;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::io::Read;

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("commit").arg(
        Arg::with_name("msg")
            .takes_value(true)
            .short("m")
            .help("sets the commit message"),
    )
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let db = Database::new(root.join(".git/objects"));
    let refs = Refs::new(root.join(".git"));
    let index = Index::from(root.join(".git/index"))?;

    let root = Tree::build(index.entries());
    root.traverse(&|x| db.store(x).unwrap());

    let name = std::env::var("GIT_AUTHOR_NAME")?;
    let email = std::env::var("GIT_AUTHOR_EMAIL")?;
    let author = Author::new(name, email, Utc::now());

    let mut msg = String::new();
    let message = if matches.is_present("msg") {
        matches.value_of("msg").unwrap()
    } else {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        handle.read_to_string(&mut msg)?;
        msg.as_ref()
    };

    let parent = refs.get_head();
    let parented = parent.is_some();

    let commit = Commit::new(parent, &root.oid(), author, message);

    if parented {
        println!("[{}]", &commit.oid());
    } else {
        println!("[(root-commit) {}]", &commit.oid());
    }

    refs.update_head(&commit.oid())?;

    db.store(commit)?;
    index.release_lock()?;
    Ok(())
}
