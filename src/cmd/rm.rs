use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};

use crate::{
    index::{DeltaIndex, DeltaIndexEntry},
    repo::DeltaRepository,
};

pub fn rm(path: Vec<PathBuf>) -> Result<()> {
    let repo = DeltaRepository::repo_find(std::env::current_dir()?)?;

    Ok(())
}

fn remove(
    repo: &DeltaRepository,
    paths: Vec<&Path>,
    delete: bool,
    skip_missing: bool,
) -> Result<()> {
    let mut index = DeltaIndex::read_index(repo)?;
    let worktree = &repo.worktree;
    let mut absolute_paths = HashSet::<PathBuf>::new();

    for path in paths {
        let absolute_path = fs::canonicalize(path)?;
        if absolute_path.starts_with(worktree) {
            absolute_paths.insert(absolute_path);
        } else {
            return Err(anyhow!(
                "Cannot remove paths outside of worktree: {}",
                path.display()
            ));
        }
    }

    let mut kept_entries = Vec::<DeltaIndexEntry>::new();
    let mut remove = Vec::<PathBuf>::new();

    for e in index.entries {
        let path = worktree.join(&e.name);

        if absolute_paths.contains(&path) {
            absolute_paths.remove(&path);
            remove.push(path);
        } else {
            kept_entries.push(e);
        }
    }

    if !absolute_paths.is_empty() && !skip_missing {
        return Err(anyhow!(
            "Cannot remove paths not in the index: {:?}",
            absolute_paths
        ));
    }

    if delete {
        for path in remove {
            fs::remove_file(path)?;
        }
    }

    index.entries = kept_entries;
    index.write_index(repo)
}
