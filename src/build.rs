use crate::error::{Error, Result};
use crate::update_std::target_json_path;
use cargo_metadata::Message;
use linkle::format::nxo::NxoFile;
use std::env;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub(crate) fn get_rustup_home() -> Result<PathBuf> {
    env::var("RUSTUP_HOME").map(PathBuf::from).or_else(|_| {
        dirs::home_dir()
            .map(|home| home.join(".rustup"))
            .ok_or(Error::NoHomeDir)
    })
}

fn get_toolchain_bin_dir() -> Result<PathBuf> {
    let rel_path = if cfg!(windows) {
        r"toolchains\*\lib\rustlib\*\bin\"
    } else {
        r"toolchains/*/lib/rustlib/*/bin/"
    };

    let search_path = get_rustup_home()?.join(rel_path);

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

#[derive(Copy, Clone, PartialEq, Eq)]
enum CargoCommand {
    Build,
    Check,
    Clippy,
    Doc,
}

impl CargoCommand {
    fn to_str(self) -> &'static str {
        match self {
            CargoCommand::Build => "build",
            CargoCommand::Check => "check",
            CargoCommand::Clippy => "clippy",
            CargoCommand::Doc => "doc",
        }
    }
}

pub fn check(json: bool) -> Result<()> {
    cargo_run_command(CargoCommand::Check, Vec::new(), json).map(|_| ())
}

pub fn clippy(args: Vec<String>, json: bool) -> Result<()> {
    cargo_run_command(CargoCommand::Clippy, args, json).map(|_| ())
}

pub fn build_get_artifacts(args: Vec<String>) -> Result<Vec<PathBuf>> {
    let cargo_output = cargo_run_command(CargoCommand::Build, args, false)?;
    let artifact_paths: Vec<_> = cargo_output
        .into_iter()
        .filter_map(|message| {
            if let Message::CompilerArtifact(artifact) = message {
                if artifact.target.kind.iter().any(|kind| kind == "cdylib") {
                    Some(artifact.filenames[0].clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if artifact_paths.is_empty() {
        return Err(Error::FailParseCargoStream);
    }

    Ok(artifact_paths)
}

fn cargo_run_command(
    command: CargoCommand,
    args: Vec<String>,
    print_cargo_messages: bool,
) -> Result<Vec<Message>> {
    crate::update_std::check_std_installed()?;

    let target_json_path = target_json_path();

    // Ensure rust-lld is added to the PATH on Windows
    if Command::new("rust-lld")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
        || cfg!(windows)
    {
        let toolchain_bin_dir = get_toolchain_bin_dir()?;

        let paths = env::var_os("PATH").ok_or(Error::NoPathFound)?;

        let mut split_paths = env::split_paths(&paths).collect::<Vec<_>>();
        split_paths.push(toolchain_bin_dir);

        let new_path = env::join_paths(split_paths).unwrap();

        env::set_var("PATH", &new_path);
    }

    // rustup run skyline-v3 SKYLINE_ADD_NRO_HEADER=1 RUSTFLAGS="--cfg skyline_std_v3" cargo build --target ~/.cargo/skyline/aarch64-skyline-switch.json -Z build-std=core,alloc,std,panic_abort
    let mut command = Command::new("rustup")
        .arg("run")
        .arg("skyline-v3")
        .arg("cargo")
        .args(&[
            command.to_str(),
            "--message-format=json-diagnostic-rendered-ansi",
            "--color",
            "always",
            "--target",
        ])
        .arg(&target_json_path)
        .args(&["-Z", "build-std=core,alloc,std,panic_abort"])
        .args(args)
        .env("SKYLINE_ADD_NRO_HEADER", "1")
        .env("RUSTFLAGS", "--cfg skyline_std_v3")
        .current_dir(env::current_dir()?)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let cargo_messages = BufReader::new(command.stdout.as_mut().unwrap())
        .lines()
        .inspect(|line| {
            if print_cargo_messages {
                if let Ok(msg) = line {
                    println!("{}", msg)
                }
            }
        })
        .map(|line| {
            // Inlined implementation of cargo_metadata's MessageIter
            line.map(|it| serde_json::from_str(&it).unwrap_or(Message::TextLine(it)))
        })
        .inspect(|message| {
            if let Ok(Message::CompilerMessage(compiler_message)) = message {
                if let Some(msg) = &compiler_message.message.rendered {
                    println!("{}", msg);
                }
            }
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::FailParseCargoStream)?;

    let exit_status = command.wait().unwrap();

    if !exit_status.success() {
        Err(Error::ExitStatus(exit_status.code().unwrap_or(1)))
    } else {
        Ok(cargo_messages)
    }
}

pub fn build_get_nros(args: Vec<String>) -> Result<Vec<PathBuf>> {
    let artifacts = build_get_artifacts(args)?;

    let mut nro_paths = Vec::with_capacity(artifacts.len());

    for artifact in &artifacts {
        let path = artifact.with_extension("nro");

        NxoFile::from_elf(artifact.to_str().ok_or(Error::FailWriteNro)?)?.write_nro(
            &mut std::fs::File::create(&path).map_err(|_| Error::FailWriteNro)?,
            None,
            None,
            None,
        )?;

        nro_paths.push(path);
    }

    Ok(nro_paths)
}

pub fn build_get_nsos(args: Vec<String>) -> Result<Vec<PathBuf>> {
    let artifacts = build_get_artifacts(args)?;

    let mut nso_paths = Vec::with_capacity(artifacts.len());

    for artifact in &artifacts {
        let path = artifact.with_extension("nso");

        NxoFile::from_elf(artifact.to_str().ok_or(Error::FailWriteNro)?)?
            .write_nso(&mut std::fs::File::create(&path).map_err(|_| Error::FailWriteNro)?)?;

        nso_paths.push(path);
    }

    Ok(nso_paths)
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
        build_get_nsos(args)?;
    } else {
        build_get_nros(args)?;
    }

    Ok(())
}

pub fn doc(args: Vec<String>) -> Result<()> {
    cargo_run_command(CargoCommand::Doc, args, false)?;

    Ok(())
}
