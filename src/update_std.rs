use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::build::get_rustup_home;
use crate::Error;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

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

const ORG: &str = "skyline-rs";
const REPO: &str = "rust-src";
const BRANCH: &str = "skyline";

fn url() -> String {
    format!("https://github.com/{}/{}", ORG, REPO)
}

#[tokio::main(flavor = "current_thread")]
async fn get_base_nightly() -> Result<String, Error> {
    let octocrab = octocrab::instance();

    let commit = octocrab
        .repos(ORG, REPO)
        .list_commits()
        .branch(BRANCH)
        .author("bors")
        .per_page(1)
        .send()
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NoBaseCommit)?;

    Ok(format!(
        "nightly-{}",
        commit
            .commit
            .author
            .expect("No author for the last bors commit")
            .date
            .expect("No date for last bors commit")
            .format("%Y-%m-%d")
    ))
}

fn get_original_toolchain(
    base_nightly_progress: &ProgressBar,
    progress: &ProgressBar,
    success_style: ProgressStyle,
    failed_style: ProgressStyle,
) -> Result<PathBuf, Error> {
    let base_nightly = std::thread::spawn(get_base_nightly);

    while !base_nightly.is_finished() {
        base_nightly_progress.tick();
    }

    let base_nightly = base_nightly.join().unwrap().map_err(|err| {
        base_nightly_progress.set_style(failed_style.clone());
        base_nightly_progress.finish_with_message("Failed to get find base nightly");

        err
    })?;

    let toolchain = get_rustup_home()?
        .push_join("toolchains")
        .push_join(format!("{}-{}", base_nightly, TARGET));

    progress.println(format!(
        "Using {base_nightly} as a base for installation..."
    ));

    base_nightly_progress.set_style(success_style.clone());
    base_nightly_progress.finish_with_message("Base nightly found");

    if toolchain.exists() {
        Ok(toolchain)
    } else {
        let mut rustup_cmd = Command::new("rustup")
            .args(&["toolchain", "add", &base_nightly])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|_| Error::RustupToolchainAddFailed)?;

        let err = |_| Error::RustupToolchainAddFailed;

        let status = loop {
            progress.tick();

            if let Some(status) = rustup_cmd.try_wait().map_err(err)? {
                break status;
            }
        };

        let install_succeed = status.success();

        if install_succeed {
            progress.set_style(success_style);
            progress.finish_with_message("Base toolchain downloaded");
        } else {
            progress.set_style(failed_style);
            progress.finish_with_message("Failed to get find base nightly");
        }

        (install_succeed && toolchain.exists())
            .then(|| toolchain)
            .ok_or(Error::RustupToolchainAddFailed)
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn target_json_path() -> PathBuf {
    get_cargo_skyline_dir().push_join("aarch64-skyline-switch.json")
}

fn linker_script_path() -> PathBuf {
    get_cargo_skyline_dir().push_join("link.T")
}

const LINKER_SCRIPT: &str = include_str!("link.T");

fn ensure_target_json_exists() {
    let target_json_path = target_json_path();

    let link_script_path = linker_script_path();
    if !link_script_path.exists() {
        fs::write(&link_script_path, LINKER_SCRIPT).expect("Failed to create link.T linker script");
    }

    if !target_json_path.exists() {
        fs::write(&target_json_path, target_json())
            .expect("Failed to create aarch64-skyline-switch target json");
    }
}

fn target_json() -> String {
    let linker_script = if cfg!(windows) {
        linker_script_path()
            .to_str()
            .unwrap()
            .replace('\\', "/")
            .into()
    } else {
        linker_script_path()
    };

    format!(
        r#"{{
        "arch": "aarch64",
        "crt-static-default": false,
        "crt-static-respected": false,
        "data-layout": "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32",
        "dynamic-linking": true,
        "executables": true,
        "has-rpath": false,
        "linker": "rust-lld",
        "linker-flavor": "ld.lld",
        "llvm-target": "aarch64-unknown-none",
        "max-atomic-width": 128,
        "os": "switch",
        "panic-strategy": "abort",
        "position-independent-executables": true,
        "pre-link-args": {{
          "ld.lld": [
            "-T{linker_script}",
            "-init=__custom_init",
            "-fini=__custom_fini",
            "--export-dynamic"
          ]
        }},
        "post-link-args": {{
          "ld.lld": [
            "--no-gc-sections",
            "--eh-frame-hdr"
          ]
        }},
        "relro-level": "off",
        "target-c-int-width": "32",
        "target-endian": "little",
        "target-pointer-width": "64",
        "vendor": "jam1garner"
    }}"#,
        linker_script = linker_script.to_string_lossy()
    )
}

