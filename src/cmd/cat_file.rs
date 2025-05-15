use crate::object::DeltaObject;
use crate::repo::DeltaRepository;
use std::{env, error::Error, io::Write};

pub fn cat_file(object: String, format: String) -> Result<(), Box<dyn Error>> {
    let cwd = env::current_dir().expect("Error getting cwd");
    let repo = DeltaRepository::repo_find(cwd)?;

    let sha = repo.object_find(object, format, true);
    let obj = repo.object_read(&sha)?.ok_or("Object not found")?;
    let data = obj.serialise()?;

    std::io::stdout().write_all(&data)?;
    Ok(())
}
