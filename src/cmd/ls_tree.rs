use std::error::Error;
use std::path::PathBuf;

use crate::object::{DeltaObjectEnum, ObjectFormat};
use crate::repo::DeltaRepository;

pub fn ls_tree(tree: String, recursive: bool) -> Result<(), Box<dyn Error>> {
    let repo = DeltaRepository::repo_find(std::env::current_dir()?)?;
    recurse(&repo, &tree, recursive, PathBuf::from(String::new()))?;

    Ok(())
}

fn recurse(
    repo: &DeltaRepository,
    tree: &str,
    recursive: bool,
    prefix: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let sha = repo.object_find(tree, ObjectFormat::Tree, true)?;
    let DeltaObjectEnum::Tree(obj) = repo.object_read(&sha)?.ok_or("Error: Object not found")?
    else {
        return Err("Object is not a tree".into());
    };

    for item in obj.items()? {
        let mode_type = &item.mode[..2];
        let obj_format = match mode_type {
            b"04" => ObjectFormat::Tree,
            b"10" | b"12" => ObjectFormat::Blob,
            b"16" => ObjectFormat::Commit,
            _ => return Err(format!("Invalid type: {:?}", mode_type).into()),
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
