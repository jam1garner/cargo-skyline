use std::io::prelude::*;
use std::process::Command;
use std::fs;
use structopt::StructOpt;
use error::{Error, Result};
use std::path::Path;
use colored::*;
use std::path::PathBuf;

mod installer;
mod error;
mod cargo_info;
mod build;
mod ftp;

#[derive(StructOpt)]
enum SubCommands {
    #[structopt(about = "Create a new plugin from a template")]
    New {
        name: String,
    },
    #[structopt(about = "Build the current plugin as an NRO")]
    Build {
        #[structopt(long)]
        release: bool,
        args: Vec<String>
    },
    #[structopt(about = "Build the current plugin and install to a switch over FTP")]
    Install {
        #[structopt(short, long)]
        debug: bool,
        
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>
    },
    #[structopt(about = "Set the IP address of the switch to install to")]
    SetIp {
        ip: String
    },
    #[structopt(about = "Show the currently configured IP address")]
    ShowIp,
    #[structopt(about = "Install the current plugin and listen for skyline logging")]
    Run {
        #[structopt(short, long)]
        debug: bool,
        
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>
    },
    #[structopt(about = "Download the latest stdlib for aarch64-skyline-switch")]
    UpdateStd {
        #[structopt(short, long, default_value = "https://github.com/jam1garner/rust-std-skyline-squashed")]
        git: String,

        #[structopt(short, long)]
        std_path: Option<PathBuf>
    }
}

#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
enum Args {
    Skyline(SubCommands)
}

fn main() {
    let Args::Skyline(subcommand) = Args::from_args();

    use SubCommands::*;

    let result = match subcommand {
        Install { ip, title_id, debug } => installer::install(ip, title_id, !debug),
        SetIp { ip } => installer::set_ip(ip),
        ShowIp => installer::show_ip(),
        Build { args, release } => build::build(args, release),
        Run { ip, title_id, debug} => installer::install_and_run(ip, title_id, !debug),
        New { name } => new(name),
        UpdateStd { git, std_path } => update_std(git, std_path)
    };

    if let Err(err) = result {
        match err {
            Error::NoIpFound => eprintln!("{}", error::NO_IP.red()),
            Error::BadIpAddr => eprintln!("{}", error::BAD_IP_ADDR.red()),
            Error::FtpError(ftp_err) => {
                eprintln!("{}{}","An FTP Error Occurred: ".red(), ftp_err)
            }
            Error::NoHomeDir => eprintln!("{}", "No home directory could be found".red()),
            Error::CreateSwitchDirDenied
                => eprintln!("{}", "Could not create $HOME/.switch".red()),
            Error::WriteIpDenied => eprintln!("{}", "Could not write IP to file".red()),
            //Error::NoCargoToml => eprintln!("{}", "No Cargo.toml could be found. Make sure you are within your plugin directory.".red()),
            //Error::BadCargoToml => eprintln!("{}", "Cargo.toml is formatted incorrectly.".red()),
            Error::NoTitleId => eprintln!("{}", "Unable to install as no title id could be found to install to. Set in Cargo.toml in the `package.metadata.titleid` key or pass via `--titleid [id]`".red()),
            Error::FailParseCargoStream => eprintln!("{}", "Unable to parse cargo output stream"),
            Error::CargoError(err) => eprintln!("{}{}", "CargoError: ".red(), err),
            Error::ExitStatus(code) => std::process::exit(code),
            Error::FailWriteNro => eprintln!("{}", "Unable to convert file from ELF to NRO".red()),
            Error::IoError(err) => eprintln!("{}{}", "IoError: ".red(), err),
            Error::FailUpdateStd => eprintln!("{}", "Could not update std due to a git-related failure".red()),
            Error::NoStdFound => eprintln!("{}", "Could not find stdlib. Make sure you're inside of either your workspace or a plugin folder".red()),
        }

        std::process::exit(1);
    }
}

fn replace(path: &str, find: &str, replace: &str) -> Result<()> {
    let temp = fs::read_to_string(path)?;
    let temp = temp.replace(find, replace);
    fs::write(path, temp)?;

    Ok(())
}

const STD_GIT_URL: &str = "https://github.com/jam1garner/rust-std-skyline-squashed.git";
const TEMPLATE_GIT_URL: &str = "https://github.com/ultimate-research/skyline-rs-template.git";

fn new(name: String) -> Result<()> {
    if !Path::new("rust-std-skyline-squashed").exists() {
        println!("Not setup to be a plugin folder, Set it up as one? This will take up to 1 GB of space.");
        println!("Note: this can be shared between all the plugins in the folder.");
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
                "clone", TEMPLATE_GIT_URL, &name
            ])
            .stdout(std::process::Stdio::piped())
            .status()
            .unwrap();
    
    if status.success() {
        let paths = &["Cargo.toml", "src/lib.rs"];

        for path in paths {
            replace(&format!("{}/{}", name, path), "skyline_rs_template", &name)?;
        }
    }

    Ok(())
} 


fn update_std(git_url: String, std_path: Option<PathBuf>) -> Result<()> {
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