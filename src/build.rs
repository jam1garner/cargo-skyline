use std::process::{Command, Stdio};
use cargo_metadata::Message;
use crate::error::{Result, Error};
use std::path::PathBuf;
use linkle::format::nxo::NxoFile;

const XARGO_GIT_URL: &str = "https://github.com/jam1garner/xargo";

pub fn build_get_artifact(args: Vec<String>) -> Result<PathBuf> {
    if !Command::new("xargo").stdout(Stdio::null()).status().map(|x| x.success()).unwrap_or_default() {
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

    let mut command =
        Command::new("xargo")
            .args(&[
                "build", "--message-format=json", "--color", "always"
            ])
            .args(args)
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