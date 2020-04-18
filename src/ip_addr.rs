use std::net::IpAddr;
use crate::error::{Result, Error};

const IP_ADDR_FILE: &str = "ip_addr.txt";

pub fn verify_ip(ip: String) -> Result<IpAddr> {
    let ip: IpAddr = ip.trim()
                        .replace(" ", "")
                        .parse()
                        .map_err(|_| Error::BadIpAddr)?;

    Ok(ip)
}

pub fn get_ip(cli_ip: Option<String>) -> Result<String> {
    cli_ip
        .or_else(|| std::env::var("SWITCH_IP").ok())
        .or_else(get_home_ip)
        .ok_or(Error::NoIpFound)
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

pub fn show_ip() -> Result<()> {
    let ip = verify_ip(get_ip(None)?)?;

    println!("{}", ip);

    Ok(())
}

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
