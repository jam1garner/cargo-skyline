use std::net::IpAddr;
use crate::error::{Result, Error};
use crate::{build, cargo_info};
use crate::ftp::FtpClient;
use crate::tcp_listen;
use crate::ip_addr::{get_ip, verify_ip};
use colored::*;

fn connect(ip: IpAddr) -> Result<FtpClient> {
    println!("Connecting to ip '{}'...", ip);

    let mut client = FtpClient::connect(ip)?;
    client.login("anonymous", "anonymous")?;

    println!("{}", "Connected!".green());

    Ok(client)
}

fn get_plugin_path(title_id: &str) -> String {
    format!("/atmosphere/contents/{}/romfs/skyline/plugins", title_id)
}

fn get_game_path(title_id: &str) -> String {
    format!("/atmosphere/contents/{}", title_id)
}

pub fn install(ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    let args = if release {
        vec![String::from("--release")]
    } else {
        vec![]
    };
    let nro_path = build::build_get_nro(args)?;

    let ip = verify_ip(get_ip(ip)?)?;

    let mut client = connect(ip)?;

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

    let nro_name = nro_path.file_name().map(|x| x.to_str()).flatten().ok_or(Error::FailWriteNro)?;

    println!("Setting binary mode...");
    println!("Transferring file...");
    client.put(
        &format!("{}/{}", dir_path, nro_name),
        std::fs::read(nro_path)?
    )?;

    Ok(())
}

pub fn install_and_run(ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    install(ip.clone(), title_id, release)?;
    
    tcp_listen::listen(ip)
}
