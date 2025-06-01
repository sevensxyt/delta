use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};

use crate::{index::DeltaIndex, object::DeltaObject, repo::DeltaRepository};

pub struct PatternRule {
    pub pattern: String,
    pub exclude: bool,
}

pub struct DeltaIgnore {
    pub absolute: Vec<Vec<PatternRule>>,
    pub scoped: HashMap<String, Vec<PatternRule>>,
}

impl DeltaIgnore {
    pub fn deltaignore_read(repo: &DeltaRepository) -> Result<Self> {
        let mut absolute = Vec::new();
        let mut scoped = HashMap::new();

        let repo_file = repo.deltadir.join("info/exclude");
        if repo_file.exists() {
            let file = File::open(repo_file)?;
            absolute.push(deltaignore_parse(file)?);
        }

        let config_home = if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_home)
        } else {
            let home = env::var("HOME")?;
            PathBuf::from(home).join(".config")
        };

        let global_file = config_home.join("delta/ignore");
        if global_file.exists() {
            let file = File::open(global_file)?;
            absolute.push(deltaignore_parse(file)?);
        }

        let index = DeltaIndex::read_index(repo)?;
        for e in index.entries {
            if e.name == "deltaignore" || e.name.ends_with("/.deltaignore") {
                let dir_name = Path::new(&e.name)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or_default();
                let obj = repo
                    .object_read(&e.sha)?
                    .ok_or(anyhow!("Object with sha {} not found", e.sha))?;

                if let DeltaObject::Blob(blob) = obj {
                    let k = dir_name.to_string();
                    let v = deltaignore_parse(&blob.data[..])?;
                    scoped.insert(k, v);
                } else {
                    return Err(anyhow!("Expected blob, found {}", obj.format()));
                }
            }
        }

        Ok(DeltaIgnore { absolute, scoped })
    }

    // pub fn ignore_exists(&self, path: PathBuf) -> {

    // }

    // fn check_ignore_one(&self, path: PathBuf) -> {

    // }
}

fn deltaignore_parse<R: Read>(source: R) -> Result<Vec<PatternRule>> {
    let reader = BufReader::new(source);
    let mut ret = vec![];

    for line in reader.lines() {
        let line = line?;
        if let Some(parsed) = deltaignore_parse_one(&line) {
            ret.push(parsed);
        }
    }

    Ok(ret)
}

fn deltaignore_parse_one(raw: &str) -> Option<PatternRule> {
    let raw = raw.trim();

    if raw.is_empty() || raw.starts_with('#') {
        None
    } else {
        match raw.chars().next()? {
            '!' => Some(PatternRule {
                pattern: raw[1..].to_string(),
                exclude: false,
            }),
            '\\' => Some(PatternRule {
                pattern: raw[1..].to_string(),
                exclude: true,
            }),
            _ => Some(PatternRule {
                pattern: raw.to_string(),
                exclude: true,
            }),
        }
    }
}
