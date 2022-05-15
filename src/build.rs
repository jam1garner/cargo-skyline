use crate::error::{Error, Result};
use cargo_metadata::Message;
use linkle::format::nxo::NxoFile;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn get_toolchain_bin_dir() -> Result<PathBuf> {
    let rustup_home = env::var("RUSTUP_HOME").map(PathBuf::from).or_else(|_| {
        dirs::home_dir()
            .map(|home| home.join(".rustup"))
            .ok_or(Error::NoHomeDir)
    })?;

    let rel_path = if cfg!(windows) {
        r"toolchains\*\lib\rustlib\*\bin\"
    } else {
        r"toolchains/*/lib/rustlib/*/bin/"
    };

    let search_path = rustup_home.join(rel_path);

    glob::glob(
        search_path
            .to_str()
            .expect("Toolchain path could not be converted to a &str"),
    )
    .unwrap()
    .next()
    .unwrap()
    .map(Ok)
    .unwrap()
}

#[derive(Copy, Clone, PartialEq)]
enum CargoCommand {
    Rustc,
    Check,
    Clippy,
    Doc,
}

impl CargoCommand {
    fn to_str(self) -> &'static str {
        match self {
            CargoCommand::Rustc => "rustc",
            CargoCommand::Check => "check",
            CargoCommand::Clippy => "clippy",
            CargoCommand::Doc => "doc",
        }
    }
}

pub fn check() -> Result<()> {
    cargo_run_command(CargoCommand::Check, Vec::new()).map(|_| ())
}

pub fn clippy() -> Result<()> {
    cargo_run_command(CargoCommand::Clippy, Vec::new()).map(|_| ())
}

pub fn build_get_artifact(args: Vec<String>) -> Result<PathBuf> {
    cargo_run_command(CargoCommand::Rustc, args)?.ok_or(Error::FailParseCargoStream)
}

fn cargo_run_command(command: CargoCommand, args: Vec<String>) -> Result<Option<PathBuf>> {
    crate::update_std::check_std_installed()?;

    // Ensure rust-lld is added to the PATH on Windows
    if !Command::new("rust-lld")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
        || cfg!(windows)
    {
        let toolchain_bin_dir = get_toolchain_bin_dir()?;

        let paths = env::var_os("PATH").ok_or(Error::NoPathFound)?;

        let mut split_paths = env::split_paths(&paths).collect::<Vec<_>>();
        split_paths.push(toolchain_bin_dir);

        let new_path = env::join_paths(split_paths).unwrap();

        env::set_var("PATH", &new_path);
    }

    let rustc_options = if command == CargoCommand::Rustc {
        &["-C", "link-arg=-Tlink.ld"][..]
    } else {
        &[][..]
    };

    // cargo +skyline rustc --target aarch64-skyline-switch -- -C link-arg=-Tlink.ld
    let mut command = Command::new("cargo")
        .arg("+skyline")
        .args(&[
            command.to_str(),
            "--message-format=json-diagnostic-rendered-ansi",
            "--color",
            "always",
            "--target",
            "aarch64-skyline-switch",
        ])
        .args(args)
        .arg("--")
        .args(rustc_options)
        .env("SKYLINE_ADD_NRO_HEADER", "1")
        .current_dir(env::current_dir()?)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let last_artifact = cargo_metadata::parse_messages(command.stdout.as_mut().unwrap())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::FailParseCargoStream)?
        .into_iter()
        .filter_map(|message| {
            if let Message::CompilerArtifact(artifact) = message {
                Some(artifact)
            } else if let Message::CompilerMessage(message) = message {
                if let Some(msg) = message.message.rendered {
                    println!("{}", msg);
                }

                None
            } else {
                None
            }
        })
        .last();

    let exit_status = command.wait().unwrap();

    if !exit_status.success() {
        Err(Error::ExitStatus(exit_status.code().unwrap_or(1)))
    } else if let Some(artifact) = last_artifact {
        Ok(Some(artifact.filenames[0].clone()))
    } else {
        Ok(None)
    }
}

pub fn build_get_nro(args: Vec<String>) -> Result<PathBuf> {
    let artifact = build_get_artifact(args)?;

    let nro_path = artifact.with_extension("nro");

    NxoFile::from_elf(artifact.to_str().ok_or(Error::FailWriteNro)?)?.write_nro(
        &mut std::fs::File::create(&nro_path).map_err(|_| Error::FailWriteNro)?,
        None,
        None,
        None,
    )?;

    Ok(nro_path)
}

pub fn build_get_nso(args: Vec<String>) -> Result<PathBuf> {
    let artifact = build_get_artifact(args)?;

    let nso_path = artifact.with_extension("nso");

    NxoFile::from_elf(artifact.to_str().ok_or(Error::FailWriteNro)?)?
        .write_nso(&mut std::fs::File::create(&nso_path).map_err(|_| Error::FailWriteNro)?)?;

    Ok(nso_path)
}

pub fn build(
    mut args: Vec<String>,
    release: bool,
    nso: bool,
    features: Vec<String>,
    no_default_features: bool,
) -> Result<()> {
    if release {
        args.push("--release".into());
    }

    if !features.is_empty() {
        args.push(format!("--features={}", features.join(",")));
    }

    if no_default_features {
        args.push("--no-default-features".to_owned());
    }

    if nso {
        build_get_nso(args)?;
    } else {
        build_get_nro(args)?;
    }

    Ok(())
}

pub fn doc(args: Vec<String>) -> Result<()> {
    cargo_run_command(CargoCommand::Doc, args)?;

    Ok(())
}
