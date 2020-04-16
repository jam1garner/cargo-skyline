use serde::Deserialize;
use crate::error:: Result;

#[derive(Deserialize)]
pub struct Metadata {
    pub title_id: Option<String>
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

pub fn get_metadata() -> Result<Metadata> {
    let metadata = cargo_metadata::MetadataCommand::new().exec()?;

    let title_id =
        metadata.packages.iter()
            .fold(None, |x, y| x.or_else(||{
                get_title_id(&y.metadata)
            }));

    Ok(Metadata{
        title_id
    })
}