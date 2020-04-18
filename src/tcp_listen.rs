use std::net::TcpStream;
use crate::error::Result;
use crate::ip_addr::{verify_ip, get_ip};

pub fn listen(ip: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;
    
    println!("---------------------------------------------------------------");

    let stdout = std::io::stdout();

    loop {
        if let Ok(mut logger) = TcpStream::connect((ip, 6969)) {
            let _ = std::io::copy(&mut logger, &mut stdout.lock());
        }
    }
}
