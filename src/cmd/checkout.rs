use std::{error::Error, fs, path::PathBuf};

use crate::{
    kvlm::KvlmKey,
    object::{DeltaObjectEnum, DeltaTree},
    repo::DeltaRepository,
};

pub fn checkout(commit: String, path: PathBuf) -> Result<(), Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    let repo = DeltaRepository::repo_find(cwd)?;
    let Some(mut obj) = repo.object_read(&commit)? else {
        return Err(format!("Object with commit '{}' not found", commit).into());
    };

    if let DeltaObjectEnum::Commit(commit_obj) = obj {
        let tree_sha = commit_obj
            .data
            .get(KvlmKey::Tree.as_str())
            .and_then(|v| v.first())
            .ok_or(format!("Commit {} does not have a tree", commit))?;

        let tree_sha = String::from_utf8_lossy(tree_sha);
        obj = repo.object_read(&tree_sha)?.ok_or("Tree not found")?;
    }

    if path.exists() {
        if !path.is_dir() {
            return Err(format!("Not a directory {}", path.display()).into());
        }

        if fs::read_dir(&path)?.next().is_some() {
            return Err(format!("Not empty {}", path.display()).into());
        }
    } else {
        fs::create_dir_all(&path)?;
    }

    // tree_checkout(&repo, obj, path);
    Ok(())
}

fn tree_checkout(
    repo: &DeltaRepository,
    tree: DeltaTree,
    path: PathBuf,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
