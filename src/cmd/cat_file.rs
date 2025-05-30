use anyhow::{anyhow, Context, Result};

use crate::object::ObjectFormat;
use crate::repo::DeltaRepository;
use std::{env, io::Write};

pub fn cat_file(object: String, format: String) -> Result<()> {
    let cwd = env::current_dir().context("Error getting cwd")?;
    let repo = DeltaRepository::repo_find(cwd)?;

    let format = ObjectFormat::from_bytes(format.as_bytes())?;
    let sha = repo
        .object_find(&object, Some(format), true)?
        .ok_or(anyhow!("Object not found"))?;
    let obj = repo.object_read(&sha)?.context("Object not found")?;
    let data = obj.serialise()?;

    std::io::stdout().write_all(&data)?;
    Ok(())
}
