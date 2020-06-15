use std::net::IpAddr;
use crate::error::{Result, Error};
use crate::{build, cargo_info};
use crate::ftp::FtpClient;
use crate::tcp_listen;
use crate::ip_addr::{get_ip, verify_ip};
use crate::game_paths::{get_game_path, get_plugin_path};
use temp_git::TempGitDir;
use colored::*;

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

fn warn_if_old_skyline_subsdk(client: &mut FtpClient, subsdk_base: &str) {
    for i in 1..=8 {
        if client.file_exists(format!("{}{}", subsdk_base, i)).unwrap_or(false) {
            println!("{}: An old install of skyline is detected, this may cause problems.", "WARNING".yellow());
            println!("Path: \"{}{}\"\n", subsdk_base, i);
            return;
        }
    }
}

fn parse_tid(tid: &str) -> u64 {
    u64::from_str_radix(tid, 16).expect("Invalid Title ID")
}

static SKYLINE_URL: &str = "https://github.com/shadowninja108/Skyline/releases/download/beta/Skyline.zip";
static TEMPLATE_NPDM: &[u8] = include_bytes!("template.npdm");

pub fn install(ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    let args = if release {
        vec![String::from("--release")]
    } else {
        vec![]
    };
    let nro_path = build::build_get_nro(args)?;

    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, true)?;

    let metadata = cargo_info::get_metadata()?;

    let title_id =
            title_id.or_else(|| metadata.title_id)
                    .ok_or(Error::NoTitleId)?;

    let dir_path = get_plugin_path(&title_id);

    println!("Ensuring directory exists...");
    let _ = client.mkdir(&(get_game_path(&title_id)));
    let _ = client.mkdir(&(get_game_path(&title_id) + "/romfs"));
    let _ = client.mkdir(&(get_game_path(&title_id) + "/romfs/skyline"));
    let _ = client.mkdir(&(get_game_path(&title_id) + "/romfs/skyline/plugins"));
    let _ = client.mkdir(&(get_game_path(&title_id) + "/exefs"));

    warn_if_old_skyline_subsdk(&mut client, &(get_game_path(&title_id) + "/exefs/subsdk"));

    // Ensure skyline is installed if it doesn't exist
    let subsdk_path = get_game_path(&title_id) + "/exefs/subsdk9";
    if !client.file_exists(&subsdk_path).unwrap_or(false){
        println!("Skyline subsdk not installed for the given title, downloading...");
        let exefs = crate::package::get_exefs(SKYLINE_URL)?;
        println!("Installing over subsdk9...");
        client.put(&subsdk_path, exefs.subsdk1)?;
    }

    let npdm_path = get_game_path(&title_id) + "/exefs/main.npdm";
    if !client.file_exists(&npdm_path).unwrap_or(false) {
        println!("Skyline npdm not installed for the given title, generating and installing...");
        client.put(&npdm_path, [
            &TEMPLATE_NPDM[..0x340],
            &parse_tid(&title_id).to_le_bytes()[..],
            &TEMPLATE_NPDM[0x348..]
        ].concat())?;
    }

    let nro_name = nro_path.file_name().map(|x| x.to_str()).flatten().ok_or(Error::FailWriteNro)?;

    println!("Transferring file...");
    client.put(
        &format!("{}/{}", dir_path, nro_name),
        std::fs::read(nro_path)?
    )?;

    Ok(())
}

pub fn from_git(git: &str, ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    let temp_dir = TempGitDir::clone_to_current_dir(git)?;

    install(ip, title_id, release)?;

    temp_dir.delete();

    Ok(())
}

pub fn install_and_run(ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    install(ip.clone(), title_id, release)?;
    
    tcp_listen::listen(ip)
}

pub fn list(ip: Option<String>, title_id: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip, false)?;

    let metadata = cargo_info::get_metadata()?;

    let title_id =
            title_id.or_else(|| metadata.title_id)
                    .ok_or(Error::NoTitleId)?;

    println!("{}", client.ls(
        Some(&(
            get_game_path(&title_id)
            + "/romfs/skyline/plugins"
        ))
    )?);

    Ok(())
}
