use anyhow::{anyhow, Result};

use crate::kvlm::KvlmKey;
use crate::object::{DeltaCommit, ObjectFormat};
use crate::repo::DeltaRepository;
use std::{collections::HashSet, env};

pub fn log(commit: String) -> Result<()> {
    let repo = DeltaRepository::find_repo(env::current_dir()?)?;
    let object = repo
        .find_object(&commit, Some(ObjectFormat::Commit), true)?
        .ok_or(anyhow!("Commit {} not found", commit))?;
    log_graph(&repo, object);
    Ok(())
}

fn log_graph(repo: &DeltaRepository, sha: String) {
    fn recurse(repo: &DeltaRepository, sha: &str, seen: &mut HashSet<String>) -> Result<()> {
        if !seen.insert(sha.to_string()) {
            return Ok(());
        }

        let object = repo.read_object(sha)?.ok_or(anyhow!("Object not found"))?;
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

    if let Err(e) = recurse(repo, &sha, &mut HashSet::new()) {
        eprintln!("Error printing logs: {}", e);
    }
}
