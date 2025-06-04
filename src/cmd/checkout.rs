use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::{
    kvlm::KvlmKey,
    object::{DeltaObject, DeltaTree},
    repo::DeltaRepository,
};

pub fn checkout(commit: String, path: PathBuf) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = DeltaRepository::find_repo(cwd)?;
    let Some(obj) = repo.read_object(&commit)? else {
        return Err(anyhow!("Object with commit '{}' not found", commit));
    };

    let tree = if let DeltaObject::Commit(commit_obj) = obj {
        let tree_sha = commit_obj
            .data
            .get(KvlmKey::Tree.as_str())
            .and_then(|v| v.first())
            .ok_or(anyhow!("Commit {} does not have a tree", commit))?;

        let tree_sha = String::from_utf8_lossy(tree_sha);
        let tree_obj = repo.read_object(&tree_sha)?.context("Tree not found")?;

        match tree_obj {
            DeltaObject::Tree(tree) => tree,
            other => return Err(anyhow!("Expected tree, got {}", other.format())),
        }
    } else {
        match obj {
            DeltaObject::Tree(tree) => tree,
            other => return Err(anyhow!("Expected commit or tree, got {}", other.format())),
        }
    };

    if path.exists() {
        if !path.is_dir() {
            return Err(anyhow!("Not a directory {}", path.display()));
        }

        if fs::read_dir(&path)?.next().is_some() {
            return Err(anyhow!("Not empty {}", path.display()));
        }
    } else {
        fs::create_dir_all(&path)?;
    }

    tree_checkout(&repo, tree, path)?;
    Ok(())
}

fn tree_checkout(repo: &DeltaRepository, tree: DeltaTree, path: PathBuf) -> Result<()> {
    for item in tree.items()? {
        let obj = repo
            .read_object(&item.sha)?
            .ok_or(anyhow!("Cannot find item with sha {}", item.sha))?;

        let dest = path.join(item.path);

        match obj {
            DeltaObject::Tree(tree) => {
                fs::create_dir(&dest)?;
                tree_checkout(repo, tree, dest)?
            }
            DeltaObject::Blob(blob) => fs::write(dest, blob.data)?,
            other => {
                return Err(anyhow!(
                    "Unable to checkout object of type {}",
                    other.format()
                ))
            }
        }
    }
    Ok(())
}
