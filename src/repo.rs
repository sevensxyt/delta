use crate::kvlm::kvlm_parse;
use crate::object::{DeltaBlob, DeltaCommit, DeltaObject, DeltaTag, DeltaTree, ObjectFormat};
use crate::reference::ref_resolve;
use anyhow::{anyhow, Context, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use indexmap::IndexMap;
use ini::configparser::ini::Ini;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct DeltaRepository {
    pub worktree: PathBuf,
    pub deltadir: PathBuf,
    pub config: Option<Ini>,
}

pub struct ObjectHash {
    pub sha: String,
    pub payload: Vec<u8>,
}

impl DeltaRepository {
    pub fn new(path: &Path, force: bool) -> Result<Self> {
        let worktree = path.to_path_buf();
        let deltadir = worktree.join(".delta");

        if !force && !deltadir.is_dir() {
            return Err(anyhow!("Not a delta repository: {}", path.display()));
        }

        let config = if force {
            None
        } else {
            let config_path = deltadir.join("config");
            if config_path.exists() {
                let mut ini = Ini::new();
                let config_path = config_path.to_str().ok_or_else(|| {
                    anyhow!(
                        "Error converting config path {} to string",
                        config_path.display()
                    )
                })?;
                ini.load(config_path)
                    .map_err(|e| anyhow!(e))
                    .context(format!(
                        "Error loading ini config from path {}",
                        config_path
                    ))?;
                Some(ini)
            } else {
                return Err(anyhow!("Config is missing"));
            }
        };

        if !force {
            if let Some(ref config) = config {
                match config.get("core", "repositoryformatversion").as_deref() {
                    Some("0") => {}
                    Some(v) => return Err(anyhow!("Unsupported repository format version: {}", v)),
                    None => return Err(anyhow!("Missing repository format version")),
                }
            }
        }

        Ok(Self {
            worktree,
            deltadir,
            config,
        })
    }

    pub fn repo_path(&self, path: &[&str]) -> PathBuf {
        path.iter()
            .fold(self.deltadir.clone(), |acc, p| acc.join(p))
    }

    pub fn repo_file(&self, path: &[&str], mkdir: bool) -> Result<PathBuf> {
        if mkdir {
            let parent_path = &path[..path.len() - 1];
            self.repo_dir(parent_path, mkdir)?;
        }

        Ok(self.repo_path(path))
    }

    pub fn repo_dir(&self, path: &[&str], mkdir: bool) -> Result<PathBuf> {
        let path = self.repo_path(path);

        if path.exists() {
            if path.is_dir() {
                return Ok(path);
            } else {
                return Err(anyhow!(
                    "Path {} exists but is not a directory",
                    path.display()
                ));
            }
        }

        if mkdir {
            fs::create_dir_all(&path)?;
            Ok(path)
        } else {
            Err(anyhow!("Directory does not exist"))
        }
    }

    pub fn repo_create(&self, path: &Path) -> Result<DeltaRepository> {
        let repo = DeltaRepository::new(path, true)?;

        if repo.worktree.exists() {
            if !repo.worktree.is_dir() {
                return Err(anyhow!("{} is not a directory", repo.worktree.display()));
            } else if repo.deltadir.exists() && fs::read_dir(&repo.deltadir)?.next().is_some() {
                return Err(anyhow!("{} is not empty", repo.worktree.display()));
            }
        } else {
            fs::create_dir_all(&repo.worktree)?;
        }

        repo.repo_dir(&["branches"], true)?;
        repo.repo_dir(&["objects"], true)?;
        repo.repo_dir(&["refs", "tags"], true)?;
        repo.repo_dir(&["refs", "heads"], true)?;

        let description_path = repo.repo_file(&["description"], true)?;
        fs::write(
            description_path,
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        )?;

        let head_path = repo.repo_file(&["HEAD"], false)?;
        fs::write(head_path, "ref: refs/heads/master\n")?;

        let config_path = repo.repo_file(&["config"], false)?;
        let config = self.repo_default_config();
        config.write(
            config_path
                .to_str()
                .ok_or(anyhow!("Failed converting config path to string"))?,
        )?;

        Ok(repo)
    }

    pub fn repo_default_config(&self) -> Ini {
        let mut config = Ini::new();

        config.setstr("core", "repositoryformatversion", Some("0"));
        config.setstr("core", "filemode", Some("false"));
        config.setstr("core", "bare", Some("false"));

        config
    }

    pub fn repo_find_optional(path: PathBuf) -> Result<Option<Self>> {
        let path = path
            .canonicalize()
            .context(format!("No such file exists {}", path.display()))?;

        if path.join(".delta").exists() {
            return Ok(Some(Self::new(&path, false)?));
        }

        let parent = path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("No delta directory found"))?;

        Self::repo_find_optional(parent)
    }

    pub fn repo_find(path: PathBuf) -> Result<Self> {
        Self::repo_find_optional(path)?.ok_or_else(|| anyhow!("No delta repository found"))
    }

    pub fn object_read(&self, sha: &str) -> Result<Option<DeltaObject>> {
        let path = self.repo_file(&["objects", &sha[0..2], &sha[2..]], false)?;

        if !path.is_file() {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        let mut compressed = Vec::new();
        file.read_to_end(&mut compressed)?;

        let raw = Self::decompress(&compressed)?;
        let ascii_space_index = match raw.iter().position(|&b| b == b' ') {
            Some(i) => i,
            None => return Err(anyhow!("Invalid header: Missing ASCII space")),
        };
        let format = &raw[0..ascii_space_index];

        let null_byte_index = match raw.iter().position(|&b| b == 0) {
            Some(i) => i,
            None => return Err(anyhow!("Invalid header: Missing null byte")),
        };

        let s = std::str::from_utf8(&raw[ascii_space_index + 1..null_byte_index])?;
        if s.parse::<usize>()? != raw.len() - null_byte_index - 1 {
            return Err(anyhow!("Malformed object {} bad length", sha));
        }

        let content = &raw[null_byte_index + 1..];
        let format = ObjectFormat::from_bytes(format)?;
        let mut object = match format {
            ObjectFormat::Commit => DeltaObject::Commit(DeltaCommit {
                data: IndexMap::new(),
            }),
            ObjectFormat::Tree => DeltaObject::Tree(DeltaTree { data: vec![] }),
            ObjectFormat::Tag => DeltaObject::Tag(DeltaTag {
                data: IndexMap::new(),
            }),
            ObjectFormat::Blob => DeltaObject::Blob(DeltaBlob { data: vec![] }),
        };
        object.deserialise(content)?;

        Ok(Some(object))
    }

    pub fn object_write(&self, object: &DeltaObject) -> Result<()> {
        let ObjectHash { sha, payload } = Self::compute_object_hash(object)?;
        let path = self.repo_file(&["objects", &sha[0..2], &sha[2..]], true)?;
        if !path.exists() {
            let mut buffer = Vec::new();
            let mut z = ZlibEncoder::new(&mut buffer, Compression::default());
            z.write_all(&payload)?;
            z.finish()?;
            std::fs::write(path, buffer)?;
        }

        Ok(())
    }

    pub fn object_find(
        &self,
        name: &str,
        format: Option<ObjectFormat>,
        follow: bool,
    ) -> Result<Option<String>> {
        let sha = self
            .object_resolve(name)?
            .context(anyhow!("No such reference {}", name))?;

        if sha.len() > 1 {
            let candidates = sha.join("\n - ");
            return Err(anyhow!(
                "Ambiguous reference {}: Candidates are:\n - {}.",
                name,
                candidates
            ));
        }

        let mut sha = sha
            .first()
            .context(anyhow!("No such reference {}", name))?
            .to_string();

        if let Some(format) = format {
            loop {
                let Some(obj) = self.object_read(&sha)? else {
                    return Err(anyhow!("No object found with sha {}", sha));
                };

                if obj.format() == format {
                    return Ok(Some(sha.to_string()));
                }

                if !follow {
                    return Ok(None);
                }

                if let DeltaObject::Tag(obj) = obj {
                    let raw = obj
                        .data
                        .get("object")
                        .and_then(|v| v.first())
                        .ok_or(anyhow!("Tag object missing 'object' key"))?;

                    sha = String::from_utf8_lossy(raw).to_string();
                } else if let DeltaObject::Commit(obj) = obj {
                    if format == ObjectFormat::Tree {
                        let raw = obj
                            .data
                            .get("tree")
                            .and_then(|v| v.first())
                            .ok_or(anyhow!("Commit object missing 'tree' key"))?;

                        sha = String::from_utf8_lossy(raw).to_string()
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            }
        } else {
            Ok(Some(sha.to_string()))
        }
    }

    pub fn object_resolve(&self, name: &str) -> Result<Option<Vec<String>>> {
        if name.is_empty() {
            return Ok(None);
        }

        if name == "HEAD" {
            let reference = ref_resolve(self, "HEAD")?.context("Error finding head")?;
            return Ok(Some(vec![reference]));
        }

        let mut candidates: Vec<String> = vec![];
        let hash_re = Regex::new(r"^[0-9A-Fa-f]{4,40}$")?;

        if name.len() < 4 {
            return Err(anyhow!("Hash must have length of atleast 4"));
        }

        if hash_re.is_match(name) {
            if name.len() == 40 {
                candidates.push(name.to_string());
                return Ok(Some(candidates));
            }

            let name = name.to_lowercase();
            let (prefix, rest) = &name.split_at(2);
            let path = self.repo_dir(&["objects", prefix], false)?;

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let path = file_name.to_string_lossy();

                if path.starts_with(rest) {
                    candidates.push(format!("{}{}", prefix, path))
                }
            }
        }

        for namespace in ["tags", "heads", "remotes"] {
            if let Some(s) = ref_resolve(self, &format!("refs/{}/{}", namespace, name))? {
                candidates.push(s)
            }
        }

        Ok(Some(candidates))
    }

    pub fn object_hash(data: Vec<u8>, format: &str, write: bool) -> Result<String> {
        let object = match format {
            "blob" => DeltaObject::Blob(DeltaBlob { data }),
            "commit" => DeltaObject::Commit(DeltaCommit {
                data: kvlm_parse(&data)?,
            }),
            "tag" => DeltaObject::Tag(DeltaTag {
                data: kvlm_parse(&data)?,
            }),
            "tree" => DeltaObject::Tree(DeltaTree { data }),
            _ => return Err(anyhow!("Invalid format")),
        };
        let ObjectHash { sha, payload: _ } = Self::compute_object_hash(&object)?;

        if write {
            let cwd = std::env::current_dir()?;
            let repo = Self::repo_find(cwd)?;
            repo.object_write(&object)?;
        }

        Ok(sha)
    }

    pub fn compute_object_hash(object: &DeltaObject) -> Result<ObjectHash> {
        let data = object.serialise()?;
        let header = format!("{} {}\x00", object.format(), data.len());
        let payload = [header.as_bytes(), &data].concat();
        let sha = format!("{:x}", Sha1::digest(&payload));
        Ok(ObjectHash { sha, payload })
    }

    fn decompress(data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}
