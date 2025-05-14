use crate::repo::DeltaRepository;
use std::path::PathBuf;

pub fn kill(path: PathBuf) {
    let repo = match DeltaRepository::repo_find(path) {
        Ok(repo) => repo,
        Err(e) => {
            eprintln!("Error finding delta repo: {}", e);
            return;
        }
    };

    let path = repo.deltadir;
    match std::fs::remove_dir_all(&path) {
        Ok(_) => println!("Successfully deleted delta repo at: {}", path.display()),
        Err(e) => eprintln!("Error deleting delta repo at: {}", e),
    }
}
