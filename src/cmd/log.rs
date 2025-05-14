use std::env;

use crate::{object::DeltaTree, repo::DeltaRepository};

pub fn log(commit: String) {
    let cwd = env::current_dir().expect("Unable to determine cwd");
    let repo = DeltaRepository::repo_find(cwd).expect("Error finding repo");
}
