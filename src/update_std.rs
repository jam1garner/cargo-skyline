use crate::Error;

use std::convert::TryInto;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use indicatif::{ProgressBar, ProgressStyle};
use octocrab::models::repos::Asset;
use zip::ZipArchive;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn get_cargo_dir() -> PathBuf {
    env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("No home directory found")
                .push_join(".cargo")
        })
        .ensure_exists()
}

fn get_cargo_skyline_dir() -> PathBuf {
    get_cargo_dir().push_join("skyline").ensure_exists()
}

fn get_skyline_toolchain_dir() -> PathBuf {
    get_cargo_skyline_dir()
        .push_join("toolchain")
        .ensure_exists()
}

fn get_toolchain() -> PathBuf {
    get_skyline_toolchain_dir()
        .push_join("skyline")
        .ensure_exists()
}

fn get_version_file() -> PathBuf {
    get_toolchain().push_join("version")
}

fn get_current_version() -> Option<String> {
    fs::read_to_string(get_version_file().if_exists()?).ok()
}

#[derive(Debug)]
struct Update(octocrab::models::repos::Release);

const TARGET: &str = env!("TARGET");

impl Update {
    fn version(&self) -> String {
        self.0.tag_name.clone()
    }

    fn get_asset(&self) -> Result<&Asset, Error> {
        self.0
            .assets
            .iter()
            .find(|assert| assert.name.contains(&TARGET))
            .ok_or(Error::HostNotSupported)
    }

    #[tokio::main(flavor = "current_thread")]
    async fn download(&self) -> Result<Vec<u8>, Error> {
        let asset = self.get_asset()?;
        let total_size = asset.size.try_into().unwrap();
        let mut data = Vec::with_capacity(asset.size as usize);
        let mut download = reqwest::get(asset.browser_download_url.clone()).await?;

        println!("Downloading update...");
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.green/white}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("=>-"));
        while let Some(chunk) = download.chunk().await? {
            data.extend_from_slice(&chunk);
            pb.inc(chunk.len() as u64);
        }

        pb.finish_with_message("downloaded");
        println!("Update downloaded!");

        Ok(data)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn get_update(owner: &str, repo: &str) -> Result<Update, Error> {
    octocrab::instance()
        .repos(owner, repo)
        .releases()
        .get_latest()
        .await
        .map(Update)
        .map_err(Error::from)
}

#[tokio::main(flavor = "current_thread")]
async fn get_update_by_tag(owner: &str, repo: &str, tag: &str) -> Result<Update, Error> {
    octocrab::instance()
        .repos(owner, repo)
        .releases()
        .get_by_tag(tag)
        .await
        .map(Update)
        .map_err(Error::from)
}

pub fn check_std_installed() -> Result<(), Error> {
    if get_version_file().exists() {
        Ok(())
    } else {
        let should_install = dialoguer::Confirm::new()
            .with_prompt("The skyline-rs toolchain is not installed. Would you like to install it?")
            .default(true)
            .interact()
            .unwrap();
        if should_install {
            update_std("skyline-rs/rust", None)
        } else {
            std::process::exit(1);
        }
    }
}

fn rustup_toolchain_link(name: &str, path: &Path) -> Result<(), Error> {
    let status = Command::new("rustup")
        .args(&["toolchain", "link", name])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| Error::RustupNotFound)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::RustupLinkFailed)
    }
}

pub fn update_std(repo: &str, tag: Option<&str>) -> Result<(), Error> {
    let components: Vec<&str> = repo.split('/').collect();
    let [owner, repo]: [&str; 2] = components.try_into().map_err(|_| Error::InvalidRepo)?;
    let update = if let Some(tag) = tag {
        get_update_by_tag(owner, repo, tag)?
    } else {
        get_update(owner, repo)?
    };

    if get_current_version() != Some(update.version()) {
        let zip_bytes = update.download()?;

        // remove old toolchain if it exists
        let _ = fs::remove_dir_all(get_toolchain());

        let toolchain = get_toolchain();

        let mut zip = ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        for i in 0..zip.len() {
            let mut file_in_zip = zip.by_index(i).unwrap();
            let out_path = toolchain.join(file_in_zip.name());
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut file = fs::File::create(&out_path).unwrap();

            std::io::copy(&mut file_in_zip, &mut file).expect("Failed to write to file");

            #[cfg(unix)]
            if !out_path
                .extension()
                .map(|ext| ext.to_str() == Some("rlib"))
                .unwrap_or(false)
            {
                file.set_permissions(fs::Permissions::from_mode(0o775))
                    .unwrap();
            }
        }

        fs::write(get_version_file(), update.version()).expect("Failed to write version file");

        rustup_toolchain_link("skyline", &toolchain)?;
    } else {
        println!("The latest version of the toolchain is already installed.")
    }

    Ok(())
}

pub(crate) trait PathExt: Sized {
    fn ensure_exists(self) -> Self;
    fn push_join<P: AsRef<Path>>(self, join: P) -> Self;
    fn if_exists(self) -> Option<Self>;
}

impl PathExt for PathBuf {
    fn ensure_exists(self) -> Self {
        if !self.exists() {
            fs::create_dir_all(&self).unwrap();
        }

        self
    }

    fn push_join<P: AsRef<Path>>(mut self, join: P) -> Self {
        self.push(join);

        self
    }

    fn if_exists(self) -> Option<Self> {
        if self.exists() {
            Some(self)
        } else {
            None
        }
    }
}
