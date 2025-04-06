use crate::build;
use crate::cargo_info;
use crate::error::{Error, Result};
use crate::game_paths::{get_npdm_path, get_plugin_nro_path, get_subsdk_path};
use owo_colors::OwoColorize;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::result::Result as StdResult;
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter};

pub struct Exefs {
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

    Ok(Exefs { subsdk1 })
}

pub fn package(
    skyline_url: &str,
    title_id: Option<&str>,
    out_path: &str,
    include_skyline: bool,
    subsdk: bool,
) -> Result<()> {
    let args = vec![String::from("--release")];

    let binary_path = if subsdk {
        build::build_get_nso(args)?
    } else {
        build::build_get_nro(args)?
    };

    let plugin_name = binary_path.file_name().unwrap().to_string_lossy();
    println!("Built {:?}!", plugin_name);

    let metadata = cargo_info::get_metadata()?;

    let title_id = title_id
        .or(metadata.title_id.as_deref())
        .ok_or(Error::NoTitleId)?;

    let exefs = if include_skyline && !subsdk {
        println!("Downloading latest Skyline release...");
        Some(get_exefs(skyline_url)?)
    } else {
        None
    };

    println!("Building Zip File...");
    let binary_data = fs::read(&binary_path)?;

    let mut zip = ZipWriter::new(fs::File::create(out_path)?);

    let binary_install_path = if subsdk {
        get_subsdk_path(title_id, metadata.subsdk_name.as_deref().unwrap_or("subsdk9"))[1..].to_string()
    } else {
        get_plugin_nro_path(title_id, plugin_name.as_ref())[1..].to_string()
    };

    zip.start_file(
        binary_install_path,
        Default::default(),
    )?;

    zip.write_all(&binary_data)?;

    // main.npdm
    let main_npdm = metadata
        .npdm_path
        .as_ref()
        .map(fs::read)
        .transpose()
        .map_err(|_| Error::NoNpdmFileFound)?;

    // Assuming we are building a subsdk, there are few reasons to also want Skyline
    if include_skyline && !subsdk {
        let generated_npdm = crate::installer::generate_npdm(title_id);
        
        main_npdm.as_ref().unwrap_or_else(|| {
            eprintln!("\n{}: defaulting to a generated NPDM.", "Warning".yellow());
            eprintln!(
                "{}: To specify a custom npdm add the following to your Cargo.toml:",
                "NOTE".bright_blue()
            );
            eprintln!("\n{}\n", "[package.metadata.skyline]".bright_blue());
            eprintln!("{}\n", "custom-npdm = \"path/to/your.npdm\"".bright_blue());
            &generated_npdm
        });

        // subsdk
        let subsdk_name = metadata.subsdk_name.as_deref().unwrap_or("subsdk9");
        zip.start_file(
            get_subsdk_path(title_id, subsdk_name)[1..].to_string(),
            Default::default(),
        )?;
        zip.write_all(&exefs.unwrap().subsdk1)?;
    }

    // We could want a custom NPDM instead of Skyline's, so write it here
    if let Some(main_npdm) = main_npdm {
        zip.start_file(get_npdm_path(title_id)[1..].to_string(), Default::default())?;
        zip.write_all(&main_npdm)?;
    }

    for resource in &metadata.package_resources {
        let local_path = &resource.local_path;
        let output_path = &resource.package_path;

        if Path::new(&local_path).is_dir() {
            // Get all files in the directory and subdirectories
            let paths: Vec<PathBuf> = WalkDir::new(&local_path)
                .into_iter()
                .flatten()
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| entry.path().to_owned())
                .collect();

            for path in paths {
                // Strip the local directory from the path we're processing and add the destination directory as prefix
                zip.start_file(
                    output_path
                        .join(&path.strip_prefix(&local_path).unwrap())
                        .to_str()
                        .unwrap(),
                    Default::default(),
                )?;

                zip.write_all(
                    &std::fs::read(&path).map_err(|_| Error::PackageResourceMissing(path))?,
                )?;
            }
        } else {
            zip.start_file(output_path.to_str().unwrap(), Default::default())?;

            zip.write_all(
                &std::fs::read(&local_path)
                    .map_err(|_| Error::PackageResourceMissing(local_path.to_owned()))?,
            )?;
        }
    }

    println!("Finished building zip at '{}'", out_path);

    Ok(())
}
