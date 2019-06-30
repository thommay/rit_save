use rit::author::Author;
use rit::commit::Commit;
use rit::database::{Blob, Database, Storable};
use rit::tree::Tree;
use rit::utilities::stat_file;

use clap::App;
use clap::ArgMatches;
use clap::{Arg, SubCommand};
use rit::commands::status::CmdStatus;
use rit::index::Index;
use rit::refs::Refs;
use rit::workspace::Workspace;
use rit::BoxResult;
use std::io::Read;

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
        .subcommand(SubCommand::with_name("status"))
        .subcommand(SubCommand::with_name("show_head"))
        .get_matches();

    match app.subcommand() {
        ("add", Some(m)) => git_add(m),
        ("commit", Some(m)) => git_commit(m),
        ("init", Some(m)) => git_init(m),
        ("show_head", Some(_)) => show_head(),
        ("status", Some(m)) => CmdStatus::new(".")?.exec(m),
        _ => {
            println!("unrecognised command");
            Err(From::from("unrecognised command"))
        }
    }
}

fn git_add(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let workspace = Workspace::new(root);
    let db = Database::new(root.join(".git/objects"));
    let mut index = Index::from(root.join(".git/index"))?;

    for p in matches
        .values_of("PATH")
        .unwrap()
        .collect::<Vec<_>>()
        .iter()
    {
        let path = std::path::PathBuf::from(p);
        let files = workspace.list_files(Some(path));
        if files.is_err() {
            index.release_lock()?;
            eprintln!("fatal: pathspec '{}' did not match any files", p);
            std::process::exit(128);
        }
        for file in files.unwrap().iter() {
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

    let db = Database::new(root.join(".git/objects"));
    let refs = Refs::new(root.join(".git"));
    let index = Index::from(root.join(".git/index"))?;

    let root = Tree::build(index.entries());
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
    index.release_lock()?;
    Ok(())
}

fn git_init(matches: &ArgMatches) -> BoxResult<()> {
    let path = std::path::Path::new(matches.value_of("PATH").unwrap());
    let target = path.join(".git");
    std::fs::create_dir_all(target.join("objects"))?;
    std::fs::create_dir_all(target.join("refs"))?;
    Ok(())
}

fn show_head() -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let db = Database::new(root.join(".git/objects"));
    let refs = Refs::new(root.join(".git"));
    let head = refs.get_head();
    if let Some(head) = head {
        let (kind, _, data) = db.read_object(head.as_ref())?;
        if kind == "commit" {
            let commit = Commit::from(data)?;
            let tree = commit.tree;
            let (kind, _, data) = db.read_object(tree.as_ref())?;
            if kind == "tree" {
                let tree = Tree::from(data)?;
                dbg!(&tree);
            }
        }
    }
    Ok(())
}
