pub mod calibre;

use std::{fs::File, io};

use anyhow::Result;
use libeh::dto::{
    api::GalleryMetadata,
    gallery::{category::Category, detail::GalleryDetail},
    keyword::Keyword,
};
use zip::ZipArchive;

pub enum Gallery {
    Detail(GalleryDetail),
    Metadata(GalleryMetadata),
}

const fn parse_category(category: &Category) -> Option<&str> {
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

fn parse_category_str(category: &str) -> Option<&str> {
    match category {
        "Misc" => Some("misc"),
        "Doujinshi" => Some("doujinshi"),
        "Manga" => Some("manga"),
        "Artist CG" => Some("artistcg"),
        "Game CG" => Some("gamecg"),
        "Image Set" => Some("imageset"),
        "Cosplay" => Some("cosplay"),
        "Non-H" => Some("non-h"),
        "Western" => Some("western"),
        "private" => Some("private"),
        _ => None,
    }
}

fn parse_tag(tag: &Keyword) -> Option<(&str, &str)> {
    match tag {
        Keyword::Normal(_) => None,
        Keyword::Language(k) => Some(("language", k)),
        Keyword::Parody(k) => Some(("parody", k)),
        Keyword::Character(k) => Some(("character", k)),
        Keyword::Artist(k) => Some(("artist", k)),
        Keyword::Cosplayer(k) => Some(("cosplayer", k)),
        Keyword::Group(k) => Some(("group", k)),
        Keyword::Female(k) => Some(("female", k)),
        Keyword::Male(k) => Some(("male", k)),
        Keyword::Mixed(k) => Some(("mixed", k)),
        Keyword::Other(k) => Some(("other", k)),
        Keyword::Reclass(k) => Some(("reclass", k)),
        Keyword::Temp(_) => None,
        Keyword::Uploader(k) => Some(("uploader", k)),
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
                    let output_path = format!("{output_dir}/cover.{ext}");
                    let mut output_file = File::create(&output_path)?;
                    io::copy(&mut file, &mut output_file)?;
                    return Ok(Some((file.name().to_string(), output_path)));
                }
            }
        }
    }
    Ok(None)
}
