use anyhow::Result;

use crate::{object::ObjectFormat, repo::DeltaRepository};
use std::path::PathBuf;

pub fn hash_object(path: PathBuf, format: String, write: bool) -> Result<()> {
    let data = std::fs::read(&path)?;
    let format = ObjectFormat::from_bytes(&format.into_bytes())?;
    let sha = DeltaRepository::hash_object(data, format, write)?;
    println!("{}", sha);
    Ok(())
}
