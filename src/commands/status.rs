use crate::index::Index;
use crate::workspace::Workspace;
use crate::{index, workspace, BoxResult};
use clap::ArgMatches;
use std::path::{PathBuf, Path};

fn trackable_file(
    workspace: &Workspace,
    index: &Index,
    path: &Path,
    stat: std::fs::Metadata,
) -> bool {
    if stat.is_file() { return !index.has_entry(path.to_str().unwrap()); }
    if !stat.is_dir() { return false; }

    let items = workspace.list_dir(Some(path.to_path_buf())).unwrap();

    for (path, stat) in items {
        if !stat.is_file() && !stat.is_dir() { continue; }
        if trackable_file(&workspace, &index, path.as_path(), stat) { return true; }
    }
    false
}

fn scan_workspace(
    workspace: &Workspace,
    index: &Index,
    path: Option<PathBuf>,
) -> BoxResult<Vec<String>> {
    let mut untracked = Vec::new();
    for (file, stat) in workspace.list_dir(path)? {
        if index.has_entry(file.to_str().unwrap()) {
            if stat.is_dir() {
                untracked.append(&mut scan_workspace(workspace, index, Some(file))?);
            }
        } else if trackable_file(workspace, index, file.as_path(), stat.clone()) {
            let mut file = file.to_str().unwrap().to_owned();
            if stat.is_dir() { file.push('/') };
            untracked.push(file);
        }
    }
    Ok(untracked)
}

pub fn exec(_matches: &ArgMatches) -> BoxResult<()> {
    let root = std::path::Path::new(".");

    let workspace = workspace::Workspace::new(root);
    let index = index::Index::from(root.join(".git/index"))?;

    let mut files = scan_workspace(&workspace, &index, None)?;

    files.sort();
    for file in files {
        println!("?? {}", file);
    }
    index.release_lock()?;
    Ok(())
}
