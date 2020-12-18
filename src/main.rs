use structopt::StructOpt;
use error::{Error, Result};
use std::process::Command;
use std::path::PathBuf;
use owo_colors::OwoColorize;

mod installer;
mod error;
mod cargo_info;
mod package;
mod build;
mod ftp;
mod tcp_listen;
mod ip_addr;
mod git_clone_wrappers;
mod game_paths;

#[derive(StructOpt)]
enum SubCommands {
    #[structopt(about = "Create a new plugin from a template")]
    New {
        name: String,

        #[structopt(
            short, long,
            default_value = "https://github.com/ultimate-research/skyline-rs-template.git"
        )]
        template_git: String,

        #[structopt(
            short, long,
            default_value = "master"
        )]
        template_git_branch: String,
    },
    #[structopt(about = "Build the current plugin as an NRO")]
    Build {
        #[structopt(long)]
        release: bool,

        #[structopt(long)]
        nso: bool,

        args: Vec<String>
    },
    #[structopt(about = "Build the current plugin and install to a switch over FTP")]
    Install {
        #[structopt(short, long)]
        debug: bool,

        #[structopt(long)]
        nso: bool,

        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>,

        #[structopt(
            short, long,
            about = "Install a project from a git url to the switch"
        )]
        git: Option<String>,
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

        #[structopt(long)]
        nso: bool,

        #[structopt(short, long)]
        restart: bool,

        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>
    },
    #[structopt(about = "Install the current plugin and listen for skyline logging")]
    Restart {
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
    },
    #[structopt(about = "Listen for logs being output from a switch running skyline at the given ip")]
    Listen {
        #[structopt(short, long)]
        ip: Option<String>,
    },
    #[structopt(about = "List the files in the plugin directory for the given game")]
    List {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>,

        path: Option<String>
    },
    #[structopt(about = "Delete a file in the plugin directory for the given game")]
    Rm {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>,

        filename: Option<String>
    },
    #[structopt(about = "Copy a file over FTP")]
    Cp {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short, long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml",
        )]
        title_id: Option<String>,

        src: String,

        dest: String
    },
    #[structopt(about = "Update cargo-skyline command")]
    SelfUpdate {
        #[structopt(short, long, default_value = "https://github.com/jam1garner/cargo-skyline")]
        git: String,

        #[structopt(short, long)]
        from_master: bool,
    },
    #[structopt(about = "Package plugin and latest Skyline into a zip file to prepare it for release")]
    Package {
        #[structopt(
            short, long,
            default_value = "https://github.com/skyline-dev/skyline/releases/download/beta/skyline.zip"
        )]
        skyline_release: String,

        #[structopt(
            short, long,
            about = "Title ID of the game to package the plugin for",
        )]
        title_id: Option<String>,

        #[structopt(
            short, long,
            about = "Path to output zip to",
            default_value = "target/release.zip"
        )]
        out_path: String,
    },
    #[structopt(about = "Update libraries for current plugin folder")]
    Update,
    #[structopt(about = "Document the current plugin and its dependencies")]
    Doc {
        #[structopt(
            short, long,
            about = "Whether or not to open the docs in the default browser afterwards",
        )]
        open: bool,
    },
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
        Install { ip, title_id, debug, nso, git } => if let Some(git) = git {
            installer::from_git(&git, ip, title_id, !debug, nso)
        } else {
            installer::install(ip, title_id, !debug, nso)
        },
        SetIp { ip } => ip_addr::set_ip(ip),
        ShowIp => ip_addr::show_ip(),
        Build { args, release, nso } => build::build(args, release, nso),
        Run { ip, title_id, debug, nso, restart } => installer::install_and_run(ip, title_id, !debug, restart, nso),
        Restart { ip, title_id } => installer::restart_game(ip, title_id),
        New { name, template_git, template_git_branch } => git_clone_wrappers::new_plugin(name, template_git, template_git_branch),
        UpdateStd { git, std_path } => git_clone_wrappers::update_std(git, std_path),
        Listen { ip } => tcp_listen::listen(ip),
        List { ip, title_id, path } => installer::list(ip, title_id, path),
        Rm { ip, title_id, filename } => installer::rm(ip, title_id, filename),
        Cp { ip, title_id, src, dest } => installer::cp(ip, title_id, src, dest),
        SelfUpdate { from_master, git } => self_update(from_master, git),
        Package { skyline_release, title_id, out_path }
            => package::package(&skyline_release, title_id.as_deref(), &out_path),
        Update => update(),
        Doc { open } => build::doc(if open { vec!["--open".into()] } else { vec![] })
    };

    if let Err(err) = result {
        match err {
            Error::NoIpFound => eprintln!("{}: {}", "ERROR".red(), error::NO_IP),
            Error::BadIpAddr => eprintln!("{}: {}", "ERROR".red(), error::BAD_IP_ADDR),
            Error::FtpError(ftp_err) => {
                eprintln!("{}{}","An FTP Error Occurred: ".red(), ftp_err)
            }
            Error::NoHomeDir => eprintln!("{}: No home directory could be found", "ERROR".red()),
            Error::NoPathFound => eprintln!("{}: No environment variable PATH could be found.", "ERROR".red()),
            Error::CreateSwitchDirDenied
                => eprintln!("{}: Could not create $HOME/.switch", "ERROR".red()),
            Error::WriteIpDenied => eprintln!("{}: Could not write IP to file", "ERROR".red()),
            //Error::NoCargoToml => eprintln!("{}", "No Cargo.toml could be found. Make sure you are within your plugin directory.".red()),
            //Error::BadCargoToml => eprintln!("{}", "Cargo.toml is formatted incorrectly.".red()),
            Error::NoTitleId => error::no_title_id(),
            Error::FailParseCargoStream => eprintln!("{}", "Unable to parse cargo output stream"),
            Error::CargoError(err) => eprintln!("{}{}", "CargoError: ".red(), err),
            Error::ExitStatus(code) => std::process::exit(code),
            Error::FailWriteNro => eprintln!("{}: Unable to convert file from ELF to NRO", "ERROR".red()),
            Error::IoError(err) => eprintln!("{}{}", "IoError: ".red(), err),
            Error::FailUpdateStd => eprintln!("{}: Could not update std due to a git-related failure", "ERROR".red()),
            Error::NoStdFound => eprintln!("{}: Could not find stdlib. Make sure you're inside of either your workspace or a plugin folder", "ERROR".red()),
            Error::DownloadError => eprintln!("{}: Failed to download latest release of Skyline. An internet connection is required.", "ERROR".red()),
            Error::ZipError => eprintln!("{}: Failed to read Skyline release zip. Either corrupted or missing files.", "ERROR".red()),
            Error::NoNpdmFileFound => eprintln!("{}: Custom NPDM file specified in Cargo.toml not found at the specified path.", "ERROR".red()),
            Error::AbsSwitchPath => eprintln!("{}: Absolute Switch paths must be prepended with \"sd:/\"", "ERROR".red()),
        }

        std::process::exit(1);
    }
}

fn self_update(from_master: bool, git: String) -> Result<()> {
    let mut args = vec!["install", "--force"];

    if from_master {
        args.push("--git");
        args.push(&git);
    } else {
        args.push("cargo-skyline");
    }

    Command::new("cargo")
        .args(&args)
        .status()
        .unwrap();

    Ok(())
}

fn update() -> Result<()> {
    Command::new("xargo")
        .arg("update")
        .status()?;

    Ok(())
}
