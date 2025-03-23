macro_rules! define_tag_table {
    ($table_name:ident) => {
        diesel::table! {
            $table_name (id) {
                id -> Integer,
                raw -> Text,
                name -> Text,
                intro -> Text,
                links -> Text,
            }
        }
    };
    ($($table_name:ident),+) => {
        $(
            define_tag_table!($table_name);
        )+
    };
}

define_tag_table!(
    artist, character, cosplayer, female, groups, language, male, mixed, other, parody, reclass,
    rows
);

diesel::table! {
    metadata (key) {
        key -> Text,
        value -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    artist, character, cosplayer, female, groups, language, male, mixed, other, parody, reclass,
    rows, metadata,
);
