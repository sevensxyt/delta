use std::{env, error::Error, io::Write};

use crate::repo::DeltaRepository;

pub fn cat_file(object: String, format: String) {
    let cwd = env::current_dir().expect("Errored when getting cwd");
    let repo = match DeltaRepository::repo_find(cwd, true) {
        Ok(Some(repo)) => repo,
        Ok(None) => {
            eprintln!("Not a delta repo");
            return;
        }
        Err(e) => {
            eprintln!("Error finding repo {}", e);
            return;
        }
    };

    let sha = repo.object_find(object, format, true);
    let obj = match repo.object_read(&sha) {
        Ok(Some(object)) => object,
        Ok(None) => {
            eprintln!("Error finding object");
            return;
        }
        Err(e) => {
            eprintln!("Error reading object {}", e);
            return;
        }
    };
    let serialised = obj.serialise();
    let data = match serialised {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error serialising object {}", e);
            return;
        }
    };

    if let Err(e) = std::io::stdout().write_all(&data) {
        eprintln!("Error writing to stdout {}", e)
    }
}
