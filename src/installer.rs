use crate::error::{Error, Result};
use crate::ftp::FtpClient;
use crate::game_paths::{get_game_path, get_plugin_path, get_plugins_path};
use crate::ip_addr::{get_ip, verify_ip};
use crate::tcp_listen;
use crate::{build, cargo_info};
use owo_colors::OwoColorize;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use temp_git::TempGitDir;

mod temp_git;

fn connect(ip: IpAddr, print: bool) -> Result<FtpClient> {
    if print {
        println!("Connecting to ip '{}'...", ip);
    }

    let mut client = FtpClient::connect(ip)?;
    client.login("anonymous", "anonymous")?;

    if print {
        println!("{}", "Connected!".green());
    }

    Ok(client)
}

fn warn_if_old_skyline_subsdk(client: &mut FtpClient, exefs_path: &str) {
    let list = client.ls(Some(exefs_path)).unwrap();

    let subsdk_count = list.matches("subsdk").count();

    if subsdk_count > 1 {
        println!(
            "{}: An old install of skyline is detected, this may cause problems.",
            "WARNING".yellow()
        );
    }
}

fn parse_tid(tid: &str) -> u64 {
    u64::from_str_radix(tid, 16).expect("Invalid Title ID")
}

static SKYLINE_URL: &str =
    "https://github.com/skyline-dev/skyline/releases/download/beta/skyline.zip";
static TEMPLATE_NPDM: &[u8] = include_bytes!("template.npdm");

pub fn generate_npdm(tid: &str) -> Vec<u8> {
    [
        &TEMPLATE_NPDM[..0x340],
        &parse_tid(tid).to_le_bytes()[..],
        &TEMPLATE_NPDM[0x348..],
    ]
    .concat()
}

pub fn install(
    ip: Option<String>,
    title_id: Option<String>,
    release: bool,
    features: Vec<String>,
    path: Option<String>,
    no_default_features: bool,
) -> Result<()> {
    let mut args = if release {
        vec![String::from("--release")]
    } else {
        vec![]
    };

    if !features.is_empty() {
        args.push(format!("--features={}", features.join(",")));
    }

    let (path, is_rom) = if let Some(path) = path.as_ref() {
        if let Some(local_path) = path.strip_prefix("rom:/") {
            Ok((local_path, true))
        } else if let Some(absolute_path) = path.strip_prefix("sd:/") {
            Ok((absolute_path, false))
        } else {
            Err(Error::BadSdPath)
        }?
    } else {
        ("skyline/plugins", true)
    };

    if no_default_features {
        args.push("--no-default-features".to_owned());
    }

    let nro_path = build::build_get_nro(args)?;

    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, true)?;

    let metadata = cargo_info::get_metadata()?;

    let title_id = title_id
        .or_else(|| metadata.title_id.clone())
        .ok_or(Error::NoTitleId)?;

    println!("Ensuring directory exists...");

    // this is where subsdk9 goes, it doesn't depend on the path
    let _ = client.mkdir(&get_game_path(&title_id));
    let _ = client.mkdir(&(get_game_path(&title_id) + "/exefs"));

    let dirs = path
        .split('/')
        .filter(|x| !x.is_empty() && !x.ends_with(".nro"));

    let mut plugin_folder_path = if is_rom {
        format!("{}/romfs", get_game_path(&title_id))
    } else {
        String::from("")
    };

    // ensure romfs dir exists too
    let _ = client.mkdir(&plugin_folder_path);

    for dir in dirs {
        plugin_folder_path = format!("{}/{}", plugin_folder_path, dir);
        let _ = client.mkdir(&plugin_folder_path);
    }

    warn_if_old_skyline_subsdk(&mut client, &(get_game_path(&title_id) + "/exefs/"));

    // Ensure skyline is installed if it doesn't exist
    let subsdk_path = get_game_path(&title_id) + "/exefs/subsdk9";
    if !client.file_exists(&subsdk_path).unwrap_or(false) {
        println!("Skyline subsdk not installed for the given title, downloading...");
        let exefs = crate::package::get_exefs(SKYLINE_URL)?;
        println!("Installing over subsdk9...");
        client.put(&subsdk_path, exefs.subsdk1)?;
    }

    let npdm_path = get_game_path(&title_id) + "/exefs/main.npdm";
    if !client.file_exists(&npdm_path).unwrap_or(false) {
        println!("Skyline npdm not installed for the given title, generating and installing...");
        client.put(&npdm_path, generate_npdm(&title_id))?;
    }

    for dep in &metadata.plugin_dependencies {
        let dep_path = get_plugin_path(&title_id, &dep.name);
        if !client.file_exists(&dep_path).unwrap_or(false) {
            println!("Downloading dependency {}...", dep.name);
            let dep_data = reqwest::blocking::get(&dep.url)
                .map_err(|_| Error::DownloadError)?
                .bytes()
                .map_err(|_| Error::DownloadError)?;
            println!("Installing dependency {}...", dep.name);
            client.put(dep_path, &dep_data).unwrap();
        }
    }

    let nro_name = if path.ends_with(".nro") {
        path.split('/').last().unwrap()
    } else {
        nro_path
            .file_name()
            .map(|x| x.to_str())
            .flatten()
            .ok_or(Error::FailWriteNro)?
    };

    println!("Transferring file...");
    client.put(
        format!("{}/{}", plugin_folder_path, nro_name),
        std::fs::read(nro_path)?,
    )?;

    Ok(())
}

