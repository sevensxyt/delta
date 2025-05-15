use crate::kvlm;
use crate::object::DeltaCommit;
use crate::repo::DeltaRepository;
use std::error::Error;
use std::{collections::HashSet, env};

pub fn log(commit: String) -> Result<(), Box<dyn Error>> {
    let cwd = env::current_dir()?;
    let repo = DeltaRepository::repo_find(cwd)?;
    let object = repo.object_find(commit, "dummy".into(), true);
    println!("}}");
    Ok(())
}

fn log_graph(repo: DeltaRepository, sha: String) {
    fn recurse(
        repo: DeltaRepository,
        sha: String,
        mut seen: HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        if seen.contains(&sha) {
            return Ok(());
        }

        seen.insert(sha.clone());

        let object = repo.object_read(&sha)?.ok_or("Object not found")?;
        let commit: DeltaCommit = object.try_into()?;
        let message = commit.data.get(kvlm::MESSAGE_KEY);

        Ok(())
    }

    if let Err(e) = recurse(repo, sha, HashSet::new()) {
        eprintln!("Error printing logs: {}", e);
    }
}
