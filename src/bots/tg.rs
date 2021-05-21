//! Telegram bot implementation

use std::borrow::Cow;
use std::io::Cursor;

use lazy_static::lazy_static;
use log::{error, info};
use regex::Captures;
use regex::Regex;
use teloxide::dispatching::UpdateWithCx;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::prelude::{AutoSend, Message, RequesterExt};
use teloxide::requests::Requester;
use teloxide::types::{
    InputFile, MediaAudio, MediaDocument, MediaKind, MediaText, MessageCommon, MessageKind,
    UserProfilePhotos,
};
use teloxide::Bot;

use crate::engine::engine::build_message;
use crate::models::content_model::ContentModel;
use crate::models::db_conn::DBConn;
use crate::models::error::HandlerError;
use crate::models::v_data::VData::{Image, Video};
use crate::utils::locale::{Locale, TEXTS};
use crate::utils::string_utils::contains_in;
use crate::utils::version::VERSION_STRING;
use teloxide::types::MessageKind::Common;

const UNKNOWN_USER: &str = "unknown";

const HELP_CMD: &str = "/help";
const START_CMD: &str = "/start";
const VERSION_CMD: &str = "/version";

const HELP: &str = "help";
const LIST_IMAGE: &str = "listimage";
const LIST_AUDIO: &str = "listaudio";
const LIST_WORDS: &str = "listwords";
const ADD_IMAGE: &str = "addimage";
const ADD_AUDIO: &str = "addaudio";
const RM_IMAGE: &str = "rmimage";
const RM_AUDIO: &str = "rmaudio";
const EDIT_IMAGE: &str = "editimage";
const EDIT_AUDIO: &str = "editaudio";

lazy_static! {
    static ref CMD_REGEX: Regex = regex::Regex::new("/([a-zA-Z]+)( (.+))?").unwrap();
    static ref WORDS_REGEX: Regex = regex::Regex::new("[a-zA-Z0-9а-яА-Я,]+").unwrap();
    static ref CHANGE_WORDS_REGEX: Regex = regex::Regex::new("(.+) ([a-zA-Z0-9а-яА-Я,]+)").unwrap();
}

/// Run TG bot and await
pub async fn run_tg_bot() {
    let bot = Bot::from_env().auto_send();
    teloxide::repl(bot, handler).await;
}

async fn handler<'a>(message: UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), ()> {
    return match handle_message(&message).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{:?}", e.message);
            Ok(())
        }
    };
}