pub fn from_git(
    git: &str,
    ip: Option<String>,
    title_id: Option<String>,
    release: bool,
    features: Vec<String>,
    path: Option<String>,
    no_default_features: bool,
) -> Result<()> {
    let temp_dir = TempGitDir::clone_to_current_dir(git)?;

    install(ip, title_id, release, features, path, no_default_features)?;

    temp_dir.delete();

    Ok(())
}

use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

const RESTART_PLUGIN_PORT: u16 = 45423;

pub fn restart_game(ip: Option<String>, title_id: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    let mut port =
        TcpStream::connect_timeout(&(ip, RESTART_PLUGIN_PORT).into(), Duration::from_secs(1))?;

    let metadata = cargo_info::get_metadata()?;

    let title_id = title_id
        .or_else(|| metadata.title_id.clone())
        .ok_or(Error::NoTitleId)?;

    let title_id: u64 = u64::from_str_radix(&title_id, 0x10).unwrap_or(0);

    port.write_all(&title_id.to_be_bytes())?;

    Ok(())
}

pub fn install_and_run(
    ip: Option<String>,
    title_id: Option<String>,
    release: bool,
    restart: bool,
    features: Vec<String>,
    path: Option<String>,
    no_default_features: bool,
) -> Result<()> {
    install(
        ip.clone(),
        title_id.clone(),
        release,
        features,
        path,
        no_default_features,
    )?;

    if restart {
        let restart_ip = ip.clone();
        std::thread::spawn(move || {
            // Give logger some time to spin up
            std::thread::sleep(std::time::Duration::from_millis(50));

            let _ = restart_game(restart_ip, title_id);
        });
    }

    tcp_listen::listen(ip)
}

pub fn list(ip: Option<String>, title_id: Option<String>, path: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, false)?;

    if path.is_some() {
        println!("{}", client.ls(Some(&path.unwrap()))?);
        return Ok(());
    }

    let metadata = cargo_info::get_metadata()?;
    let title_id = title_id.or(metadata.title_id).ok_or(Error::NoTitleId)?;

    println!("{}", client.ls(Some(&get_plugins_path(&title_id)))?);

    Ok(())
}

/* There are really three cases here:
 ** 1. Filename is populated, and starts with '/'. Install path is filename treated as absolute path.
 ** 2. Filename is populated, but is a relative path. Install path is filename treated as relative path to plugin directory.
 ** 3. Filename isn't populated. Install path is current plugin NRO's default install path.
*/
fn get_install_path(title_id: Option<String>, filename: Option<String>) -> Result<String> {
    if filename.is_some() {
        let filename_str = (&filename).as_ref().unwrap();
        if filename_str.starts_with('/') {
            return Ok(filename_str.to_string());
        }
    }

    let metadata = cargo_info::get_metadata()?;

    let filename = filename.unwrap_or(format!("lib{}.nro", metadata.name));

    let title_id = title_id.or(metadata.title_id).ok_or(Error::NoTitleId)?;

    Ok(get_plugin_path(&title_id, &filename))
}

pub fn rm(ip: Option<String>, title_id: Option<String>, filename: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, false)?;

    client.rm(get_install_path(title_id, filename)?)?;

    Ok(())
}

// for now, we assume src is local and dest is Switch
pub fn cp(ip: Option<String>, title_id: Option<String>, src: String, dest: String) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, false)?;

    // TODO: remove once two-way CP is supported
    if dest.starts_with('/') {
        return Err(Error::AbsSwitchPath);
    }

    let dest_path = PathBuf::from(&dest.replace("sd:/", "/"));

    let mut install_path =
        get_install_path(title_id, Some(dest_path.to_str().unwrap().to_string()))?;

    let src_path = PathBuf::from(src);
    let src_basename = src_path.file_name().unwrap();
    let dest_basename = dest_path.file_name();

    // if we're given a folder rather than a full filepath
    if dest_basename.is_none() || dest_basename.unwrap() != src_basename {
        install_path = Path::new(&install_path)
            .join(src_basename)
            .to_str()
            .unwrap()
            .to_string();
    }

    println!("Transferring file to {}...", install_path);
    client.put(
        install_path,
        std::fs::read(src_path.to_str().unwrap().to_string())?,
    )?;

    Ok(())
}
