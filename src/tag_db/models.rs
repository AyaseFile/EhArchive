use diesel::prelude::*;

use super::schema::*;

#[derive(Queryable, Insertable, Debug, Clone)]
#[diesel(table_name = metadata)]
pub struct Metadata {
    pub key: String,
    pub value: String,
}
