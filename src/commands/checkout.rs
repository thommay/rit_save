use crate::database::ObjectKind;
use crate::repository::Repository;
use crate::revision::RevisionResolver;
use crate::BoxResult;
use clap::{App, Arg, ArgMatches, SubCommand};

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("checkout").arg(Arg::with_name("BRANCH").required(true).index(1))
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");
    let mut repository = Repository::new(root)?;
    let branch = matches
        .value_of("BRANCH")
        .expect("failed to specify branch name");

    let mut rr = RevisionResolver::new(&repository.database, &repository.refs, branch);
    let res = rr.resolver(ObjectKind::Commit);
    let branch_oid = if let Err(e) = res {
        for error in rr.errors {
            eprintln!("{}", error);
        }
        eprintln!("fatal: {}", e);
        None
    } else {
        res.ok()
    };
    let head = repository.refs.get_head();

    let tree_diff = repository.database.tree_diff(head, branch_oid);
    let migration = repository.migration(tree_diff).plan_changes();

    if let Err(e) = repository.apply_migration(migration) {
        eprintln!("{}", e)
    };

    repository.commit_changes()?;
    Ok(())
}
