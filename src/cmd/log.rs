use crate::kvlm;
use crate::object::DeltaCommit;
use crate::repo::DeltaRepository;
use std::{collections::HashSet, env, error::Error};

pub fn log(commit: String) -> Result<(), Box<dyn Error>> {
    let repo = DeltaRepository::repo_find(env::current_dir()?)?;
    let object = repo.object_find(commit, "dummy".into(), true);
    log_graph(&repo, object);
    Ok(())
}

fn log_graph(repo: &DeltaRepository, sha: String) {
    fn recurse(
        repo: &DeltaRepository,
        sha: &str,
        seen: &mut HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        if !seen.insert(sha.to_string()) {
            return Ok(());
        }

        let object = repo.object_read(&sha)?.ok_or("Object not found")?;
        let commit: DeltaCommit = object.try_into()?;
        let message = commit
            .data
            .get(kvlm::MESSAGE_KEY)
            .map_or(String::new(), |m| {
                m.iter()
                    .map(|v| String::from_utf8_lossy(v))
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .strip_suffix("\n")
                    .unwrap_or("")
                    .into()
            });

        println!(" c_{} [label=\"{}: {}\"]", sha, &sha[0..7], message);
        match commit.data.get(kvlm::PARENT_KEY) {
            None => return Ok(()),
            Some(parents) => {
                for p in parents {
                    let p = String::from_utf8_lossy(p).to_string();
                    println!(" c_{} -> c_{}", sha, p);
                    recurse(repo, &p, seen)?;
                }
            }
        }

        Ok(())
    }

    if let Err(e) = recurse(&repo, &sha, &mut HashSet::new()) {
        eprintln!("Error printing logs: {}", e);
    }
}
