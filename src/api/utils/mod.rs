pub(super) mod archive;
pub(super) mod comic_info;

use libeh::dto::keyword::Keyword;

fn parse_category(category: String) -> Option<String> {
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
        Keyword::Temp(_) | Keyword::Uploader(_) => None,
        Keyword::Location(k) => Some(("location", k)),
    }
}
