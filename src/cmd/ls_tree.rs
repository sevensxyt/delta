use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::object::{DeltaObject, ObjectFormat};
use crate::repo::DeltaRepository;

pub fn ls_tree(tree: String, recursive: bool) -> Result<()> {
    let repo = DeltaRepository::find_repo(std::env::current_dir()?)?;
    recurse(&repo, &tree, recursive, PathBuf::from(String::new()))?;

    Ok(())
}

fn recurse(repo: &DeltaRepository, tree: &str, recursive: bool, prefix: PathBuf) -> Result<()> {
    let sha = repo
        .find_object(tree, Some(ObjectFormat::Tree), true)?
        .ok_or(anyhow!("Object not found"))?;
    let obj = repo.read_object(&sha)?.context("Error: Object not found")?;

    let DeltaObject::Tree(obj) = obj else {
        return Err(anyhow!("Object is not a tree"));
    };

    for item in obj.items()? {
        let mode_type = &item.mode[..2];
        let obj_format = match mode_type {
            b"04" => ObjectFormat::Tree,
            b"10" | b"12" => ObjectFormat::Blob,
            b"16" => ObjectFormat::Commit,
            _ => return Err(anyhow!("Invalid type: {:?}", mode_type)),
        };

        if recursive && obj_format == ObjectFormat::Tree {
            recurse(repo, tree, recursive, prefix.join(item.path))?;
        } else {
            let mode = std::str::from_utf8(&item.mode)?;
            let path = prefix.join(item.path);
            println!("{} {} {}\t{}", mode, obj_format, sha, path.display());
        }
    }

    Ok(())
}
