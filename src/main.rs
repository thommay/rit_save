use rit::database::{Blob, Database, Storable};
use rit::utilities::stat_file;

use clap::App;
use clap::ArgMatches;
use clap::{Arg, SubCommand};
use rit::commands::{commit, diff, status};
use rit::index::Index;
use rit::workspace::Workspace;
use rit::BoxResult;

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
        .subcommand(commit::cli())
        .subcommand(diff::cli())
        .subcommand(
            SubCommand::with_name("init").arg(Arg::with_name("PATH").required(true).index(1)),
        )
        .subcommand(status::cli())
        .get_matches();

    match app.subcommand() {
        ("add", Some(m)) => git_add(m),
        ("commit", Some(m)) => commit::exec(m),
        ("diff", Some(m)) => diff::exec(m),
        ("init", Some(m)) => git_init(m),
        ("status", Some(m)) => status::exec(m),
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

fn git_init(matches: &ArgMatches) -> BoxResult<()> {
    let path = std::path::Path::new(matches.value_of("PATH").unwrap());
    let target = path.join(".git");
    std::fs::create_dir_all(target.join("objects"))?;
    std::fs::create_dir_all(target.join("refs"))?;
    Ok(())
}
