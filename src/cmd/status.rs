use std::{
    collections::HashMap,
    fs,
    os::macos::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use walkdir::WalkDir;

use crate::{
    cmd::check_ignore::has_ignore,
    ignore::DeltaIgnore,
    index::DeltaIndex,
    object::{DeltaObject, ObjectFormat},
    repo::DeltaRepository,
};

pub fn status() -> Result<()> {
    let repo = DeltaRepository::repo_find(std::env::current_dir()?)?;
    let index = DeltaIndex::read_index(&repo)?;

    branch_status(&repo)?;
    head_index_status(&repo, &index)?;
    println!();
    status_index_worktree(&repo, &index)?;
    Ok(())
}

fn get_active_branch(repo: &DeltaRepository) -> Result<Option<String>> {
    let path = repo.repo_file(&["HEAD"], false)?;

    Ok(fs::read_to_string(path)?
        .strip_prefix("ref: refs/heads/")
        .map(|s| s.trim().to_string()))
}

fn branch_status(repo: &DeltaRepository) -> Result<()> {
    if let Some(branch) = get_active_branch(repo)? {
        println!("On branch {}.", branch);
    } else {
        let sha = repo
            .object_find("HEAD", None, true)?
            .ok_or_else(|| anyhow!("Error finding object for detached HEAD"))?;
        println!("HEAD detached at {}", sha);
    }

    Ok(())
}

fn head_index_status(repo: &DeltaRepository, index: &DeltaIndex) -> Result<()> {
    println!("Changes to be commited:");
    let mut head = tree_to_dict(repo, "HEAD", None)?;

    for entry in &index.entries {
        if let Some(sha) = head.get(&entry.name) {
            if sha != &entry.sha {
                println!("\tmodified:{}", entry.name);
            }
            head.remove(&entry.name);
        } else {
            println!("\tadded:\t{}", entry.name);
        }
    }

    for entry in head.keys() {
        println!("\tdeleted: {}", entry);
    }
    Ok(())
}

fn tree_to_dict(
    repo: &DeltaRepository,
    reference: &str,
    prefix: Option<&Path>,
) -> Result<HashMap<String, String>> {
    let prefix = prefix.unwrap_or(Path::new(""));
    let mut res = HashMap::<String, String>::new();

    let sha = repo
        .object_find(reference, Some(ObjectFormat::Tree), true)?
        .ok_or_else(|| anyhow!("Tree not found for reference {}", reference))?;

    let tree = match repo
        .object_read(&sha)?
        .ok_or_else(|| anyhow!("Cannot read object with sha {}", sha))?
    {
        DeltaObject::Tree(tree) => tree,
        other => return Err(anyhow!("Expected tree, got {}", other.format())),
    };

    for leaf in tree.items()? {
        let path = prefix.join(leaf.path);
        let is_subtree = leaf.mode.starts_with(b"04");

        if is_subtree {
            res.extend(tree_to_dict(repo, reference, Some(prefix))?);
        } else {
            res.insert(path.to_string_lossy().to_string(), leaf.sha);
        }
    }

    Ok(res)
}

fn status_index_worktree(repo: &DeltaRepository, index: &DeltaIndex) -> Result<()> {
    println!("Changes not staged for commit:");

    let ignore = DeltaIgnore::deltaignore_read(repo)?;
    let mut files = Vec::<PathBuf>::new();
    let worktree = &repo.worktree;

    for entry in WalkDir::new(&repo.worktree) {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && path.starts_with(&repo.deltadir) {
            continue;
        }

        if path.is_file() {
            let absolute_path = fs::canonicalize(path)?;
            let relative_path = absolute_path.strip_prefix(worktree)?;

            files.push(relative_path.to_path_buf());
            files.push(absolute_path);
        }
    }

    for e in &index.entries {
        let path = worktree.join(&e.name);

        if !path.exists() {
            println!("\tdeleted:\t{}", e.name);
        } else {
            let metadata = fs::metadata(&path)?;

            let ctime_ns = e.ctime.0 * u32::pow(10, 9) * e.ctime.1;
            let mtime_ns = e.mtime.0 * u32::pow(10, 9) * e.mtime.1;

            if metadata.st_ctime_nsec() != ctime_ns.into()
                || metadata.st_mtime_nsec() != mtime_ns.into()
            {
                let data = fs::read(&path)?;
                let sha = DeltaRepository::object_hash(data, "blob", false)?;

                if e.sha != sha {
                    println!("\tmodified:\t{}", e.name);
                }
            }
        }

        let relative_path = fs::canonicalize(path)?
            .strip_prefix(worktree)?
            .to_path_buf();

        if let Some(i) = files.iter().position(|p| p == &relative_path) {
            files.remove(i);
        }
    }

    println!("\nUntracked files:");

    for file in files {
        if has_ignore(&ignore, &file)?.unwrap_or_default() {
            println!("{}", file.display());
        }
    }

    Ok(())
}
