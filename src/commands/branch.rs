use crate::database::ObjectKind;
use crate::repository::Repository;
use crate::revision::RevisionResolver;
use crate::BoxResult;
use clap::{App, Arg, ArgMatches, SubCommand};

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("branch")
        .arg(Arg::with_name("BRANCH").required(true).index(1))
        .arg(Arg::with_name("START").required(false).index(2))
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");
    let repository = Repository::new(root)?;
    let name = matches
        .value_of("BRANCH")
        .expect("failed to specify branch name");

    let start_oid = if let Some(start) = matches.value_of("START") {
        let mut rr = RevisionResolver::new(&repository.database, &repository.refs, start);
        let res = rr.resolver(ObjectKind::Commit);
        if let Err(e) = res {
            for error in rr.errors {
                eprintln!("{}", error);
            }
            eprintln!("fatal: {}", e);
            None
        } else {
            res.ok()
        }
    } else {
        repository.refs.get_head()
    };

    if start_oid.is_some() {
        if let Err(e) = repository.refs.create_branch(name, start_oid) {
            eprintln!("fatal: {}", e);
        }
    }
    repository.commit_changes()?;
    Ok(())
}
