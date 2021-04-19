use std::io;
use crate::ftp::FtpError;
use owo_colors::OwoColorize;

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
    DownloadError,
    ZipError,
    NoNpdmFileFound,
    IoError(io::Error),
    FtpError(FtpError),
    CargoError(cargo_metadata::Error),
    ExitStatus(i32),
    AbsSwitchPath,
    BadSdPath
}

pub type Result<T> = core::result::Result<T, Error>;

pub static NO_IP: &str = "\n\nNo ip address found. Configure using `cargo skyline set-ip [addr]`, set using the SWITCH_IP environment variable, or pass as an argument.";
pub static BAD_IP_ADDR: &str = "\n\nCould not parse IP address: likely is not correctly formatted.";

pub fn no_title_id() {
    eprintln!(concat!(
        "{}: Unable to install as no title id could be found to install to.",
        "Set in Cargo.toml in the `package.metadata.skyline.titleid` key or pass via `--titleid [id]`"),
        "ERROR".red()
    );
    eprintln!("\n{}:\n\n[package.metadata.skyline]\ntitleid = \"01006A800016E000\"\n\n", "Example".bright_blue());
}

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

impl From<zip::result::ZipError> for Error {
    fn from(_: zip::result::ZipError) -> Self {
        Self::ZipError
    }
}
