use clap::App;
use clap::{Arg, SubCommand};
use clap::ArgMatches;
use crate::entry::Entry;
use crate::database::{Blob, Storable};
use crate::tree::Tree;
use crate::author::Author;
use std::io::Read;
use crate::commit::Commit;

mod author;
mod commit;
mod database;
mod entry;
mod lockfile;
mod refs;
mod tree;
mod utilities;
mod workspace;

type BoxResult<T> = Result<T, Box<std::error::Error>>;

fn main() -> BoxResult<()>{
    let app = App::new("jit").version("0.0.1").about("my git clone")
        .subcommand(SubCommand::with_name("commit"))
        .subcommand(SubCommand::with_name("init")
            .arg(Arg::with_name("PATH").required(true).index(1))).get_matches();

    match app.subcommand() {
        ("init", Some(init_matches)) => git_init(init_matches),
        ("commit", Some(commit_matches)) => git_commit(commit_matches),
        _ => {
            println!("unrecognised command");
            Err(From::from("unrecognised command"))
        },
    }
}

fn git_init(matches: &ArgMatches) -> BoxResult<()>{
    let path = std::path::Path::new(matches.value_of("PATH").unwrap());
    let target = path.join(".git");
    std::fs::create_dir_all(target.join("objects"))?;
    std::fs::create_dir_all(target.join("refs"))?;
    Ok(())
}

fn git_commit(_: &ArgMatches) -> BoxResult<()> {
    let path = std::path::Path::new(".");

    let workspace = workspace::Workspace::new(path.into());
    let db = database::Database::new(path.join(".git/objects"));
    let refs = refs::Refs::new(path.join(".git"));

    let mut entries: Vec<Entry> = vec![];
    for file in workspace.list_files()?.iter() {
        let b = Blob::new(workspace.read_file(file)?);
        db.store(b.clone())?;

        let exe = workspace.is_executable(file)?;
        let entry  = Entry::new(file.into(), b.oid(), exe);
        entries.push(entry);
    }

    let root = Tree::build(entries, ".");
    root.traverse(&|x| db.store(x).unwrap());
    println!("tree is at {:?}", &root.oid());

    let name = std::env::var("GIT_AUTHOR_NAME")?;
    let email = std::env::var("GIT_AUTHOR_EMAIL")?;
    let author = Author::new(name, email, std::time::SystemTime::now());

    let mut message = String::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    handle.read_to_string(&mut message)?;

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
    Ok(())
}
