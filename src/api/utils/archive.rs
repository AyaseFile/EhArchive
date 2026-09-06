use std::{
    fs::{self, File, OpenOptions},
    io::{Write, copy},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::comic_info::{self, ComicMetadata};

pub fn find_archive(output_dir: &Path, gid: i64, token: &str) -> Result<Option<PathBuf>> {
    if !output_dir.exists() {
        return Ok(None);
    }
    let prefix = format!("{gid}_{token}_");
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        let matches = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&prefix))
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cbz"));
        if matches {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub fn build_archive(
    source: File,
    output_dir: &Path,
    identifier: &str,
    metadata: &ComicMetadata,
) -> Result<String> {
    fs::create_dir_all(output_dir)?;
    let output_path = output_dir.join(format!("{identifier}.cbz"));
    let partial = output_dir.join(format!(".{identifier}.cbz.partial"));

    if output_path.exists() {
        return Ok(output_path.to_string_lossy().into_owned());
    }

    if partial.exists() {
        fs::remove_file(&partial).context("Failed to remove stale partial archive")?;
    }
    let comic_info = comic_info::serialize_comic_info(metadata)?;
    if let Err(e) = write_archive(source, &partial, &comic_info) {
        let _ = fs::remove_file(&partial);
        return Err(e);
    }

    fs::rename(&partial, &output_path)?;
    Ok(output_path.to_string_lossy().into_owned())
}

fn write_archive(source: File, partial: &Path, xml: &[u8]) -> Result<()> {
    let mut input = ZipArchive::new(source).context("Input is not a valid ZIP archive")?;
    let output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(partial)?;
    let mut output = ZipWriter::new(output);

    for index in 0..input.len() {
        let mut entry = input.by_index(index)?;
        let name = entry.name().to_string();
        ensure!(
            name != "ComicInfo.xml",
            "Input ZIP already contains ComicInfo.xml at its root"
        );
        let mut options =
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        if let Some(last_modified) = entry.last_modified() {
            options = options.last_modified_time(last_modified);
        }
        if entry.is_dir() {
            output.add_directory(&name, options)?;
            continue;
        }
        output.start_file(&name, options)?;
        copy(&mut entry, &mut output)?;
    }

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    output.start_file("ComicInfo.xml", options)?;
    output.write_all(xml)?;
    output.finish()?;
    Ok(())
}
