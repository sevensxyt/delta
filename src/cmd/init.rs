use crate::repo::DeltaRepository;
use clap::error::Result;
use std::{error::Error, path::PathBuf};

pub fn init(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let repo = DeltaRepository::new(&path, true)?;
    repo.repo_create(&path)?;
    println!("Initialised empty delta repo at: {}", path.display());
    Ok(())
}
