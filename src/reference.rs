use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Context, Result};

use crate::repo::DeltaRepository;

pub enum RefEntry {
    Direct(String),
    Indirect(HashMap<String, RefEntry>),
}

pub fn ref_resolve(repo: &DeltaRepository, reference: &str) -> Result<Option<String>> {
    let path = repo
        .repo_file(&[&reference], false)
        .with_context(|| format!("Failed to locate ref '{}'", reference))?;

    if !path.is_file() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)?;
    let data = raw.trim_end();

    if data.starts_with("ref: ") {
        ref_resolve(repo, &data[5..])
    } else {
        Ok(Some(data.to_string()))
    }
}

pub fn ref_list(repo: &DeltaRepository, path: Option<&Path>) -> Result<HashMap<String, RefEntry>> {
    let dir = repo.repo_dir(&["refs"], false)?;
    let path = path.unwrap_or(&dir);
    let mut res = HashMap::new();

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let path = entry.path();

        let filename = entry.file_name();
        let filename = filename
            .to_str()
            .ok_or(anyhow!("Error converting filename to str"))?;

        if path.is_dir() {
            let children = ref_list(&repo, Some(&path))?;
            res.insert(filename.to_string(), RefEntry::Indirect(children));
        } else {
            let s = path
                .to_str()
                .ok_or(anyhow!("Error converting path to str"))?;
            if let Some(data) = ref_resolve(&repo, s)? {
                res.insert(filename.to_string(), RefEntry::Direct(data));
            }
        }
    }

    Ok(res)
}
