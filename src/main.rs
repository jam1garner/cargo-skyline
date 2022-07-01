use error::{Error, Result};
use owo_colors::OwoColorize;
use structopt::StructOpt;

use std::fs;
use std::process::Command;

mod build;
mod cargo_info;
mod error;
mod ftp;
mod game_paths;
mod installer;
mod ip_addr;
mod new_plugin;
mod package;
mod tcp_listen;
mod update_std;

#[derive(StructOpt)]
enum SubCommands {
    #[structopt(about = "Create a new plugin from a template")]
    New { name: String },
    #[structopt(about = "Check if the current plugin builds and emit any errors found")]
    Check {
        #[structopt(long)]
        json: bool,
    },
    #[structopt(about = "Emit beginner-helpful lints and warnings")]
    Clippy {
        #[structopt(long)]
        no_deps: bool,

        #[structopt(long)]
        fix: bool,

        #[structopt(short, long)]
        features: Option<String>,

        #[structopt(long)]
        all_features: bool,

        #[structopt(long)]
        no_default_features: bool,

        #[structopt(long)]
        json: bool,

        #[structopt(last = true)]
        opts: Vec<String>,
    },
    #[structopt(about = "Build the current plugin as an NRO")]
    Build {
        #[structopt(long)]
        release: bool,

        #[structopt(long)]
        nso: bool,

        #[structopt(long)]
        no_default_features: bool,

        #[structopt(long)]
        features: Vec<String>,

        args: Vec<String>,
    },
    #[structopt(about = "Build the current plugin and install to a switch over FTP")]
    Install {
        #[structopt(short, long)]
        debug: bool,

        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,

        #[structopt(short, long, about = "Install a project from a git url to the switch")]
        git: Option<String>,

        #[structopt(long)]
        no_default_features: bool,

        #[structopt(long)]
        features: Vec<String>,

        #[structopt(long)]
        install_path: Option<String>,
    },
    #[structopt(about = "Set the IP address of the switch to install to")]
    SetIp { ip: String },
    #[structopt(about = "Show the currently configured IP address")]
    ShowIp,
    #[structopt(about = "Install the current plugin and listen for skyline logging")]
    Run {
        #[structopt(short, long)]
        debug: bool,

        #[structopt(short, long)]
        restart: bool,

        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,

        #[structopt(long)]
        no_default_features: bool,

        #[structopt(long)]
        features: Vec<String>,

        #[structopt(long)]
        install_path: Option<String>,
    },
    #[structopt(about = "Install the current plugin and listen for skyline logging")]
    Restart {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to install the plugin for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,
    },
    #[structopt(about = "Download the latest stdlib for aarch64-skyline-switch")]
    UpdateStd {
        #[structopt(short, long, default_value = "skyline-rs/rust")]
        repo: String,

        #[structopt(short, long)]
        tag: Option<String>,

        #[structopt(
            long,
            about = "Rather than shallow clone, perform a deep clone, allowing changes to be pushed afterwards"
        )]
        deep: bool,

        #[structopt(
            long,
            about = "Rather than re-clone, pull new commits. Assumes non-shallow clone.",
            conflicts_with = "deep"
        )]
        pull: bool,
    },
    #[structopt(
        about = "Listen for logs being output from a switch running skyline at the given ip"
    )]
    Listen {
        #[structopt(short, long)]
        ip: Option<String>,
    },
    #[structopt(about = "List the files in the plugin directory for the given game")]
    List {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,

        path: Option<String>,
    },
    #[structopt(about = "Delete a file in the plugin directory for the given game")]
    Rm {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,

        filename: Option<String>,
    },
    #[structopt(about = "Copy a file over FTP")]
    Cp {
        #[structopt(short, long)]
        ip: Option<String>,

        #[structopt(
            short,
            long,
            about = "Title ID of the game to list the installed plugins for, can be overriden in Cargo.toml"
        )]
        title_id: Option<String>,

        src: String,

        dest: String,
    },
    #[structopt(about = "Update cargo-skyline command")]
    SelfUpdate {
        #[structopt(
            short,
            long,
            default_value = "https://github.com/jam1garner/cargo-skyline"
        )]
        git: String,

        #[structopt(short, long)]
        from_master: bool,
    },
    #[structopt(
        about = "Package plugin and latest Skyline into a zip file to prepare it for release"
    )]
    Package {
        #[structopt(
            short,
            long,
            default_value = "https://github.com/skyline-dev/skyline/releases/download/beta/skyline.zip"
        )]
        skyline_release: String,

        #[structopt(
            short,
            long,
            about = "Disable the inclusion of skyline into the package"
        )]
        no_skyline: bool,

        #[structopt(short, long, about = "Title ID of the game to package the plugin for")]
        title_id: Option<String>,

        #[structopt(
            short,
            long,
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
            short,
            long,
            about = "Whether or not to open the docs in the default browser afterwards"
        )]
        open: bool,
    },

    #[structopt(
        about = "Clean a pre-existing project files no longer needed for the latest version"
    )]
    CleanProject,

    #[structopt(about = "Restart the given game using restart-plugin")]
    RestartGame,
}

