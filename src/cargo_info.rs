use serde::Deserialize;
use crate::error::Result;

#[derive(Deserialize)]
pub struct Metadata {
    pub name: String,
    pub title_id: Option<String>,
    pub npdm_path: Option<String>,
    pub subsdk_name: Option<String>,
    pub plugin_dependencies: Vec<Dependency>,
}

#[derive(Deserialize, Debug)]
pub struct Dependency {
    pub name: String,
    pub url: String
}

fn get_title_id(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("titleid")?
            .as_str()?
            .into()
    )
}

fn get_npdm_path(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("custom-npdm")?
            .as_str()?
            .into()
    )
}

fn get_subsdk_name(md: &serde_json::Value) -> Option<String> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("subsdk-name")?
            .as_str()?
            .into()
    )
}

fn get_dep_urls(md: &serde_json::Value) -> Option<Vec<Dependency>> {
    Some(
        md.get("skyline")?
            .as_object()?
            .get("plugin-dependencies")?
            .as_array()?
            .into_iter()
            .map(|x|{
                let dep = x.as_object().unwrap();
                let name = dep.get("name").unwrap().as_str().unwrap().into();
                let url = dep.get("url").unwrap().as_str().unwrap().into();
                Dependency { name, url }
            })
            .collect()
    )
}

use cargo_metadata::MetadataCommand;

pub fn get_metadata() -> Result<Metadata> {
    let output = MetadataCommand::new()
        //.other_options(["--target".to_string(), "aarch64-skyline-switch".to_string()])
        .cargo_command()?
        .env("RUSTUP_TOOLCHAIN", "skyline")
        .output()?;

    if !output.status.success() {
        return Err(cargo_metadata::Error::CargoMetadata {
            stderr: String::from_utf8(output.stderr).unwrap(),
        }.into());
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .find(|line| line.starts_with('{'))
        .ok_or_else(|| cargo_metadata::Error::NoJson)?;

    let metadata = MetadataCommand::parse(stdout)?;

    let name = metadata.workspace_members.first()
        .unwrap()
        .repr.split(" ").next()
        .unwrap()
        .to_string();

    let title_id =
        metadata.packages.iter()
            .fold(None, |x, y| x.or_else(||{
                get_title_id(&y.metadata)
            }));
    
    let npdm_path =
        metadata.packages.iter()
            .fold(None, |x, y| x.or_else(||{
                get_npdm_path(&y.metadata)
            }));
    
    let subsdk_name =
        metadata.packages.iter()
            .fold(None, |x, y| x.or_else(||{
                get_subsdk_name(&y.metadata)
            }));

    let plugin_dependencies =
        metadata.packages.iter()
            .fold(vec![], |mut x, y| {
                x.append(
                    &mut get_dep_urls(&y.metadata).unwrap_or_default()
                );
                x
            });

    Ok(Metadata {
        name,
        title_id,
        npdm_path,
        subsdk_name,
        plugin_dependencies
    })
}
