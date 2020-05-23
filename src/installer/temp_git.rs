use std::fs;
use std::env;
use std::path::PathBuf;
use crate::error::Result;
use std::process::Command;

/// Helper struct which uses RAII to help ensure a git directory is cleaned up after cloning
pub struct TempGitDir {
    previous_dir: PathBuf,
    dir: PathBuf,
}

impl TempGitDir {
    pub fn clone_to_current_dir(url: &str) -> Result<Self> {
        Command::new("git")
            .args(&["clone", url, "tempdir_j93jfs3ff"])
            .status()?;
        let previous_dir = env::current_dir()?;
        env::set_current_dir(previous_dir.join("tempdir_j93jfs3ff"))?;
        let dir = env::current_dir()?;
        println!("{}", dir.display());
        Ok(Self {
            previous_dir,
            dir: env::current_dir()?
        })
    }

    pub fn delete(self) {  }
}

impl std::ops::Drop for TempGitDir {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.previous_dir);
        let _ = fs::remove_dir_all(&self.dir);
    }
}
