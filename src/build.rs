use std::process::{Command, Stdio};
use cargo_metadata::Message;
use crate::error::{Result, Error};
use std::path::PathBuf;
use std::env;
use linkle::format::nxo::NxoFile;

const XARGO_GIT_URL: &str = "https://github.com/jam1garner/xargo";

fn get_toolchain_bin_dir() -> Result<PathBuf> {
    let rel_path = if cfg!(windows) {
        r".rustup\toolchains\*\lib\rustlib\*\bin\"
    } else {
        r".rustup/toolchains/*/lib/rustlib/*/bin/"
    };

    Ok(
        dirs::home_dir()
            .ok_or(Error::NoHomeDir)?
            .join(rel_path)
    )
}

pub fn build_get_artifact(args: Vec<String>) -> Result<PathBuf> {
    // Ensure rust-lld is added to the PATH on Windows
    if !Command::new("rust-lld").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() || cfg!(windows) {
        let toolchain_bin_dir = get_toolchain_bin_dir()?;

        let paths = env::var_os("PATH").ok_or(Error::NoPathFound)?;
        
        let mut split_paths = env::split_paths(&paths).collect::<Vec<_>>();
        split_paths.push(toolchain_bin_dir);

        let new_path = env::join_paths(split_paths).unwrap();

        env::set_var("PATH", &new_path);
    }

    if !Command::new("xargo").stdout(Stdio::null()).status().is_ok() {
        match Command::new("cargo")
                    .args(&["install", "--git", XARGO_GIT_URL, "--force"])
                    .stdout(Stdio::piped())
                    .status()
                    .unwrap()
                    .code() {
            Some(0) => {},
            x @ Some(_) | x @ None => {
                std::process::exit(x.unwrap_or(1));
            }
        }
    }

    let current_dir = std::env::current_dir()?;

    println!("{}", current_dir.display());

    let mut command =
        Command::new("xargo")
            .args(&[
                "build", "--message-format=json-diagnostic-rendered-ansi", "--color", "always"
            ])
            .args(args)
            .current_dir(env::current_dir()?)
            // Needed to make crates.io crates use the custom target
            .env("RUST_TARGET_PATH", current_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

    let last_artifact =
        cargo_metadata::parse_messages(command.stdout.as_mut().unwrap())
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
        Ok(artifact.filenames[0].clone())
    } else {
        return Err(Error::FailParseCargoStream)
    }
}

pub fn build_get_nro(args: Vec<String>) -> Result<PathBuf> {
    let artifact = build_get_artifact(args)?;

    let nro_path = artifact.with_extension("nro");
    
    NxoFile::from_elf(artifact.to_str().ok_or(Error::FailWriteNro)?)?
        .write_nro(
            &mut std::fs::File::create(&nro_path).map_err(|_| Error::FailWriteNro)?,
            None,
            None,
            None
        )?;
        

    Ok(nro_path)
}

pub fn build(mut args: Vec<String>, release: bool) -> Result<()> {
    if release {
        args.push("--release".into());
    }
    build_get_nro(args)?;

    Ok(())
}
