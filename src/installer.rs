use std::net::{IpAddr, TcpStream};
use crate::error::{Result, Error};
use crate::{build, cargo_info};
use crate::ftp::FtpClient;
use colored::*;

const IP_ADDR_FILE: &str = "ip_addr.txt";

fn get_home_ip() -> Option<String> {
    let switch_home_dir = dirs::home_dir()?.join(".switch");
    if switch_home_dir.exists() {
        let ip_addr_file = switch_home_dir.join(IP_ADDR_FILE);
        if ip_addr_file.exists() {
            std::fs::read_to_string(ip_addr_file).ok()
        } else {
            None
        }
    } else {
        None
    }
}

fn get_ip(cli_ip: Option<String>) -> Result<String> {
    cli_ip
        .or_else(|| std::env::var("SWITCH_IP").ok())
        .or_else(get_home_ip)
        .ok_or(Error::NoIpFound)
}

pub fn show_ip() -> Result<()> {
    let ip = verify_ip(get_ip(None)?)?;

    println!("{}", ip);

    Ok(())
}

fn connect(ip: IpAddr) -> Result<FtpClient> {
    println!("Connecting to ip '{}'...", ip);

    let mut client = FtpClient::connect(ip)?;
    client.login("anonymous", "anonymous")?;

    println!("{}", "Connected!".green());

    Ok(client)
}

fn verify_ip(ip: String) -> Result<IpAddr> {
    let ip: IpAddr = ip.trim()
                        .replace(" ", "")
                        .parse()
                        .map_err(|_| Error::BadIpAddr)?;

    Ok(ip)
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

pub fn set_ip(ip: String) -> Result<()> {
    let ip = verify_ip(ip)?;

    let home = dirs::home_dir().ok_or(Error::NoHomeDir)?;
    
    if !home.exists() {
        return Err(Error::NoHomeDir)
    }

    let switch_home_dir = home.join(".switch");

    if !switch_home_dir.exists() {
        std::fs::create_dir(&switch_home_dir).map_err(|_| Error::CreateSwitchDirDenied)?;
    }

    std::fs::write(
        switch_home_dir.join(IP_ADDR_FILE),
        ip.to_string()
    ).map_err(|_| Error::WriteIpDenied)
}

pub fn install_and_run(ip: Option<String>, title_id: Option<String>, release: bool) -> Result<()> {
    install(ip.clone(), title_id, release)?;
    
    let ip = verify_ip(get_ip(ip)?)?;
    
    println!("---------------------------------------------------------------");

    let stdout = std::io::stdout();

    loop {
        if let Ok(mut logger) = TcpStream::connect((ip, 6969)) {
            let _ = std::io::copy(&mut logger, &mut stdout.lock());
        }
    }
}