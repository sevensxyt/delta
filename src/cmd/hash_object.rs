use crate::repo::DeltaRepository;
use std::path::PathBuf;

pub fn hash_object(path: PathBuf, format: String, write: bool) {
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading data at {}: {}", path.display(), e);
            return;
        }
    };

    let sha = match DeltaRepository::object_hash(data, &format, write) {
        Ok(sha) => sha,
        Err(e) => {
            eprintln!("Error hashing object: {}", e);
            return;
        }
    };

    println!("{}", sha)
}
