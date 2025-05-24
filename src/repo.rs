use crate::kvlm::kvlm_parse;
use crate::object::{
    DeltaBlob, DeltaCommit, DeltaObject, DeltaObjectEnum, DeltaTag, DeltaTree, ObjectFormat,
};
use anyhow::{anyhow, Context, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use indexmap::IndexMap;
use ini::configparser::ini::Ini;
use sha1::{Digest, Sha1};
use std::{
    error::Error,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct DeltaRepository {
    pub worktree: PathBuf,
    pub deltadir: PathBuf,
    pub config: Option<Ini>,
}

struct ObjectHash {
    sha: String,
    payload: Vec<u8>,
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
                    .with_context(|| {
                        format!("Error loading ini config from path {}", config_path)
                    })?;
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
            // Err("Directory does not exist".into())
            Err(anyhow!("Directory does not exist"))
        }
    }

    pub fn repo_create(&self, path: &PathBuf) -> Result<DeltaRepository, Box<dyn Error>> {
        let repo = DeltaRepository::new(&path, true)?;

        if repo.worktree.exists() {
            if !repo.worktree.is_dir() {
                return Err(format!("{} is not a directory", repo.worktree.display()).into());
            } else if repo.deltadir.exists() && fs::read_dir(&repo.deltadir)?.next().is_some() {
                return Err(format!("{} is not empty", repo.worktree.display()).into());
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
                .expect("Failed converting config path to string"),
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
            .with_context(|| format!("No such file exists {}", path.display()))?;

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

    pub fn object_read(&self, sha: &str) -> Result<Option<DeltaObjectEnum>, Box<dyn Error>> {
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
            None => return Err("Invalid header: Missing ASCII space".into()),
        };
        let format = &raw[0..ascii_space_index];

        let null_byte_index = match raw.iter().position(|&b| b == 0) {
            Some(i) => i,
            None => return Err("Invalid header: Missing null byte".into()),
        };

        let s = std::str::from_utf8(&raw[ascii_space_index + 1..null_byte_index])?;
        if s.parse::<usize>()? != raw.len() - null_byte_index - 1 {
            return Err(format!("Malformed object {} bad length", sha).into());
        }

        let content = &raw[null_byte_index + 1..];
        let format = ObjectFormat::from_bytes(format)?;
        let object = match format {
            ObjectFormat::Commit => {
                let mut obj = DeltaCommit {
                    data: IndexMap::new(),
                };
                obj.deserialise(content)?;
                DeltaObjectEnum::Commit(obj)
            }
            ObjectFormat::Tree => {
                let mut obj = DeltaTree { data: vec![] };
                obj.deserialise(content)?;
                DeltaObjectEnum::Tree(obj)
            }
            ObjectFormat::Tag => {
                let mut obj = DeltaTag { data: vec![] };
                obj.deserialise(content)?;
                DeltaObjectEnum::Tag(obj)
            }
            ObjectFormat::Blob => {
                let mut obj = DeltaBlob { data: vec![] };
                obj.deserialise(content)?;
                DeltaObjectEnum::Blob(obj)
            }
        };

        Ok(Some(object))
    }

    fn object_write(&self, object: &dyn DeltaObject) -> Result<(), Box<dyn Error>> {
        let ObjectHash { sha, payload } = Self::compute_object_hash(object)?;
        let path = Self::repo_file(&self, &["objects", &sha[0..2], &sha[2..]], true)?;
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
        format: ObjectFormat,
        follow: bool,
    ) -> Result<String, Box<dyn Error>> {
        Ok(name.to_string())
    }

    pub fn object_resolve(&self, name: String) {}

    pub fn object_hash(data: Vec<u8>, format: &str, write: bool) -> Result<String, Box<dyn Error>> {
        let object: Box<dyn DeltaObject> = match format {
            "blob" => Box::new(DeltaBlob { data }),
            "commit" => Box::new(DeltaCommit {
                data: kvlm_parse(&data)?,
            }),
            "tag" => Box::new(DeltaTag { data }),
            "tree" => Box::new(DeltaTree { data }),
            _ => return Err("Invalid format".into()),
        };
        let ObjectHash { sha, payload: _ } = Self::compute_object_hash(object.as_ref())?;

        if write {
            let cwd = std::env::current_dir()?;
            let repo = Self::repo_find(cwd)?;
            repo.object_write(object.as_ref())?;
        }

        Ok(sha)
    }

    fn compute_object_hash(object: &dyn DeltaObject) -> Result<ObjectHash, Box<dyn Error>> {
        let data = object.serialise()?;
        let header = format!("{} {}\x00", object.format(), data.len());
        let payload = [header.as_bytes(), &data].concat();
        let sha = format!("{:x}", Sha1::digest(&payload));
        Ok(ObjectHash { sha, payload })
    }

    fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}
