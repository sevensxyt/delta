use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use glob::Pattern;

use crate::{
    ignore::{DeltaIgnore, PatternRule},
    repo::DeltaRepository,
};

pub fn check_ignore(paths: Vec<PathBuf>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = DeltaRepository::repo_find(cwd)?;
    let rules = DeltaIgnore::deltaignore_read(&repo)?;

    for path in paths {
        if let Some(true) = has_ignore(&rules, &path)? {
            println!("{}", path.display());
        }
    }

    Ok(())
}

fn has_ignore(rules: &DeltaIgnore, path: &Path) -> Result<Option<bool>> {
    if path.is_absolute() {
        return Err(anyhow!(
            "Path needs to be relative to delta repository's root"
        ));
    }

    if let Some(result) = check_ignore_scoped(&rules.scoped, path)? {
        return Ok(Some(result));
    }

    if let Some(result) = check_ignore_absolute(&rules.absolute, path)? {
        return Ok(Some(result));
    }

    Ok(None)
}

fn check_ignore_scoped(
    rules: &HashMap<String, Vec<PatternRule>>,
    path: &Path,
) -> Result<Option<bool>> {
    fn get_parent(path: &Path) -> Result<&Path> {
        path.parent()
            .ok_or_else(|| anyhow!("No parent found for path {}", path.display()))
    }

    fn to_key(path: &Path) -> Result<String> {
        Ok(path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in path {}", path.display()))?
            .to_string())
    }

    let mut parent = get_parent(path)?;

    loop {
        let key = to_key(parent)?;
        if let Some(rules) = rules.get(&key) {
            if let Some(result) = check_ignore_one(rules, path)? {
                return Ok(Some(result));
            }
        }

        if key.is_empty() {
            break;
        }

        parent = get_parent(parent)?;
    }

    Ok(None)
}

fn check_ignore_absolute(rules: &Vec<Vec<PatternRule>>, path: &Path) -> Result<Option<bool>> {
    for ruleset in rules {
        if let Some(result) = check_ignore_one(ruleset, path)? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

fn check_ignore_one(rules: &[PatternRule], path: &Path) -> Result<Option<bool>> {
    let path = path
        .to_str()
        .ok_or_else(|| anyhow!("Error converting path {} to &str", path.display()))?;

    Ok(rules
        .iter()
        .filter(|rule| Pattern::new(&rule.pattern).is_ok_and(|p| p.matches(path)))
        .map(|rule| rule.exclude)
        .next_back())
}