pub fn create_modified_toolchain(deep: bool, pull: bool) -> Result<(), Error> {
    let multiprogress = MultiProgress::new();
    let style =
        ProgressStyle::default_spinner().template("{prefix:.bold.dim} {spinner} {wide_msg}");
    let finished_style =
        ProgressStyle::default_spinner().template("{prefix:.bold.dim} ✔️ {wide_msg}");
    let failed_style =
        ProgressStyle::default_spinner().template("{prefix:.bold.dim} ❌ {wide_msg}");

    let get_base_nightly_pb = multiprogress.add(
        ProgressBar::new_spinner()
            .with_message("Searching git history for base nightly")
            .with_prefix("[1/3]")
            .with_style(style.clone()),
    );

    let base_chain_pb = multiprogress.add(
        ProgressBar::new_spinner()
            .with_message("Downloading base toolchain")
            .with_prefix("[2/3]")
            .with_style(style.clone()),
    );

    let std_clone_pb = multiprogress.add(
        ProgressBar::new_spinner()
            .with_message("Downloading custom Rust standard library")
            .with_prefix("[3/3]")
            .with_style(style),
    );

    std::thread::spawn(move || multiprogress.join());

    let toolchain = get_toolchain();

    if pull {
        let pull_success = Command::new("git")
            .current_dir(toolchain.join("lib/rustlib/src/rust"))
            .args(&["pull", "--recurse-submodules", "-q"])
            .status()
            .map_err(|_| Error::GitNotInstalled)?
            .success();

        return if pull_success {
            Ok(())
        } else {
            Err(Error::StdCloneFailed)
        };
    }

    let _ = fs::remove_dir_all(&toolchain);

    let original_toolchain = get_original_toolchain(
        &get_base_nightly_pb,
        &base_chain_pb,
        finished_style.clone(),
        failed_style.clone(),
    )?;

    copy_dir_all(&original_toolchain, &toolchain).map_err(|_| Error::ToolchainCopyFailed)?;

    let src_dir = toolchain.join("lib/rustlib/src");

    if src_dir.exists() {
        let _ = fs::remove_dir_all(&src_dir);
    }

    if fs::create_dir_all(&src_dir).is_err() {
        panic!("Failed to create {:?}", &src_dir);
    }

    let src_dir = src_dir.push_join("rust");

    let mut clone_cmd = Command::new("git")
        .args(&["clone", "--recurse-submodules"])
        .args(if deep {
            &[]
        } else {
            &["--shallow-submodules", "--depth", "1"][..]
        })
        .arg("--branch")
        .arg(BRANCH)
        .arg(url())
        .arg(&src_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|_| Error::GitNotInstalled)?;

    let clone_status = loop {
        std_clone_pb.tick();

        if let Some(status) = clone_cmd.try_wait()? {
            break status;
        }
    };

    std_clone_pb.set_style(if clone_status.success() {
        finished_style
    } else {
        failed_style
    });
    std_clone_pb.finish_with_message(if clone_status.success() {
        "Finished downloading custom Rust standard library"
    } else {
        "Failed to download custom Rust standard library"
    });

    rustup_toolchain_link("skyline-v3", &toolchain)?;

    if clone_status.success() {
        Ok(())
    } else {
        Err(Error::StdCloneFailed)
    }
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

const TARGET: &str = env!("TARGET");

pub fn check_std_installed() -> Result<(), Error> {
    ensure_target_json_exists();

    if get_rustup_home()?
        .push_join("toolchains/skyline-v3")
        .exists()
    {
        Ok(())
    } else {
        let should_install = dialoguer::Confirm::new()
            .with_prompt("The skyline-rs toolchain is not installed. Would you like to install it?")
            .default(true)
            .interact()
            .unwrap();

        if should_install {
            create_modified_toolchain(false, false)
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

pub fn update_std(_repo: &str, _tag: Option<&str>, deep: bool, pull: bool) -> Result<(), Error> {
    create_modified_toolchain(deep, pull)?;

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
