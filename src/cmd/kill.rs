use anyhow::Result;

use crate::repo::DeltaRepository;
use std::{io, path::PathBuf};

pub fn kill(path: PathBuf) -> Result<()> {
    let repo = DeltaRepository::find_repo(path)?;
    let deltadir = repo.deltadir.canonicalize().unwrap_or(repo.deltadir);
    let display_path = deltadir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(deltadir.clone());

    println!(
        "Are you sure you wanted to delete your repo at {}? [y/n]: ",
        display_path.display()
    );

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        std::fs::remove_dir_all(&deltadir)?;
        println!("Removed repository at {}", display_path.display())
    }

    Ok(())
}