async fn download_file(bot: &AutoSend<Bot>, file_id: String) -> Option<Vec<u8>> {
    if let Ok(file) = bot.get_file(file_id).await {
        let mut out: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut out);
        if let Ok(_) = bot.download_file(&file.file_path, &mut cursor).await {
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

#[cfg(not(feature = "db"))]
fn get_text(message: &UpdateWithCx<AutoSend<Bot>, Message>) -> Option<&str> {
    message.update.text()
}

#[cfg(feature = "db")]
fn get_text(message: &UpdateWithCx<AutoSend<Bot>, Message>) -> Option<&str> {
    match &message.update.kind {
        Common(MessageCommon {
            media_kind: MediaKind::Text(MediaText { text, .. }),
            ..
        }) => Some(text),
        Common(MessageCommon {
            media_kind: MediaKind::Document(MediaDocument { caption, .. }),
            ..
        }) => caption.as_deref(),
        Common(MessageCommon {
            media_kind: MediaKind::Audio(MediaAudio { caption, .. }),
            ..
        }) => caption.as_deref(),
        _ => None,
    }
}

async fn get_custom_words_from_db(chat_id: i64) -> Option<String> {
    if let Ok(db_conn) = DBConn::new().await {
        if let Ok(words) = db_conn.get_words(chat_id).await {
            if words.is_empty() {
                None
            } else {
                Some(words)
            }
        } else {
            None
        }
    } else {
        None
    }
}

async fn handle_message<'a>(
    message: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<(), HandlerError> {
    let user = match message.update.from() {
        None => return Ok(()),
        Some(user) => user,
    };
    let bot: &AutoSend<Bot> = &message.requester;
    let photos: UserProfilePhotos = bot.get_user_profile_photos(user.id).await?;
    let data = match get_text(message) {
        Some(data) if !data.is_empty() => data,
        _ => return Ok(()),
    };

    if message.update.chat.is_private() {
        let help_text = TEXTS.get_tg("private_help_text", &message);
        match message.update.text() {
            Some(HELP_CMD) => {
                message.answer(help_text).await?;
            }
            Some(START_CMD) => {
                message.answer(help_text).await?;
            }
            Some(VERSION_CMD) => {
                message.answer(VERSION_STRING).await?;
            }
            _ => {}
        };
    }

    if !message.update.chat.is_group() && !message.update.chat.is_supergroup() {
        return Ok(());
    }

    info!("Bot received a new message: {}", data);

    if data.starts_with("/") {
        if let Ok(group_admins) = bot.get_chat_administrators(message.update.chat.id).await {
            if group_admins
                .iter()
                .map(|u| u.user.id)
                .collect::<Vec<i64>>()
                .contains(&user.id)
            {
                info!(
                    "@{} is admin.",
                    user.username
                        .as_ref()
                        .unwrap_or(&String::from(UNKNOWN_USER))
                );
                return if data == VERSION_CMD {
                    message.reply_to(VERSION_STRING).await?;
                    Ok(())
                } else {
                    exec_command(message).await
                };
            }
            info!(
                "@{} is not a admin.",
                user.username
                    .as_ref()
                    .unwrap_or(&String::from(UNKNOWN_USER))
            );
        }
    } else {
        let image_handler = async move {
            if let Ok(db_conn) = DBConn::new().await {
                if let Some(custom_words) = get_custom_words_from_db(message.update.chat.id).await {
                    let _words = contains_in(custom_words, String::from(data));
                    if !_words.is_empty() {
                        if let Ok(content) = db_conn
                            .get_random_content(message.update.chat.id, true, _words)
                            .await
                        {
                            info!("Use random image from DB.");
                            return Some(content.data);
                        }
                    }
                }
            } else if let Some(photo_id) = photos
                .photos
                .first()
                .and_then(|s| s.last())
                .and_then(|i| Some(i.file_id.clone()))
            {
                info!("Use user avatar.");
                return download_file(bot, photo_id).await;
            }
            info!("Use default image.");
            return None;
        };
        let audio_handler = async move {
            if let Ok(db_conn) = DBConn::new().await {
                if let Some(custom_words) = get_custom_words_from_db(message.update.chat.id).await {
                    let _words = contains_in(custom_words, String::from(data));
                    if !_words.is_empty() {
                        if let Ok(content) = db_conn
                            .get_random_content(message.update.chat.id, false, _words)
                            .await
                        {
                            return Some(content.data);
                        }
                    }
                }
            }
            None
        };
        return match build_message(
            &*data,
            get_custom_words_from_db(message.update.chat.id).await,
            image_handler,
            audio_handler,
        )
        .await
        {
            Ok(v_data) => match v_data {
                Video(video) => {
                    message
                        .answer_video(InputFile::Memory {
                            file_name: "data.mp4".to_string(),
                            data: Cow::from(video),
                        })
                        .reply_to_message_id(message.update.id)
                        .await?;
                    Ok(())
                }
                Image(image) => {
                    message
                        .answer_photo(InputFile::Memory {
                            file_name: "image.jpg".to_string(),
                            data: Cow::from(image),
                        })
                        .reply_to_message_id(message.update.id)
                        .await?;
                    Ok(())
                }
            },
            Err(err) => {
                if err.message.is_none() {
                    Ok(())
                } else {
                    Err(err)
                }
            }
        };
    }

    Ok(())
}

#[cfg(not(feature = "db"))]
async fn exec_command(msg: &UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), HandlerError> {
    match msg.update.text() {
        Some("/help") => {
            msg.reply_to(TEXTS.get_tg("group_help_without_db", msg))
                .await?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "db")]
async fn exec_command(msg: &UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), HandlerError> {
    let data = get_text(msg).unwrap_or("");
    let match_cmd = CMD_REGEX
        .captures(&*data)
        .ok_or(HandlerError::new(String::from("Invalid command")))?;
    let cmd = match_cmd
        .get(1)
        .map(|data| data.as_str())
        .or_else(|| Some(""))
        .unwrap();

    async fn get_help(msg: &UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), HandlerError> {
        msg.reply_to(TEXTS.get_tg("group_help_with_db", msg))
            .await?;
        Ok(())
    }

    async fn get_words(msg: &UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), HandlerError> {
        let resp = DBConn::new().await?.get_words(msg.update.chat.id).await?;
        if resp.is_empty() {
            msg.reply_to(TEXTS.get_tg("empty_list_message", msg))
                .await?;
        } else {
            msg.reply_to(resp).await?;
        }
        Ok(())
    }

    async fn get_all_contents(
        msg: &UpdateWithCx<AutoSend<Bot>, Message>,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        let items = DBConn::new()
            .await?
            .get_all_contents(msg.update.chat.id, is_image)
            .await?;
        if items.is_empty() {
            msg.reply_to(TEXTS.get_tg("empty_list_message", msg))
                .await?;
        } else {
            let resp = items
                .iter()
                .map(|i| format!("{} - {}", i.name, i.words))
                .collect::<Vec<String>>()
                .join("\n");
            msg.reply_to(resp).await?;
        }
        Ok(())
    }

    async fn rm_content(
        match_cmd: Captures<'_>,
        msg: &UpdateWithCx<AutoSend<Bot>, Message>,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            msg.reply_to(TEXTS.get_tg("invalid_arguments", msg)).await?;
            return Err(HandlerError::from_str("Args invalid"));
        }
        let args = match_cmd
            .get(3)
            .map(|data| data.as_str())
            .or_else(|| Some(""))
            .unwrap()
            .trim();
        match DBConn::new()
            .await?
            .rm_content(msg.update.chat.id, is_image, String::from(args))
            .await
        {
            Ok(_) => {
                msg.reply_to(TEXTS.get_tg("rm_content_success", msg))
                    .await?;
            }
            Err(_) => {
                msg.reply_to(TEXTS.get_tg("rm_content_error", msg)).await?;
            }
        }
        Ok(())
    }

    async fn add_audio(
        msg: &&UpdateWithCx<AutoSend<Bot>, Message>,
        words: String,
    ) -> Result<(), HandlerError> {
        if let MessageKind::Common(item) = &msg.update.kind {
            if let MediaKind::Audio(audio) = &item.media_kind {
                if let Some(file_name) = &audio.audio.file_name {
                    if let Some(data) =
                        download_file(&msg.requester, audio.audio.file_id.clone()).await
                    {
                        DBConn::new()
                            .await?
                            .add_content(ContentModel::from(
                                msg.update.chat.id,
                                false,
                                words,
                                file_name.clone(),
                                data,
                            ))
                            .await?;
                        msg.reply_to(TEXTS.get_tg("audio_add_success", msg)).await?;
                        return Ok(());
                    }
                    msg.reply_to(TEXTS.get_tg("audio_add_dw_error", msg))
                        .await?;
                    return Err(HandlerError::from_str("Invalid file load"));
                }
            }
        }
        msg.reply_to(TEXTS.get_tg("audio_add_format_error", msg))
            .await?;
        return Err(HandlerError::from_str("Invalid document"));
    }

    async fn add_image(
        msg: &&UpdateWithCx<AutoSend<Bot>, Message>,
        words: String,
    ) -> Result<(), HandlerError> {
        if let MessageKind::Common(item) = &msg.update.kind {
            if let MediaKind::Document(doc) = &item.media_kind {
                if doc.document.mime_type == Some(mime::IMAGE_JPEG) {
                    if let Some(file_name) = &doc.document.file_name {
                        if let Some(data) =
                            download_file(&msg.requester, doc.document.file_id.clone()).await
                        {
                            DBConn::new()
                                .await?
                                .add_content(ContentModel::from(
                                    msg.update.chat.id,
                                    true,
                                    words,
                                    file_name.clone(),
                                    data,
                                ))
                                .await?;
                            msg.reply_to(TEXTS.get_tg("image_add_success", msg)).await?;
                            return Ok(());
                        }
                        msg.reply_to(TEXTS.get_tg("image_add_dw_error", msg))
                            .await?;
                        return Err(HandlerError::from_str("Invalid file load"));
                    }
                }
                msg.reply_to(TEXTS.get_tg("image_add_format_invalid", msg))
                    .await?;
                return Err(HandlerError::from_str("Invalid image format"));
            }
        }
        msg.reply_to(TEXTS.get_tg("image_add_format_error", msg))
            .await?;
        return Err(HandlerError::from_str("Invalid document"));
    }

    async fn add_content(
        match_cmd: Captures<'_>,
        msg: &&UpdateWithCx<AutoSend<Bot>, Message>,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            msg.reply_to(TEXTS.get_tg("invalid_arguments", msg)).await?;
            return Err(HandlerError::from_str("Args invalid"));
        }
        let args = match_cmd
            .get(3)
            .map(|data| data.as_str())
            .or_else(|| Some(""))
            .unwrap()
            .trim();
        if !WORDS_REGEX.is_match(args) {
            msg.reply_to(TEXTS.get_tg("keyword_error", msg)).await?;
            return Err(HandlerError::from_str("Invalid keywoeds"));
        }
        match is_image {
            true => add_image(msg, String::from(args)).await,
            false => add_audio(msg, String::from(args)).await,
        }
    }

    async fn change_words(
        match_cmd: Captures<'_>,
        msg: &UpdateWithCx<AutoSend<Bot>, Message>,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            msg.reply_to(TEXTS.get_tg("invalid_arguments", msg)).await?;
            return Err(HandlerError::from_str("Args invalid"));
        }
        let args = match_cmd
            .get(3)
            .map(|data| data.as_str())
            .or_else(|| Some(""))
            .unwrap()
            .trim();
        if let Some(match_args) = CHANGE_WORDS_REGEX.captures(args) {
            let file_name = match_args
                .get(1)
                .map(|data| data.as_str())
                .or_else(|| Some(""))
                .unwrap();
            let new_words = match_args
                .get(2)
                .map(|data| data.as_str())
                .or_else(|| Some(""))
                .unwrap();
            DBConn::new()
                .await?
                .change_words(
                    msg.update.chat.id,
                    is_image,
                    String::from(file_name),
                    String::from(new_words),
                )
                .await?;
            msg.reply_to(TEXTS.get_tg("done_msg", msg)).await?;
            return Ok(());
        }
        msg.reply_to(TEXTS.get_tg("error_msg", msg)).await?;
        Err(HandlerError::from_str("Args invalid"))
    }

    match cmd {
        HELP => get_help(msg).await?,
        LIST_IMAGE => get_all_contents(msg, true).await?,
        LIST_AUDIO => get_all_contents(msg, false).await?,
        LIST_WORDS => get_words(msg).await?,
        ADD_IMAGE => add_content(match_cmd, &msg, true).await?,
        ADD_AUDIO => add_content(match_cmd, &msg, false).await?,
        RM_IMAGE => rm_content(match_cmd, &msg, true).await?,
        RM_AUDIO => rm_content(match_cmd, &msg, false).await?,
        EDIT_IMAGE => change_words(match_cmd, msg, true).await?,
        EDIT_AUDIO => change_words(match_cmd, msg, false).await?,
        &_ => return Err(HandlerError::from_str("Command not found")),
    };
    Ok(())
}

impl Locale {
    /// Get localized string with unwrap lang code from teloxide message
    ///
    /// Parameters:
    ///  - key: localization key
    ///  - msg: message instance from teloxide message handler
    ///
    /// Return: localized string or key
    fn get_tg(&self, key: &str, msg: &UpdateWithCx<AutoSend<Bot>, Message>) -> String {
        let lang = &msg
            .update
            .from()
            .and_then(|u| u.language_code.clone())
            .unwrap_or(String::from("en"));
        self.get(&*format!("tg_{}", key), lang.as_str())
    }
}
