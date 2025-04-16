use crate::object::{DeltaBlob, DeltaCommit, DeltaObject, DeltaTag, DeltaTree};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
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

impl DeltaRepository {
    pub fn new(path: &Path, force: bool) -> Result<Self, Box<dyn Error>> {
        let worktree = path.to_path_buf();
        let deltadir = worktree.join(".delta");

        if !force && !deltadir.is_dir() {
            return Err(format!("Not a delta repository: {}", path.display()).into());
        }

        let config = if force {
            None
        } else {
            let config_path = deltadir.join("config");
            if config_path.exists() {
                let mut ini = Ini::new();
                ini.load(config_path.to_str().unwrap())?;
                Some(ini)
            } else {
                return Err("Config is missing".into());
            }
        };

        if !force {
            if let Some(ref config) = config {
                match config.get("core", "repositoryformatversion").as_deref() {
                    Some("0") => {}
                    Some(v) => {
                        return Err(format!("Unsupported repositoryformatversion: {}", v).into())
                    }
                    None => return Err("Missing repositoryformatversion".into()),
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

    pub fn repo_file(&self, path: &[&str], mkdir: bool) -> Result<PathBuf, Box<dyn Error>> {
        if mkdir {
            let parent_path = &path[..path.len() - 1];
            self.repo_dir(parent_path, mkdir)?;
        }

        Ok(self.repo_path(path))
    }

    pub fn repo_dir(&self, path: &[&str], mkdir: bool) -> Result<PathBuf, Box<dyn Error>> {
        let path = self.repo_path(path);

        if path.exists() {
            if path.is_dir() {
                return Ok(path);
            } else {
                return Err("Path exists but is not a directory".into());
            }
        }

        if mkdir {
            fs::create_dir_all(&path)?;
            Ok(path)
        } else {
            Err("Directory does not exist".into())
        }
    }

    pub fn repo_create(&self, path: PathBuf) -> Result<DeltaRepository, Box<dyn Error>> {
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

    pub fn repo_find(path: PathBuf, throw_error: bool) -> Result<Option<Self>, Box<dyn Error>> {
        if path.join(".delta").exists() {
            return Ok(Some(Self::new(&path, false)?));
        }

        let parent = match path.parent() {
            Some(path) => path.to_path_buf(),
            None => {
                if throw_error {
                    return Err("No delta directory found".into());
                } else {
                    return Ok(None);
                }
            }
        };

        Self::repo_find(parent, throw_error)
    }

    pub fn object_read(&self, sha: &str) -> Result<Option<Box<dyn DeltaObject>>, Box<dyn Error>> {
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

        let s = std::str::from_utf8(&raw[ascii_space_index..null_byte_index])?;
        if s.parse::<usize>()? != raw.len() - null_byte_index - 1 {
            return Err(format!("Malformed obejct {} bad length", sha).into());
        }

        let mut constructor: Box<dyn DeltaObject> = match format {
            b"commit" => Box::new(DeltaCommit { data: vec![] }),
            b"tree" => Box::new(DeltaTree { data: vec![] }),
            b"tag" => Box::new(DeltaTag { data: vec![] }),
            b"blob" => Box::new(DeltaBlob { data: vec![] }),
            _ => {
                return Err(format!(
                    "Unknown type {} for object {}",
                    std::str::from_utf8(format)?,
                    sha
                )
                .into())
            }
        };

        let content = &raw[null_byte_index + 1..];
        constructor.deserialise(content)?;
        Ok(Some(constructor))
    }

    fn object_write<T: DeltaObject>(&self, object: T) -> Result<String, Box<dyn Error>> {
        let data = object.serialise()?;
        let result: Vec<u8> = vec![
            object.format(),
            b" ",
            format!("{}", data.len()).as_bytes(),
            b"\x00",
            &data,
        ]
        .iter()
        .flat_map(|s| s.iter().copied())
        .collect();
        let sha = format!("{:x}", Sha1::digest(&result));

        let path = Self::repo_file(&self, &["objects", &sha[0..2], &sha[2..]], true)?;
        if !path.exists() {
            let mut buffer = Vec::new();
            let mut z = ZlibEncoder::new(&mut buffer, Compression::default());
            z.write_all(&result)?;
            z.finish()?;
            std::fs::write(path, buffer)?;
        }

        Ok(sha)
    }

    fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}
