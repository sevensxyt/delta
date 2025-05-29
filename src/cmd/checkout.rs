use std::{error::Error, fs, path::PathBuf};

use crate::{
    kvlm::KvlmKey,
    object::{DeltaObject, DeltaTree},
    repo::DeltaRepository,
};

pub fn checkout(commit: String, path: PathBuf) -> Result<(), Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    let repo = DeltaRepository::repo_find(cwd)?;
    let Some(obj) = repo.object_read(&commit)? else {
        return Err(format!("Object with commit '{}' not found", commit).into());
    };

    let tree = if let DeltaObject::Commit(commit_obj) = obj {
        let tree_sha = commit_obj
            .data
            .get(KvlmKey::Tree.as_str())
            .and_then(|v| v.first())
            .ok_or(format!("Commit {} does not have a tree", commit))?;

        let tree_sha = String::from_utf8_lossy(tree_sha);
        let tree_obj = repo.object_read(&tree_sha)?.ok_or("Tree not found")?;

        match tree_obj {
            DeltaObject::Tree(tree) => tree,
            other => return Err(format!("Expected tree, got {}", other.format()).into()),
        }
    } else {
        match obj {
            DeltaObject::Tree(tree) => tree,
            other => return Err(format!("Expected commit or tree, got {}", other.format()).into()),
        }
    };

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

    tree_checkout(&repo, tree, path)?;
    Ok(())
}

fn tree_checkout(
    repo: &DeltaRepository,
    tree: DeltaTree,
    path: PathBuf,
) -> Result<(), Box<dyn Error>> {
    for item in tree.items()? {
        let obj = repo
            .object_read(&item.sha)?
            .ok_or(format!("Cannot find item with sha {}", item.sha))?;

        let dest = path.join(item.path);

        match obj {
            DeltaObject::Tree(tree) => {
                fs::create_dir(&dest)?;
                tree_checkout(repo, tree, dest)?
            }
            DeltaObject::Blob(blob) => fs::write(dest, blob.data)?,
            other => {
                return Err(format!("Unable to checkout object of type {}", other.format()).into())
            }
        }
    }
    Ok(())
}
