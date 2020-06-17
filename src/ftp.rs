use std::net::{IpAddr, TcpStream};
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::time::Duration;
use std::fmt;

#[derive(Debug)]
pub enum FtpError {
    Io(io::Error),
    ParseFail,
    UnexpectedStatus(usize),
}

type Result<T> = std::result::Result<T, FtpError>;

pub struct FtpClient {
    pub tcp: BufReader<TcpStream>
}

impl FtpClient {
    pub fn connect(ip: IpAddr) -> Result<Self> {
        let mut client = FtpClient {
            tcp: BufReader::new(TcpStream::connect((ip, 5000))?)
        };

        let status = client.next()?.0;
        if status == 220 {
            Ok(client)
        } else {
            Err(FtpError::UnexpectedStatus(status))
        } 
    }
    
    pub fn next(&mut self) -> Result<(usize, String)> {
        let mut status = self.next_line()?;
        let line = status.split_off(4);
        let num: usize = status[..3].parse().map_err(|_| FtpError::ParseFail)?;
        Ok((num, line))
    }

    pub fn next_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.tcp.read_line(&mut line)
            .map_err(|e| FtpError::Io(e))?;

        #[cfg(feature = "debug")] {
            println!("<FTP> {}", line.trim_end_matches("\n"));
        }
        Ok(line)
    }

    pub fn clear_status(&mut self) {
        let _ = self.tcp.get_mut().set_read_timeout(Some(Duration::from_millis(20)));
        let mut dump = vec![];
        let _ = self.tcp.read_to_end(&mut dump);
        let _ = self.tcp.get_mut().set_read_timeout(Some(Duration::from_millis(500)));
    }

    pub fn login(&mut self, user: &str, pass: &str) -> Result<&mut Self> {
        self.user(user)?
            .pass(pass)
    }

    pub fn expect_success(&mut self) -> Result<()> {
        let (num, _) = self.next()?;

        if (200..299).contains(&num) || num == 150 {
            Ok(())
        } else {
            //println!("{} {}", num, text);
            Err(FtpError::UnexpectedStatus(num))
        }
    }

    pub fn send<D: std::fmt::Display>(&mut self, string: D) -> Result<()> {
        #[cfg(feature = "debug")] {
            println!("[FTP] {}", string);
        }
        write!(self.tcp.get_mut(), "{}\n", string)?;

        Ok(())
    }

    pub fn user(&mut self, username: &str) -> Result<&mut Self> {
        self.send(format!("USER {}", username))?;

        self.expect_success()?;

        Ok(self)
    }

    pub fn pass(&mut self, password: &str) -> Result<&mut Self> {
        self.send(format!("PASS {}", password))?;
        
        self.expect_success()?;
        
        Ok(self)
    }

    pub fn mkdir<S: AsRef<str>>(&mut self, dir: S) -> Result<()> {
        self.send(format!("MKD {}", dir.as_ref()))?;
        self.expect_success()
    }

    pub fn open_passive_channel(&mut self) -> Result<(String, TcpStream)> {
        self.clear_status();
        self.send("PASV")?;
        
        let ip = loop {
            match self.next()? {
                (227, ip) => {
                    break ip
                }
                (status, _) if !(200..299).contains(&status) => {
                    return Err(FtpError::UnexpectedStatus(status))
                }
                _ => continue
            };
        };

        let ip: Vec<_> = ip.split(",").map(String::from).map(|mut x| { x.retain(char::is_numeric); x }).collect();

        if ip.len() < 6 {
            Err(FtpError::ParseFail)
        } else {
            let ip: String = ip[0..4].join(".") + ":" + &((int(&ip[4])? << 8) + int(&ip[5])?).to_string();

            let stream = TcpStream::connect(&ip)?;

            Ok((ip, stream))
        }
    }

    pub fn ls(&mut self, dir: Option<&str>) -> Result<String> {
        let mut channel = self.open_passive_channel()?.1;

        if let Some(dir) = dir {
            self.change_dir(dir)?;
        }

        self.send("LIST")?;

        let mut string = String::new();

        channel.read_to_string(&mut string)?;
        
        Ok(string)
    }

    pub fn file_exists<S: AsRef<str>>(&mut self, path: S) -> Result<bool> {
        self.clear_status();
        let mut channel = self.open_passive_channel().unwrap().1;

        self.send(format!("LIST {}", path.as_ref()))?;

        if self.expect_success().is_err() {
            return Ok(false);
        }

        let _ = self.next_line().unwrap();

        // Return true if stream is non-empty, i.e. the listing contains an item
        Ok(
            if channel.read(&mut [0; 2][..])? > 1 {
                true
            } else {
                false
            }
        )
    }

    pub fn change_dir<S: AsRef<str>>(&mut self, path: S) -> Result<()> {
        self.send(format!("CWD {}", path.as_ref()))?;

        self.expect_success()
    }

    pub fn put<S: AsRef<str>, D: AsRef<[u8]>>(&mut self, path: S, file: D) -> Result<()> {
        self.clear_status();
        self.send(format!("DELE {}", path.as_ref()))?;

        let _ = self.next_line()?;

        self.send("TYPE I")?;

        //self.next_line()?);
        self.expect_success()?;

        let (_ip, mut channel) = self.open_passive_channel()?;

        //println!("Transferring data over {}...", ip);
        
        self.send(format!("STOR {}", path.as_ref()))?;

        channel.write_all(file.as_ref())?;

        std::thread::sleep(Duration::from_millis(500));
        
        let _ = self.next_line()?;
        //let _ = dbg!(self.next_line()?);
        Ok(())
    }
}

fn int(s: &str) -> Result<usize> {
    s.parse().map_err(|_| FtpError::ParseFail)
}


impl From<io::Error> for FtpError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl fmt::Display for FtpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ParseFail => write!(f, "Failed to parse"),
            Self::Io(io) => write!(f, "IoError: {}", io),
            Self::UnexpectedStatus(status) => write!(f, "Unexpected status {}", status)
        }
    }
}
