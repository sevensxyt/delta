use crate::repo::DeltaRepository;
use std::{env, path::PathBuf};

pub fn hash_object(path: PathBuf, format: String, write: bool) {
    let repo = if write {
        let cwd = match env::current_dir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Error reading cwd {}", e);
                return;
            }
        };

        let repo = match DeltaRepository::repo_find(cwd, true) {
            Ok(repo) => repo,
            Err(e) => {
                eprintln!("Error finding repository {}", e);
                return;
            }
        };
        repo
    } else {
        None
    };

    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading data at {} {}", path.display(), e);
            return;
        }
    };
    let sha = match DeltaRepository::object_hash(data, &format, repo, write) {
        Ok(sha) => sha,
        Err(e) => {
            eprintln!("Error hashing object {}", e);
            return;
        }
    };
    println!("{}", sha)
}
