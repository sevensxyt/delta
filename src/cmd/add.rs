use std::{collections::HashSet, fs, path::PathBuf};

use anyhow::{anyhow, Result};

use crate::repo::DeltaRepository;

use super::rm;

pub fn add(path: Vec<PathBuf>) -> Result<()> {
    let repo = DeltaRepository::repo_find(std::env::current_dir()?)?;

    add_to_index(&repo, &path, true, false)
}

fn add_to_index(
    repo: &DeltaRepository,
    paths: &Vec<PathBuf>,
    delete: bool,
    skip_missing: bool,
) -> Result<()> {
    rm(paths)?;
    let worktree = &repo.worktree;
    let mut clear_paths = HashSet::<PathBuf>::new();

    for path in paths {
        let absolute_path = fs::canonicalize(path)?;
        if !absolute_path.starts_with(worktree) && absolute_path.is_file() {
            return Err(anyhow!(
                "Not a file, or outside of worktree {}",
                absolute_path.display()
            ));
        }

        let relative_path = absolute_path.strip_prefix(worktree)?;
        clear_paths.insert(relative_path.to_path_buf());
        clear_paths.insert(absolute_path);
    }
    Ok(())
}
