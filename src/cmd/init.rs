use std::path::PathBuf;

use crate::repo::DeltaRepository;

pub fn init(path: PathBuf) {
    match DeltaRepository::new(&path, true) {
        Ok(repo) => match repo.repo_create(&path) {
            Ok(_) => println!("Initialised empty delta repo at: {}", path.display()),
            Err(e) => eprintln!("Failed to create repo: {}", e),
        },
        Err(e) => eprintln!("Failed to initialise repo: {}", e),
    }
}
