use crate::build;
use crate::cargo_info;
use crate::error::{Error, Result};
use crate::game_paths::{get_npdm_path, get_plugin_nro_path, get_subsdk_path};
use owo_colors::OwoColorize;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::result::Result as StdResult;
use zip::{ZipArchive, ZipWriter};

pub struct Exefs {
    pub main_npdm: Vec<u8>,
    pub subsdk1: Vec<u8>,
}

// TODO: Cache exefs to disk, figure out some strategy for cache invalidation?
pub fn get_exefs(url: &str) -> Result<Exefs> {
    let zip_reader = Cursor::new(
        reqwest::blocking::get(url)
            .map_err(|_| Error::DownloadError)?
            .bytes()
            .map_err(|_| Error::DownloadError)?,
    );

    let mut zip = ZipArchive::new(zip_reader).unwrap();

    let subsdk1 = zip
        .by_name("exefs/subsdk9")?
        .bytes()
        .collect::<StdResult<_, _>>()?;
    let main_npdm = zip
        .by_name("exefs/main.npdm")?
        .bytes()
        .collect::<StdResult<_, _>>()?;

    Ok(Exefs { main_npdm, subsdk1 })
}

pub fn package(
    skyline_url: &str,
    title_id: Option<&str>,
    out_path: &str,
    include_skyline: bool,
) -> Result<()> {
    let args = vec![String::from("--release")];
    let nro_path = build::build_get_nro(args)?;
    let plugin_name = nro_path.file_name().unwrap().to_string_lossy();
    println!("Built {:?}!", plugin_name);

    let metadata = cargo_info::get_metadata()?;

    let title_id = title_id
        .or_else(|| metadata.title_id.as_deref())
        .ok_or(Error::NoTitleId)?;

    let exefs = if include_skyline {
        println!("Downloading latest Skyline release...");
        Some(get_exefs(skyline_url)?)
    } else {
        None
    };

    println!("Building Zip File...");
    let plugin_data = fs::read(&nro_path)?;

    let mut zip = ZipWriter::new(fs::File::create(out_path)?);

    zip.start_file(
        get_plugin_nro_path(title_id, plugin_name.as_ref())[1..].to_string(),
        Default::default(),
    )?;
    zip.write_all(&plugin_data)?;

    if include_skyline {
        // main.npdm
        let main_npdm = metadata
            .npdm_path
            .as_ref()
            .map(|path| fs::read(path))
            .transpose()
            .map_err(|_| Error::NoNpdmFileFound)?;
        let generated_npdm = crate::installer::generate_npdm(&title_id);
        let main_npdm = main_npdm.as_ref().unwrap_or_else(|| {
            eprintln!("\n{}: defaulting to a generated NPDM.", "Warning".yellow());
            eprintln!(
                "{}: To specify a custom npdm add the following to your Cargo.toml:",
                "NOTE".bright_blue()
            );
            eprintln!("\n{}\n", "[package.metadata.skyline]".bright_blue());
            eprintln!("{}\n", "custom-npdm = \"path/to/your.npdm\"".bright_blue());
            &generated_npdm
        });
        zip.start_file(get_npdm_path(title_id)[1..].to_string(), Default::default())?;
        zip.write_all(main_npdm)?;

        // subsdk
        let subsdk_name = metadata.subsdk_name.as_deref().unwrap_or("subsdk9");
        zip.start_file(
            get_subsdk_path(title_id, subsdk_name)[1..].to_string(),
            Default::default(),
        )?;
        zip.write_all(&exefs.unwrap().subsdk1)?;
    }

    println!("Finished building zip at '{}'", out_path);

    Ok(())
}
