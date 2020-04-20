use std::io;
use crate::ftp::FtpError;

pub enum Error {
    NoIpFound,
    BadIpAddr,
    NoHomeDir,
    NoPathFound,
    CreateSwitchDirDenied,
    WriteIpDenied,
    //NoCargoToml,
    //BadCargoToml,
    NoTitleId,
    FailParseCargoStream,
    FailWriteNro,
    NoStdFound,
    FailUpdateStd,
    IoError(io::Error),
    FtpError(FtpError),
    CargoError(cargo_metadata::Error),
    ExitStatus(i32)
}

pub type Result<T> = core::result::Result<T, Error>;

pub static NO_IP: &str = "\n\nNo ip address found. Configure using `cargo skyline set-ip [addr]`, set using the SWITCH_IP environment variable, or pass as an argument.";
pub static BAD_IP_ADDR: &str = "\n\nCould not parse IP address: likely is not correctly formatted.";


impl From<FtpError> for Error {
    fn from(err: FtpError) -> Self {
        Self::FtpError(err)
    }
}

impl From<cargo_metadata::Error> for Error {
    fn from(err: cargo_metadata::Error) -> Self {
        Self::CargoError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}