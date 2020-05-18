use std::io::prelude::*;
use std::process::Command;
use std::fs;
use crate::error::{Error, Result};
use std::path::Path;
use colored::*;
use std::path::PathBuf;
use std::env::current_dir;

fn replace(path: &str, find: &str, replace: &str) -> Result<()> {
    let temp = fs::read_to_string(path)?;
    let temp = temp.replace(find, replace);
    fs::write(path, temp)?;

    Ok(())
}

const STD_GIT_URL: &str = "https://github.com/jam1garner/rust-std-skyline-squashed.git";
const TEMPLATE_GIT_URL: &str = "https://github.com/ultimate-research/skyline-rs-template.git";

fn output_expected_tree(plugin_name: &str) -> Result<()> {
    let current_dir = current_dir()?;
    let name = if let Some(x) = current_dir.file_name() {
        x.to_string_lossy()
    } else {
        return Ok(());
    };

    println!("\nMake sure '{}' is an acceptable plugins workspace folder.", name.bright_blue());
    println!("This should likely be an empty folder that only contains plugins");
    println!("\nExpected resulting directory structure:");
    println!("{}", name.bright_blue());
    println!("├── {}", "rust-std-skyline-squashed".bright_blue());
    println!("└── {}", plugin_name.bright_blue());
    println!("    ├── {}", "src".bright_blue());
    println!("    │   └── {{...}}");
    println!("    └── Cargo.toml");
    println!("");
    println!("Setup workspace?");

    Ok(())
}

pub fn new_plugin(name: String) -> Result<()> {
    if !Path::new("rust-std-skyline-squashed").exists() {
        println!("Not setup to be a plugin folder, Set it up as one? This will take up to 1 GB of space.");
        println!("Note: this can be shared between all the plugins in the folder.");
        let _ = output_expected_tree(&name);
        print!("\n(y/n) ");

        let _ = std::io::stdout().lock().flush();

        let mut s = String::new();
        std::io::stdin().lock().read_line(&mut s).unwrap();

        if s.contains("y") {
            println!("Setting up plugin folder... (this might take a while)");
            let status =
                Command::new("git")
                    .args(&[
                        "clone", STD_GIT_URL
                    ])
                    .stdout(std::process::Stdio::piped())
                    .status()
                    .unwrap();
            if !status.success() || !Path::new("rust-std-skyline-squashed").exists() {
                eprintln!("{}", "Failed to clone rust-std-skyline-squashed".red());
                std::process::exit(1);
            }
        } else {
            std::process::exit(1);
        }
    }
    
    println!("Creating plugin...");
    let status =
        Command::new("git")
            .args(&[
                "clone", "-b", "master", "--single-branch", TEMPLATE_GIT_URL, &name
            ])
            .stdout(std::process::Stdio::piped())
            .status()
            .unwrap();
    
    if status.success() {
        let paths = &["Cargo.toml", "src/lib.rs", ".github/workflows/rust_build.yml"];

        for path in paths {
            replace(&format!("{}/{}", name, path), "skyline_rs_template", &name)?;
        }
    }

    Ok(())
} 


pub fn update_std(git_url: String, std_path: Option<PathBuf>) -> Result<()> {
    let in_same_folder: &Path = Path::new("rust-std-skyline-squashed");
    let in_parent_folder: &Path = Path::new("../rust-std-skyline-squashed");
    let path = if let Some(path) = &std_path {
        Ok(&**path)
    } else if in_same_folder.exists() {
        Ok(in_same_folder)
    } else if in_parent_folder.exists() {
        Ok(in_parent_folder)
    } else {
        Err(Error::NoStdFound)
    }?;

    println!("Removing existing stdlib...");
    let _ = fs::remove_dir_all(path);

    println!("Cloning current stdlib...");
    let status = 
        Command::new("git")
            .args(&[
                "clone", &git_url, path.to_str().ok_or(Error::NoStdFound)?
            ])
            .stdout(std::process::Stdio::piped())
            .status()
            .unwrap();

    if !status.success() {
        return Err(Error::FailUpdateStd)
    }

    println!("Clearing xargo cache...");
    let _ = fs::remove_dir_all(
        dirs::home_dir()
            .ok_or(Error::NoHomeDir)?
            .join(".xargo")
    );
    
    Ok(())
}
