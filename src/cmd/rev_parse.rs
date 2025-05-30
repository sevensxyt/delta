use anyhow::{anyhow, Context, Result};

use crate::{object::ObjectFormat, repo::DeltaRepository};

pub fn rev_parse(format: Option<String>, name: String) -> Result<()> {
    let format = match format {
        Some(f) => Some(ObjectFormat::from_bytes(f.as_bytes())?),
        None => None,
    };

    let cwd = std::env::current_dir().context(anyhow!("Error finding current directory"))?;
    let repo = DeltaRepository::repo_find(cwd)?;
    let sha = repo
        .object_find(&name, format, true)?
        .ok_or(anyhow!("Error finding object {}", name))?;

    println!("{}", sha);
    Ok(())
}
