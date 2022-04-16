use crate::error::Result;
use crate::ip_addr::{get_ip, verify_ip};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

pub fn listen(ip: Option<String>) -> Result<()> {
    let ip = verify_ip(get_ip(ip)?)?;

    println!("---------------------------------------------------------------");

    let stdout = std::io::stdout();

    loop {
        if let Ok(mut logger) = TcpStream::connect((ip, 6969)) {
            let _ = std::io::copy(&mut logger, &mut stdout.lock());
        }
        thread::sleep(Duration::from_millis(10));
    }
}
