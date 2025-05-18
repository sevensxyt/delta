use crate::kvlm::KvlmKey;
use crate::object::{DeltaCommit, ObjectFormat};
use crate::repo::DeltaRepository;
use std::{collections::HashSet, env, error::Error};

pub fn log(commit: String) -> Result<(), Box<dyn Error>> {
    let repo = DeltaRepository::repo_find(env::current_dir()?)?;
    let object = repo.object_find(&commit, ObjectFormat::Commit, true)?;
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
            .get(KvlmKey::Message.as_str())
            .map(|m| {
                let raw = m
                    .iter()
                    .map(|v| String::from_utf8_lossy(v))
                    .collect::<String>();

                raw.strip_suffix('\n')
                    .unwrap_or(&raw)
                    .trim()
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
            })
            .unwrap_or_default();

        println!(" c_{} [label=\"{}: {}\"]", sha, &sha[0..7], message);
        match commit.data.get(KvlmKey::Parent.as_str()) {
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
