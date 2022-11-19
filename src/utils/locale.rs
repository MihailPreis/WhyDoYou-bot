//! Localization helper

use crate::models::error::HandlerError;
use include_dir::{include_dir, Dir, File};
use lazy_static::lazy_static;
use log::{error, info, warn};
use regex::{Captures, Regex};
use std::collections::HashMap;

lazy_static! {
    /// Shared instance of Locale
    pub static ref TEXTS: Locale = Locale::parse().unwrap();
    static ref ROW_REGEX: Regex = regex::Regex::new("\"([\\w]+)\" = \"([^\"]+)\";").unwrap();
}

const LOCALE_DIR: Dir = include_dir!("assets/locale");

#[derive(Debug)]
pub struct Locale {
    locales: Vec<LocaleFileMeta>,
}

impl Locale {
    fn parse() -> Result<Self, HandlerError> {
        let mut locales: Vec<LocaleFileMeta> = Vec::new();
        for file in LOCALE_DIR.files() {
            if let Some(meta) = LocaleFileMeta::from(file) {
                locales.push(meta);
            }
        }
        let item = Self { locales };
        item._test_keys()?;
        Ok(item)
    }

    /// Get localized string
    ///
    /// Parameters:
    ///  - key: localization key
    ///  - lang: language code (ex.: en, ru)
    ///
    /// Return: localized string or key
    pub fn get(&self, key: &str, lang: &str) -> String {
        let result = &self
            .locales
            .iter()
            .find(|l| l.lang.to_lowercase() == lang.to_lowercase())
            .or_else(|| *{ &self.locales.iter().find(|l| l.is_base) })
            .and_then(|l| l.data.get(key).to_owned().and_then(|s| Some(s.as_str())))
            .unwrap_or(key);
        return result.to_string();
    }

    fn _test_keys(&self) -> Result<(), HandlerError> {
        let mut is_error = false;
        for locale in &self.locales {
            self.locales.iter().for_each(|l| {
                if l.lang == locale.lang {
                    return;
                };
                locale.data.iter().for_each(|a| {
                    if !l.data.contains_key(a.0) {
                        if l.is_base {
                            is_error = true;
                            error!(
                                "{} lang not contain '{}' key which is in {} lang",
                                l.title(),
                                a.0,
                                locale.title()
                            );
                        } else {
                            warn!(
                                "{} lang not contain '{}' key which is in {} lang",
                                l.title(),
                                a.0,
                                locale.title()
                            );
                        }
                    }
                })
            });
        }
        if is_error {
            Err(HandlerError::from_str("Locales has errors."))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
struct LocaleFileMeta {
    lang: String,
    is_base: bool,
    data: HashMap<String, String>,
}

impl LocaleFileMeta {
    fn from(file: &File) -> Option<Self> {
        if let Some(os_str_name) = file.path().file_name() {
            if let Some(raw_str) = os_str_name.to_str() {
                let components = raw_str.split(".").collect::<Vec<&str>>();
                if components.len() > 3 || components.len() < 2 {
                    return None;
                }
                if components.last().unwrap().to_lowercase() != String::from("locale") {
                    return None;
                }
                if let Some(content) = file.contents_utf8() {
                    let f_content = content
                        .split("\n")
                        .filter(|row| !row.starts_with("//") || !row.is_empty())
                        .collect::<Vec<&str>>()
                        .join("\n");
                    let mut data: HashMap<String, String> = HashMap::new();
                    for row in ROW_REGEX
                        .captures_iter(&*f_content)
                        .collect::<Vec<Captures>>()
                    {
                        let key = row.get(1).unwrap().as_str().parse().unwrap();
                        let value = row.get(2).unwrap().as_str().parse().unwrap();
                        data.insert(key, value);
                    }
                    let item = LocaleFileMeta {
                        lang: components.get(0).unwrap().parse().unwrap(),
                        is_base: components.len() == 3
                            && components.get(1).unwrap().to_lowercase() == String::from("base"),
                        data,
                    };
                    info!("{} language found & loaded.", item.title());
                    return Some(item);
                }
            }
        }
        None
    }

    fn title(&self) -> String {
        let mut result = String::from(&self.lang.clone());
        if self.is_base {
            result.push_str("*");
        }
        result.to_string().to_uppercase()
    }
}
