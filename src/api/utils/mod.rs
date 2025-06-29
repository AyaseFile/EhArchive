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

fn parse_category(category: Category) -> Option<String> {
    match category {
        Category::None => None,
        Category::Misc => Some("misc".to_string()),
        Category::Doujinshi => Some("doujinshi".to_string()),
        Category::Manga => Some("manga".to_string()),
        Category::ArtistCG => Some("artistcg".to_string()),
        Category::GameCG => Some("gamecg".to_string()),
        Category::ImageSet => Some("imageset".to_string()),
        Category::Cosplay => Some("cosplay".to_string()),
        Category::NonH => Some("non-h".to_string()),
        Category::Western => Some("western".to_string()),
        Category::All => None,
        Category::Private => Some("private".to_string()),
        Category::Unknown => None,
    }
}

fn parse_category_str(category: String) -> Option<String> {
    match category.as_str() {
        "Misc" => Some("misc".to_string()),
        "Doujinshi" => Some("doujinshi".to_string()),
        "Manga" => Some("manga".to_string()),
        "Artist CG" => Some("artistcg".to_string()),
        "Game CG" => Some("gamecg".to_string()),
        "Image Set" => Some("imageset".to_string()),
        "Cosplay" => Some("cosplay".to_string()),
        "Non-H" => Some("non-h".to_string()),
        "Western" => Some("western".to_string()),
        "private" => Some("private".to_string()),
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
