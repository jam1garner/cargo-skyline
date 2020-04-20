use structopt::StructOpt;
use error::Error;
use colored::*;
use std::path::PathBuf;

mod installer;
mod error;
mod cargo_info;
mod build;
mod ftp;
mod tcp_listen;
mod ip_addr;
mod git_clone_wrappers;

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
    },
    #[structopt(about = "Listen for logs being output from a switch running skyline at the given ip")]
    Listen {
        #[structopt(short, long)]
        ip: Option<String>,
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
        Install { ip, title_id, debug } => installer::install(ip, title_id, !debug),
        SetIp { ip } => ip_addr::set_ip(ip),
        ShowIp => ip_addr::show_ip(),
        Build { args, release } => build::build(args, release),
        Run { ip, title_id, debug} => installer::install_and_run(ip, title_id, !debug),
        New { name } => git_clone_wrappers::new_plugin(name),
        UpdateStd { git, std_path } => git_clone_wrappers::update_std(git, std_path),
        Listen { ip } => tcp_listen::listen(ip),
    };

    if let Err(err) = result {
        match err {
            Error::NoIpFound => eprintln!("{}", error::NO_IP.red()),
            Error::BadIpAddr => eprintln!("{}", error::BAD_IP_ADDR.red()),
            Error::FtpError(ftp_err) => {
                eprintln!("{}{}","An FTP Error Occurred: ".red(), ftp_err)
            }
            Error::NoHomeDir => eprintln!("{}", "No home directory could be found".red()),
            Error::NoPathFound => eprintln!("{}", "No environment variable PATH could be found.".red()),
            Error::CreateSwitchDirDenied
                => eprintln!("{}", "Could not create $HOME/.switch".red()),
            Error::WriteIpDenied => eprintln!("{}", "Could not write IP to file".red()),
            //Error::NoCargoToml => eprintln!("{}", "No Cargo.toml could be found. Make sure you are within your plugin directory.".red()),
            //Error::BadCargoToml => eprintln!("{}", "Cargo.toml is formatted incorrectly.".red()),
            Error::NoTitleId => eprintln!("{}\n\nExample:\n\n[package.metadata.skyline]\ntitleid = \"01006A800016E000\"\n\n", "Unable to install as no title id could be found to install to. Set in Cargo.toml in the `package.metadata.skyline.titleid` key or pass via `--titleid [id]`".red()),
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
