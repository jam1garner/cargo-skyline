use crate::error::Result;
use std::fs;
use std::process::Command;

fn replace(path: &str, find: &str, replace: &str) -> Result<()> {
    let temp = fs::read_to_string(path)?;
    let temp = temp.replace(find, replace);
    fs::write(path, temp)?;

    Ok(())
}

pub fn new_plugin(name: String, git_url: String, git_branch: String) -> Result<()> {
    crate::update_std::check_std_installed()?;

    println!("Creating plugin...");
    let status = Command::new("git")
        .args(&[
            "clone",
            "-b",
            &git_branch,
            "--single-branch",
            &git_url,
            &name,
        ])
        .stdout(std::process::Stdio::piped())
        .status()
        .unwrap();

    if status.success() {
        let paths = &[
            "Cargo.toml",
            "src/lib.rs",
            ".github/workflows/rust_build.yml",
        ];

        for path in paths {
            replace(&format!("{}/{}", name, path), "skyline_rs_template", &name)?;
        }

        let _ = fs::remove_file(&format!("{}/{}", name, ".github/workflows/rustdoc.yml"));
    }

    Ok(())
}
