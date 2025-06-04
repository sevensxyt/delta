use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, FixedOffset};
use indexmap::IndexMap;
use ini::configparser::ini::Ini;

use crate::{
    index::{DeltaIndex, DeltaIndexEntry},
    kvlm::KvlmKey,
    object::{tree::DeltaTreeLeaf, DeltaCommit, DeltaObject, DeltaTree},
    repo::DeltaRepository,
};

pub fn commit(message: String) -> Result<()> {
    let repo = DeltaRepository::find_repo(std::env::current_dir()?)?;
    let index = DeltaIndex::read_index(&repo)?;
    let tree = tree_from_index(&repo, &index)?;
    let parent = repo.find_object("HEAD", None, true)?;
    let config = read_config()?;
    let author = get_config_user(config).ok_or_else(|| anyhow!("User not found in config"))?;
    let offset = chrono::Local::now().offset().utc_minus_local();
    let timestamp = chrono::Local::now().with_timezone(&FixedOffset::east_opt(offset).unwrap());

    let _sha = create_commit(&repo, &tree, parent.as_deref(), &author, timestamp, message)?;

    Ok(())
}

enum EntryOrTree {
    Entry(DeltaIndexEntry),
    Tree(String, String),
}

fn create_commit(
    repo: &DeltaRepository,
    tree: &str,
    parent: Option<&str>,
    author: &str,
    timestamp: DateTime<FixedOffset>,
    message: String,
) -> Result<String> {
    let mut data = IndexMap::<String, Vec<Vec<u8>>>::new();
    data.insert(KvlmKey::Tree.to_string(), vec![tree.into()]);

    if let Some(commit) = parent {
        data.insert(KvlmKey::Parent.to_string(), vec![commit.into()]);
    }

    let message = format!("{}\n", message.trim());
    let author = format_author_with_timezone(author, timestamp);

    data.insert(KvlmKey::Author.to_string(), vec![author.clone().into()]);
    data.insert(KvlmKey::Committer.to_string(), vec![author.into()]);
    data.insert(KvlmKey::Message.to_string(), vec![message.into()]);

    let sha = repo.write_object(&DeltaObject::Commit(DeltaCommit { data }))?;

    Ok(sha)
}

fn format_author_with_timezone(author: &str, timestamp: DateTime<FixedOffset>) -> String {
    let offset_seconds = timestamp.offset().utc_minus_local();

    let sign = if offset_seconds >= 0 { '+' } else { '-' };
    let abs_offset = offset_seconds.abs();
    let hours = abs_offset / 3600;
    let minutes = (abs_offset % 3600) / 60;

    let timestamp_str = format!(" {} ", timestamp.timestamp());
    let timezone = format!("{}{:02}{:02}", sign, hours, minutes);

    format!("{}{}{}", author, timestamp_str, timezone)
}

fn tree_from_index(repo: &DeltaRepository, index: &DeltaIndex) -> Result<String> {
    let mut contents = HashMap::<String, Vec<EntryOrTree>>::new();
    contents.insert("".to_string(), vec![]);

    for e in &index.entries {
        let path = PathBuf::from(&e.name);
        let mut dir_name = path.parent().unwrap_or(Path::new(""));
        let mut key = dir_name.display().to_string();
        let parent = key.clone();

        while !key.is_empty() {
            if !contents.contains_key(&key) {
                contents.insert(key.clone(), vec![]);
            }

            dir_name = dir_name.parent().unwrap_or(Path::new(""));
            key = dir_name.display().to_string();
        }

        if let Some(vec) = contents.get_mut(&parent) {
            vec.push(EntryOrTree::Entry(e.clone()));
        }
    }

    let mut sorted_paths: Vec<String> = contents.keys().cloned().collect();
    sorted_paths.sort_by_key(|k| std::cmp::Reverse(k.len()));
    let mut sha = String::from("");

    for path in &sorted_paths {
        let entries = contents
            .get(path)
            .ok_or_else(|| anyhow!("Failed to find value with key {}", path))?;
        let mut leaves: Vec<DeltaTreeLeaf> = vec![];

        for entry in entries {
            let leaf = match entry {
                EntryOrTree::Entry(entry) => {
                    let leaf_mode =
                        format!("{:02o}{:04o}", entry.mode_type.to_bytes(), entry.mode_perms);
                    DeltaTreeLeaf {
                        mode: leaf_mode.as_bytes().try_into()?,
                        path: PathBuf::from(path),
                        sha: entry.sha.clone(),
                    }
                }
                EntryOrTree::Tree(path, sha) => DeltaTreeLeaf {
                    mode: b"040000".to_owned(),
                    path: path.into(),
                    sha: sha.into(),
                },
            };

            leaves.push(leaf);
        }

        let tree = DeltaObject::Tree(DeltaTree::from_leaves(leaves)?);
        sha = repo.write_object(&tree)?;

        let path = PathBuf::from(path);
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("Error getting parent from {}", path.display()))?
            .to_string_lossy()
            .to_string();
        let base = path
            .file_name()
            .ok_or_else(|| anyhow!("Error getting base name from {}", path.display()))?
            .to_string_lossy()
            .to_string();

        let vec = contents
            .get_mut(&parent)
            .ok_or_else(|| anyhow!("Vector not found with key {}", parent))?;
        let value = EntryOrTree::Tree(base, sha.clone());
        vec.push(value);
    }

    Ok(sha)
}

fn read_config() -> Result<Ini> {
    let xdg_config_home =
        PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or("~/.config".to_string()));
    let home = PathBuf::from(env::var("HOME")?);

    let xdg_config = home.join(xdg_config_home).join("delta/config");
    let home_config = home.join("~/.deltaconfig");
    let mut config = Ini::new();

    for path in [xdg_config, home_config] {
        if path.exists() {
            let _ = config.load(&path.display().to_string());
        }
    }

    Ok(config)
}

fn get_config_user(config: Ini) -> Option<String> {
    let user = config.get("", "user")?;
    let name = config.get(&user, "name")?;
    let email = config.get(&user, "email")?;

    Some(format!("{} <{}>", name, email))
}
