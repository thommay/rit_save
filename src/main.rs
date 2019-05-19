use crate::author::Author;
use crate::commit::Commit;
use crate::database::{Blob, Storable};
use crate::tree::Tree;
use crate::utilities::stat_file;
use clap::App;
use clap::ArgMatches;
use clap::{Arg, SubCommand};
use std::io::Read;

mod author;
mod commit;
mod database;
mod index;
mod lockfile;
mod refs;
mod tree;
mod utilities;
mod workspace;

type BoxResult<T> = Result<T, Box<std::error::Error>>;

fn main() -> BoxResult<()> {
    let app = App::new("jit")
        .version("0.0.1")
        .about("my git clone")
        .subcommand(
            SubCommand::with_name("add").arg(
                Arg::with_name("PATH")
                    .required(true)
                    .index(1)
                    .multiple(true),
            ),
        )
        .subcommand(
            SubCommand::with_name("commit").arg(
                Arg::with_name("msg")
                    .takes_value(true)
                    .short("m")
                    .help("sets the commit message"),
            ),
        )
        .subcommand(
            SubCommand::with_name("init").arg(Arg::with_name("PATH").required(true).index(1)),
        )
        .get_matches();

    match app.subcommand() {
        ("add", Some(m)) => git_add(m),
        ("commit", Some(m)) => git_commit(m),
        ("init", Some(m)) => git_init(m),
        _ => {
            println!("unrecognised command");
            Err(From::from("unrecognised command"))
        }
    }
}

fn git_add(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let workspace = workspace::Workspace::new(root.into());
    let db = database::Database::new(root.join(".git/objects"));
    let mut index = index::Index::from(root.join(".git/index"))?;

    for p in matches
        .values_of("PATH")
        .unwrap()
        .collect::<Vec<_>>()
        .iter()
    {
        let path = std::path::PathBuf::from(p);
        for file in workspace.list_files(Some(path))?.iter() {
            let data = workspace.read_file(file)?;
            let stat = stat_file(file)?;

            let blob = Blob::new(data);
            db.store(blob.clone())?;
            index.add(file.as_path(), blob.oid().as_ref(), stat);
        }
    }

    index.write_updates()?;
    Ok(())
}

fn git_commit(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let db = database::Database::new(root.join(".git/objects"));
    let refs = refs::Refs::new(root.join(".git"));
    let index = index::Index::from(root.join(".git/index"))?;

    let root = Tree::build(index.into(), ".");
    root.traverse(&|x| db.store(x).unwrap());

    let name = std::env::var("GIT_AUTHOR_NAME")?;
    let email = std::env::var("GIT_AUTHOR_EMAIL")?;
    let author = Author::new(name, email, std::time::SystemTime::now());

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
    Ok(())
}

fn git_init(matches: &ArgMatches) -> BoxResult<()> {
    let path = std::path::Path::new(matches.value_of("PATH").unwrap());
    let target = path.join(".git");
    std::fs::create_dir_all(target.join("objects"))?;
    std::fs::create_dir_all(target.join("refs"))?;
    Ok(())
}
