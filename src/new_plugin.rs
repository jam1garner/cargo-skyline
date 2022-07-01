use crate::error::{Error, Result};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

macro_rules! files {
    ($($path:literal),*) => {
        &[$(
            (
                $path,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/new-project-template/", $path, ".template")),
            )
        ),*]
    };
}

const FILES: &[(&str, &str)] = files!["Cargo.toml", "src/lib.rs"];

pub fn new_plugin(name: String) -> Result<()> {
    crate::update_std::check_std_installed()?;

    let plugin_folder = Path::new(".").join(&name);
    if plugin_folder.exists() {
        return Err(Error::ProjectAlreadyExists);
    }

    fs::create_dir(&plugin_folder).map_err(|_| Error::FailCreateProject)?;

    for (file, contents) in FILES {
        let path = plugin_folder.join(file);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::write(path, contents.replace("skyline_rs_template", &name))
            .map_err(|_| Error::FailCreateProject)?;
    }

    let success = Command::new("git")
        .arg("init")
        .current_dir(&plugin_folder)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !success {
        eprintln!("Warning: Failed to initialize git repository in {name}");
    }

    Ok(())
}
