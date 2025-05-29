use crate::repo::DeltaRepository;
use anyhow::Result;
use std::path::PathBuf;

pub fn init(path: PathBuf) -> Result<()> {
    let repo = DeltaRepository::new(&path, true)?;
    repo.repo_create(&path)?;
    println!("Initialised empty delta repo at: {}", path.display());
    Ok(())
}
