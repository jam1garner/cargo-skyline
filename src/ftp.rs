use std::net::{IpAddr, TcpStream};
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::time::Duration;
use std::fmt;

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
        //println!("{}", line);
        Ok(line)
    }

    pub fn login(&mut self, user: &str, pass: &str) -> Result<&mut Self> {
        self.user(user)?
            .pass(pass)
    }

    pub fn expect_success(&mut self) -> Result<()> {
        let (num, _) = self.next()?;

        if (200..299).contains(&num) {
            Ok(())
        } else {
            //println!("{} {}", num, text);
            Err(FtpError::UnexpectedStatus(num))
        }
    }

    pub fn send<D: std::fmt::Display>(&mut self, string: D) -> Result<()> {
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

    pub fn open_passive_channel(&mut self) -> Result<TcpStream> {
        self.send("PASV")?;
        
        let ip = match self.next()? {
            (status, ip) if (200..299).contains(&status) => {
                ip
            }
            (status, _) => {
                return Err(FtpError::UnexpectedStatus(status))
            }
        };

        let ip: Vec<_> = ip.split(",").map(|x| x.trim()).collect();

        if ip.len() < 6 {
            Err(FtpError::ParseFail)
        } else {
            let ip: String = ip[0..4].join(".") + ":" + &((int(ip[4])? << 8) + int(ip[5])?).to_string();

            println!("Transferring data over {}...", ip);

            Ok(TcpStream::connect(&ip)?)
        }
    }

    pub fn put<S: AsRef<str>, D: AsRef<[u8]>>(&mut self, path: S, file: D) -> Result<()> {
        self.send(format!("DELE {}", path.as_ref()))?;

        let _ = self.next_line()?;

        self.send("TYPE I")?;

        //self.next_line()?);
        self.expect_success()?;

        let mut channel = self.open_passive_channel()?;
        
        self.send(format!("STOR {}", path.as_ref()))?;

        channel.write_all(file.as_ref())?;

        std::thread::sleep(Duration::from_millis(500));

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