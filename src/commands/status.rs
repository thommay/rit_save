use crate::repository::{Repository, Status};
use crate::BoxResult;
use clap::{App, Arg, ArgMatches, SubCommand};
use colored::*;
use std::collections::BTreeMap;

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("status").arg(
        Arg::with_name("porcelain")
            .long("--porcelain")
            .help("Give the output in an easy-to-parse format for scripts."),
    )
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");
    let mut repository = Repository::new(root)?;
    let porcelain = matches.is_present("porcelain");
    repository.status()?;
    repository.print(porcelain);
    repository.commit_changes()?;
    Ok(())
}

trait StatusPrinter {
    fn print(&self, porcelain: bool);
    fn print_long_format(&self);
    fn print_status(&self);
    fn print_porcelain(&self);
    fn status_for(&self, file: &str) -> String;
}

impl StatusPrinter for Repository {
    fn print(&self, porcelain: bool) {
        if porcelain {
            self.print_porcelain();
        } else {
            self.print_long_format();
        }
    }

    fn print_long_format(&self) {
        let index = self.index_changes.clone();
        let workspace = self.workspace_changes.clone();
        let untracked = self.untracked.clone();

        print_changes("Changes to be committed", index, "green");
        print_changes("Changes not staged for commit", workspace, "red");

        if !untracked.is_empty() {
            println!("Untracked files");
            println!();
            for file in untracked {
                println!("\t{}", file.red());
            }
            println!();
        }
        self.print_status();
    }

    fn print_status(&self) {
        if !self.index_changes.is_empty() {
            return;
        }
        if !self.workspace_changes.is_empty() {
            println!("no changes added to commit");
        } else if !self.untracked.is_empty() {
            println!("nothing added to commit but untracked files present");
        } else {
            println!("nothing to commit, working tree clean");
        }
    }

    fn print_porcelain(&self) {
        let mut changed = self.changed.clone();
        changed.sort();
        changed.dedup();

        let untracked = self.untracked.clone();

        for file in changed {
            println!("{} {}", self.status_for(&file), file);
        }

        for file in untracked {
            println!("?? {}", file);
        }
    }

    fn status_for(&self, file: &str) -> String {
        format!(
            "{}{}",
            self.index_changes.get(file).unwrap_or(&Status::None),
            self.workspace_changes.get(file).unwrap_or(&Status::None)
        )
    }
}

fn long_format(status: Status) -> String {
    match status {
        Status::Deleted => String::from("deleted:"),
        Status::Modified => String::from("modified:"),
        Status::Added => String::from("new file:"),
        Status::None => String::new(),
    }
}

fn print_changes(msg: &str, index: BTreeMap<String, Status>, colour: &str) {
    if !index.is_empty() {
        println!("{}", msg);
        println!();
        for (path, status) in index {
            let item = format!("{:12}{}", long_format(status), path).color(colour);
            println!("\t{}", item);
        }
        println!();
    }
}
