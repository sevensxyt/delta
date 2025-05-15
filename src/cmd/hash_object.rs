use crate::repo::DeltaRepository;
use std::{error::Error, path::PathBuf};

pub fn hash_object(path: PathBuf, format: String, write: bool) -> Result<(), Box<dyn Error>> {
    let data = std::fs::read(&path)?;
    let sha = DeltaRepository::object_hash(data, &format, write)?;
    println!("{}", sha);
    Ok(())
}