#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
enum Args {
    Skyline(SubCommands),
}

fn main() {
    let Args::Skyline(subcommand) = Args::from_args();

    use SubCommands::*;

    if !matches!(&subcommand, CleanProject) {
        let default_config = fs::read_to_string(".cargo/config")
            .ok()
            .map(|config| config.trim().replace('\r', "") == DEFAULT_CONFIG)
            .unwrap_or(false);

        if default_config {
            eprintln!(
                "{}: outdated .cargo/config detected, this may cause issues.",
                "WARN".yellow().bold()
            );
            eprintln!(
                "   └ {}: consider running `cargo skyline clean-project`\n",
                "HELP".cyan().bold()
            );
        }
    }

    let result = match subcommand {
        Install {
            ip,
            title_id,
            debug,
            git,
            features,
            no_default_features,
            install_path,
        } => {
            if let Some(git) = git {
                installer::from_git(
                    &git,
                    ip,
                    title_id,
                    !debug,
                    features,
                    install_path,
                    no_default_features,
                )
            } else {
                installer::install(
                    ip,
                    title_id,
                    !debug,
                    features,
                    install_path,
                    no_default_features,
                )
            }
        }
        SetIp { ip } => ip_addr::set_ip(ip),
        ShowIp => ip_addr::show_ip(),
        Build {
            args,
            release,
            nso,
            features,
            no_default_features,
        } => build::build(args, release, nso, features, no_default_features),
        Check { json } => build::check(json),
        Clippy {
            no_deps,
            fix,
            features,
            all_features,
            no_default_features,
            json,
            opts,
        } => {
            let mut args = Vec::new();

            if no_deps {
                args.push("--no-deps".into());
            }

            if fix {
                args.push("--fix".into());
            }

            if let Some(features) = features {
                args.push("--features".into());
                args.push(features);
            }

            if all_features {
                args.push("--all-features".into());
            }

            if no_default_features {
                args.push("--no-default-features".into());
            }

            args.extend(opts);

            build::clippy(args, json)
        }
        Run {
            ip,
            title_id,
            debug,
            restart,
            features,
            install_path,
            no_default_features,
        } => installer::install_and_run(
            ip,
            title_id,
            !debug,
            restart,
            features,
            install_path,
            no_default_features,
        ),
        Restart { ip, title_id } => installer::restart_game(ip, title_id),
        New { name } => new_plugin::new_plugin(name),
        UpdateStd {
            repo,
            tag,
            deep,
            pull,
        } => update_std::update_std(&repo, tag.as_deref(), deep, pull),
        Listen { ip } => tcp_listen::listen(ip),
        List { ip, title_id, path } => installer::list(ip, title_id, path),
        Rm {
            ip,
            title_id,
            filename,
        } => installer::rm(ip, title_id, filename),
        Cp {
            ip,
            title_id,
            src,
            dest,
        } => installer::cp(ip, title_id, src, dest),
        SelfUpdate { from_master, git } => self_update(from_master, git),
        Package {
            skyline_release,
            title_id,
            out_path,
            no_skyline,
        } => package::package(
            &skyline_release,
            title_id.as_deref(),
            &out_path,
            !no_skyline,
        ),
        Update => update(),
        Doc { open } => build::doc(if open { vec!["--open".into()] } else { vec![] }),
        CleanProject => clean_project(),
        RestartGame => installer::restart_game(None, None),
    };

    if let Err(err) = result {
        let error = "ERROR".red();

        match err {
            Error::NoIpFound => eprintln!("{}: {}", error, error::NO_IP),
            Error::BadIpAddr => eprintln!("{}: {}", error, error::BAD_IP_ADDR),
            Error::FtpError(ftp_err) => {
                eprintln!("{}{}","An FTP Error Occurred: ".red(), ftp_err)
            }
            Error::NoHomeDir => eprintln!("{}: No home directory could be found", error),
            Error::NoPathFound => eprintln!("{}: No environment variable PATH could be found.", error),
            Error::CreateSwitchDirDenied
                => eprintln!("{}: Could not create $HOME/.switch", error),
            Error::WriteIpDenied => eprintln!("{}: Could not write IP to file", error),
            //Error::NoCargoToml => eprintln!("{}", "No Cargo.toml could be found. Make sure you are within your plugin directory.".red()),
            //Error::BadCargoToml => eprintln!("{}", "Cargo.toml is formatted incorrectly.".red()),
            Error::NoTitleId => error::no_title_id(),
            Error::FailParseCargoStream => eprintln!("Unable to parse cargo output stream"),
            Error::CargoError(err) => eprintln!("{}{}", "CargoError: ".red(), err),
            Error::ExitStatus(code) => std::process::exit(code),
            Error::FailWriteNro => eprintln!("{}: Unable to convert file from ELF to NRO", error),
            Error::IoError(err) => eprintln!("{}{}", "IoError: ".red(), err),
            Error::DownloadError => eprintln!("{}: Failed to download latest release of Skyline. An internet connection is required.", error),
            Error::ZipError => eprintln!("{}: Failed to read Skyline release zip. Either corrupted or missing files.", error),
            Error::NoNpdmFileFound => eprintln!("{}: Custom NPDM file specified in Cargo.toml not found at the specified path.", error),
            Error::AbsSwitchPath => eprintln!("{}: Absolute Switch paths must be prepended with \"sd:/\"", error),
            Error::BadSdPath => eprintln!("{}: Install paths must either start with \"rom:/\" or \"sd:/\"", error),
            Error::GithubError => eprintln!("{}: failed to get the latest release from github", error),
            //Error::InvalidRepo => eprintln!("{}: repos must be in the form of `{{user}}/{{repo}}`", error),
            //Error::HostNotSupported => eprintln!("{}: your host platform is not supported.", error),
            Error::DownloadFailed => eprintln!("{}: the update failed to download.", error),
            Error::RustupNotFound => eprintln!("{}: rustup could not be executed, make sure it is installed.", error),
            Error::RustupLinkFailed => eprintln!("{}: rustup could not link the skyline toolchain.", error),
            Error::RustupToolchainAddFailed => eprintln!("{}: rustup could not install the backing toolchain", error),
            Error::ToolchainCopyFailed => eprintln!("{}: could not copy the backing toolchain", error),
            Error::GitNotInstalled => eprintln!("{}: git is not installed, please install it", error),
            Error::StdCloneFailed => eprintln!("{}: std fork failed to clone", error),
            Error::NoBaseCommit => eprintln!("{}: No base rust-src commit was found, cannot determined correct nightly.", error),
            Error::ProjectAlreadyExists => eprintln!("{}: a folder with that name already exists", error),
            Error::FailCreateProject => eprintln!("{}: project files could not be written to disk", error),
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

    Command::new("cargo").args(&args).status().unwrap();

    Ok(())
}

fn update() -> Result<()> {
    Command::new("cargo").arg("update").status()?;

    Ok(())
}

const DEFAULT_CONFIG: &str = "[build]\ntarget = \"aarch64-skyline-switch\"";

fn clean_project() -> Result<()> {
    Command::new("cargo").arg("clean").status()?;

    let _ = fs::remove_file("rust-toolchain");

    let delete_config = fs::read_to_string(".cargo/config")
        .ok()
        .map(|config| config.trim().replace('\r', "") == DEFAULT_CONFIG)
        .unwrap_or(false);

    if delete_config {
        fs::remove_file(".cargo/config").unwrap();
        fs::remove_dir(".cargo").unwrap();
    }

    let _ = fs::remove_file("Xargo.toml");
    let _ = fs::remove_file("Cargo.lock");
    let _ = fs::remove_file("aarch64-skyline-switch.json");
    let _ = fs::remove_file("link.ld");

    Ok(())
}
