pub mod calibre;

use std::{fs::File, io};

use anyhow::Result;
use libeh::dto::{gallery::category::Category, keyword::Keyword};
use zip::ZipArchive;

fn parse_category(category: &Category) -> Option<&str> {
    match category {
        Category::None => None,
        Category::Misc => Some("misc"),
        Category::Doujinshi => Some("doujinshi"),
        Category::Manga => Some("manga"),
        Category::ArtistCG => Some("artistcg"),
        Category::GameCG => Some("gamecg"),
        Category::ImageSet => Some("imageset"),
        Category::Cosplay => Some("cosplay"),
        Category::NonH => Some("non-h"),
        Category::Western => Some("western"),
        Category::All => None,
        Category::Private => Some("private"),
        Category::Unknown => None,
    }
}

fn parse_tag(tag: &Keyword) -> (Option<&str>, Option<&str>) {
    match tag {
        Keyword::Normal(_) => (None, None),
        Keyword::Language(k) => (Some("language"), Some(k)),
        Keyword::Parody(k) => (Some("parody"), Some(k)),
        Keyword::Character(k) => (Some("character"), Some(k)),
        Keyword::Artist(k) => (Some("artist"), Some(k)),
        Keyword::Cosplayer(k) => (Some("cosplayer"), Some(k)),
        Keyword::Group(k) => (Some("group"), Some(k)),
        Keyword::Female(k) => (Some("female"), Some(k)),
        Keyword::Male(k) => (Some("male"), Some(k)),
        Keyword::Mixed(k) => (Some("mixed"), Some(k)),
        Keyword::Other(k) => (Some("other"), Some(k)),
        Keyword::Reclass(k) => (Some("reclass"), Some(k)),
        Keyword::Temp(_) => (None, None),
        Keyword::Uploader(k) => (Some("uploader"), Some(k)),
    }
}

pub fn extract_cover(cbz_path: &str, output_dir: &str) -> Result<Option<(String, String)>> {
    let file = File::open(cbz_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let path = file.mangled_name();

        if let Some(ext) = path.extension() {
            if let Some(ext) = ext.to_str() {
                let ext = ext.to_lowercase();
                if ext == "jpg" || ext == "jpeg" || ext == "png" {
                    let output_path = format!("{}/cover.{}", output_dir, ext);
                    let mut output_file = File::create(&output_path)?;
                    io::copy(&mut file, &mut output_file)?;
                    return Ok(Some((file.name().to_string(), output_path)));
                }
            }
        }
    }
    Ok(None)
}
