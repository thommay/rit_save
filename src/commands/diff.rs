use crate::database::{Blob, Storable};
use crate::diff::hunk::Hunk;
use crate::diff::myers::Myers;
use crate::index::entry::Entry;
use crate::repository::{Repository, Status};
use crate::{BoxResult, CliError};
use clap::{App, Arg, ArgMatches, SubCommand};
use colored::Colorize;
use std::convert::TryFrom;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("diff").arg(
        Arg::with_name("cached")
            .long("--cached")
            .help("This form is to view the changes you staged for the next commit relative to the named commit."),
    )
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");
    let mut repository = Repository::new(root)?;
    //    pager();
    let cached = matches.is_present("cached");
    repository.status()?;
    if cached {
        repository.diff_head_index()?
    } else {
        repository.diff_index_workspace()?;
    }
    repository.commit_changes()?;
    Ok(())
}

trait Differ {
    fn diff_head_index(&self) -> BoxResult<()>;
    fn diff_index_workspace(&self) -> BoxResult<()>;
    fn get_index_file(&self, path: &str) -> BoxResult<Target>;
    fn get_head_file(&self, path: &str) -> BoxResult<Target>;
    fn get_workspace_file(&self, path: &str) -> BoxResult<Target>;
    fn get_deleted_file(&self) -> BoxResult<Target>;
    fn print_diff(&self, a: Target, b: Target);
}

const NILL_PATH: &str = "/dev/null";
const NILL_OID: &str = "0000000000000000000000000000000000000000";

struct Target {
    path: PathBuf,
    oid: String,
    mode: Option<String>,
    data: String,
}

impl Differ for Repository {
    fn diff_head_index(&self) -> BoxResult<()> {
        let changes = self.index_changes.clone();
        for (path, change) in changes {
            let path = path.as_str();
            match change {
                Status::Added => {
                    self.print_diff(self.get_deleted_file()?, self.get_index_file(path)?)
                }
                Status::Deleted => {
                    self.print_diff(self.get_head_file(path)?, self.get_deleted_file()?)
                }
                Status::Modified => {
                    self.print_diff(self.get_head_file(path)?, self.get_index_file(path)?)
                }
                _ => continue,
            };
        }
        Ok(())
    }

    fn diff_index_workspace(&self) -> BoxResult<()> {
        let workspace_changes = self.workspace_changes.clone();
        for (path, change) in workspace_changes {
            let path = path.as_str();
            match change {
                Status::Modified => {
                    self.print_diff(self.get_index_file(path)?, self.get_workspace_file(path)?)
                }
                Status::Deleted => {
                    self.print_diff(self.get_index_file(path)?, self.get_deleted_file()?)
                }
                _ => continue,
            };
        }
        Ok(())
    }

    fn get_index_file(&self, path: &str) -> BoxResult<Target> {
        if let Some(entry) = self.index.get_entry(path) {
            let mode = String::from(&entry.mode());
            let oid = String::from(&entry.oid);
            let oid = self.database.truncate_oid(oid.as_ref()).unwrap_or(oid);
            let path = Path::new(path).to_path_buf();
            let (_, _, data) = self.database.read_object(&entry.oid)?;
            let blob = Blob::try_from(data)?;
            Ok(Target {
                path,
                oid,
                mode: Some(mode),
                data: blob.data,
            })
        } else {
            Err(CliError::new("Failed to get file from workspace").into())
        }
    }

    fn get_head_file(&self, path: &str) -> BoxResult<Target> {
        if let Some(entry) = self.tree.get(Path::new(path)) {
            let mode = String::from(&entry.mode);
            let oid = String::from(&entry.oid);
            let oid = self.database.truncate_oid(oid.as_ref()).unwrap_or(oid);
            let path = Path::new(path).to_path_buf();
            let (_, _, data) = self.database.read_object(&entry.oid)?;
            let blob = Blob::try_from(data)?;
            Ok(Target {
                path,
                oid,
                mode: Some(mode),
                data: blob.data,
            })
        } else {
            Err(CliError::new("Failed to get file from tree").into())
        }
    }

    fn get_workspace_file(&self, path: &str) -> BoxResult<Target> {
        if let Ok(file) = self.workspace.read_file(path) {
            let blob = Blob::new(file);
            let oid = blob.oid();
            let oid = self.database.truncate_oid(oid.as_ref()).unwrap_or(oid);
            let stats = self
                .stats
                .get(Path::new(path))
                .expect("couldn't find entry in database");
            let mode = Entry::mode_from_stat(stats.mode());
            let path = Path::new(path).to_path_buf();
            let data = self.workspace.read_file(&path)?;
            Ok(Target {
                path,
                oid,
                mode: Some(mode),
                data,
            })
        } else {
            Err(CliError::new("Failed to get file from workspace").into())
        }
    }

    fn get_deleted_file(&self) -> BoxResult<Target> {
        let path = Path::new(NILL_PATH).to_path_buf();
        let oid = self
            .database
            .truncate_oid(NILL_OID)
            .unwrap_or_else(|| String::from(NILL_OID));
        Ok(Target {
            path,
            oid,
            mode: None,
            data: String::new(),
        })
    }

    fn print_diff(&self, a: Target, b: Target) {
        let a_pth_str = Path::new("a").join(a.path);
        let a_pth_str = a_pth_str.to_str().expect("couldn't extract path for diff");
        let b_pth_str = Path::new("b").join(b.path);
        let b_pth_str = b_pth_str.to_str().expect("couldn't extract path for diff");

        println!(
            "{}",
            format!("diff --git {} {}", a_pth_str, b_pth_str).bold()
        );

        let mode_str = if a.mode.is_none() {
            println!("{}", format!("new file mode {}", b.mode.unwrap()).bold());
            String::new()
        } else if b.mode.is_none() {
            println!(
                "{}",
                format!("deleted file mode {}", a.mode.unwrap()).bold()
            );
            String::new()
        } else if a.mode != b.mode {
            println!("{}", format!("old mode {}", a.mode.unwrap()).bold());
            println!("{}", format!("new mode {}", b.mode.unwrap()).bold());
            String::new()
        } else {
            format!(" {}", &a.mode.unwrap().bold())
        };

        if a.oid == b.oid {
            return;
        }

        println!(
            "{}",
            format!("index {}..{}{}", a.oid, b.oid, mode_str).bold()
        );
        println!("{}", format!("--- {}", a_pth_str).bold());
        println!("{}", format!("+++ {}", b_pth_str).bold());

        let edits = Myers::from(a.data.as_ref(), b.data.as_ref()).diff();
        for hunk in Hunk::filter(edits) {
            println!("{}", hunk.header().cyan());
            for edit in hunk.edits {
                println!("{}", edit);
            }
        }
    }
}
