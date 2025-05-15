use crate::repo::DeltaRepository;
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

pub fn kill(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let repo = DeltaRepository::repo_find(path)?;
    let deltadir = repo.deltadir.canonicalize().unwrap_or(repo.deltadir);
    let display_path = deltadir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(deltadir.clone());

    print!(
        "Are you sure you wanted to delete your repo at {}? [y/n]: ",
        display_path.display()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        std::fs::remove_dir_all(&deltadir)?;
        println!("Removed repository at {}", display_path.display())
    } else {
        println!("Aborted");
    }
    Ok(())
}
