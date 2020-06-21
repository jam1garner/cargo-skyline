use serde::Deserialize;
use crate::error:: Result;

#[derive(Deserialize)]
pub struct Metadata {
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

pub fn get_metadata() -> Result<Metadata> {
    let metadata = cargo_metadata::MetadataCommand::new().exec()?;

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
        title_id,
        npdm_path,
        subsdk_name,
        plugin_dependencies
    })
}
