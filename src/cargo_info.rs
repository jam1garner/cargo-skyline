use std::path::PathBuf;

use crate::error::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metadata {
    pub name: String,
    pub title_id: Option<String>,
    pub npdm_path: Option<String>,
    pub subsdk_name: Option<String>,
    pub plugin_dependencies: Vec<Dependency>,
    pub package_resources: Vec<PackageResource>,
}

#[derive(Deserialize, Debug)]
pub struct Dependency {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct PackageResource {
    pub local_path: PathBuf,
    pub package_path: PathBuf,
}

fn get_title_id(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("titleid")?
            .as_str()?
            .into(),
    )
}

fn get_npdm_path(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("custom-npdm")?
            .as_str()?
            .into(),
    )
}

fn get_subsdk_name(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("subsdk-name")?
            .as_str()?
            .into(),
    )
}

fn get_dep_urls(md: &serde_json::Value) -> Option<Vec<Dependency>> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("plugin-dependencies")?
            .as_array()?
            .iter()
            .map(|x| {
                let dep = x.as_object().unwrap();
                let name = dep.get("name").unwrap().as_str().unwrap().into();
                let url = dep.get("url").unwrap().as_str().unwrap().into();
                Dependency { name, url }
            })
            .collect(),
    )
}

fn get_package_deps(md: &serde_json::Value) -> Option<Vec<PackageResource>> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("package-resources")?
            .as_array()?
            .iter()
            .map(|x| {
                let dep = x.as_object().unwrap();
                let local_path = dep.get("local").unwrap().as_str().unwrap().into();
                let package_path = dep.get("package").unwrap().as_str().unwrap().into();
                PackageResource { local_path, package_path }
            })
            .collect(),
    )
}

use cargo_metadata::MetadataCommand;

pub fn get_metadata() -> Result<Metadata> {
    let output = MetadataCommand::new()
        //.other_options(["--target".to_string(), "aarch64-skyline-switch".to_string()])
        .cargo_command()?
        .env("RUSTUP_TOOLCHAIN", "skyline-v3")
        .output()?;

    if !output.status.success() {
        return Err(cargo_metadata::Error::CargoMetadata {
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
        .into());
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .find(|line| line.starts_with('{'))
        .ok_or(cargo_metadata::Error::NoJson)?;

    let metadata = MetadataCommand::parse(stdout)?;

    let name = metadata
        .workspace_members
        .first()
        .unwrap()
        .repr
        .split(' ')
        .next()
        .unwrap()
        .to_string();

    let title_id = metadata
        .packages
        .iter()
        .fold(None, |x, y| x.or_else(|| get_title_id(&y.metadata)));

    let npdm_path = metadata
        .packages
        .iter()
        .fold(None, |x, y| x.or_else(|| get_npdm_path(&y.metadata)));

    let subsdk_name = metadata
        .packages
        .iter()
        .fold(None, |x, y| x.or_else(|| get_subsdk_name(&y.metadata)));

    let plugin_dependencies = metadata.packages.iter().fold(vec![], |mut x, y| {
        x.append(&mut get_dep_urls(&y.metadata).unwrap_or_default());
        x
    });

    let package_resources = metadata.packages.iter().fold(vec![], |mut x, y| {
        x.append(&mut get_package_deps(&y.metadata).unwrap_or_default());
        x
    });

    Ok(Metadata {
        name,
        title_id,
        npdm_path,
        subsdk_name,
        plugin_dependencies,
        package_resources,
    })
}
