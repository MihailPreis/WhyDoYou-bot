use crate::utils::string_utils::normalize_words;

#[derive(Debug, Clone, PartialEq)]
pub struct ContentModel {
    pub id: i64,
    pub chat_id: i64,
    pub is_image: bool,
    pub words: String,
    pub name: String,
    pub data: Vec<u8>,
}

impl ContentModel {
    pub fn from(chat_id: i64, is_image: bool, words: String, name: String, data: Vec<u8>) -> Self {
        Self {
            id: 0,
            chat_id,
            is_image,
            words: normalize_words(words),
            name: name.replace(" ", "_").trim().to_lowercase(),
            data,
        }
    }
}
