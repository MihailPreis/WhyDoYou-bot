//! Telegram bot implementation

use std::io::Cursor;

use lazy_static::lazy_static;
use log::{error, info};
use regex::Captures;
use regex::Regex;
use teloxide::net::Download;
use teloxide::prelude::*;
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
    let bot = Bot::from_env();
    teloxide::repl(bot, handler).await;
}

async fn handler<'a>(bot: Bot, message: Message) -> ResponseResult<()> {
    return match handle_message(&bot, &message).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{:?}", e);
            Ok(())
        }
    };
}

async fn download_file(bot: &Bot, file_id: String) -> Option<Vec<u8>> {
    if let Ok(file) = bot.get_file(file_id).await {
        let mut out: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut out);
        if let Ok(_) = bot.download_file(&file.path, &mut cursor).await {
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
fn get_text(message: &Message) -> Option<&str> {
    match &message.kind {
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
    bot: &Bot,
    message: &Message,
) -> Result<(), HandlerError> {
    let user = match message.from() {
        None => return Ok(()),
        Some(user) => user,
    };
    let photos: UserProfilePhotos = bot.get_user_profile_photos(user.id).await?;
    let data = match get_text(message) {
        Some(data) if !data.is_empty() => data,
        _ => return Ok(()),
    };

    if message.chat.is_private() {
        let help_text = TEXTS.get_tg("private_help_text", &message);
        match message.text() {
            Some(HELP_CMD) => {
                bot.send_message(message.chat.id, help_text).reply_to_message_id(message.id).await?;
            }
            Some(START_CMD) => {
                bot.send_message(message.chat.id, help_text).reply_to_message_id(message.id).await?;
            }
            Some(VERSION_CMD) => {
                bot.send_message(message.chat.id, VERSION_STRING).reply_to_message_id(message.id).await?;
            }
            _ => {}
        };
    }

    if !message.chat.is_group() && !message.chat.is_supergroup() {
        return Ok(());
    }

    info!("Bot received a new message: {}", data);

    if data.starts_with("/") {
        if let Ok(group_admins) = bot.get_chat_administrators(message.chat.id).await {
            if group_admins
                .iter()
                .map(|u| u.user.id)
                .collect::<Vec<UserId>>()
                .contains(&user.id)
            {
                info!(
                    "@{} is admin.",
                    user.username
                        .as_ref()
                        .unwrap_or(&String::from(UNKNOWN_USER))
                );
                return if data == VERSION_CMD {
                    bot.send_message(message.chat.id, VERSION_STRING).reply_to_message_id(message.id).await?;
                    Ok(())
                } else {
                    exec_command(bot, message).await
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
                if let Some(custom_words) = get_custom_words_from_db(message.chat.id.clone().0).await {
                    let _words = contains_in(custom_words, String::from(data));
                    if !_words.is_empty() {
                        if let Ok(content) = db_conn
                            .get_random_content(message.chat.id.0, true, _words)
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
                .and_then(|i| Some(i.file.id.clone()))
            {
                info!("Use user avatar.");
                return download_file(&bot, photo_id).await;
            }
            info!("Use default image.");
            return None;
        };
        let audio_handler = async move {
            if let Ok(db_conn) = DBConn::new().await {
                if let Some(custom_words) = get_custom_words_from_db(message.chat.id.clone().0).await {
                    let _words = contains_in(custom_words, String::from(data));
                    if !_words.is_empty() {
                        if let Ok(content) = db_conn
                            .get_random_content(message.chat.id.0, false, _words)
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
            get_custom_words_from_db(message.chat.id.clone().0).await,
            image_handler,
            audio_handler,
        )
        .await
        {
            Ok(v_data) => match v_data {
                Video(video) => {
                    bot.send_video(
                        message.chat.id,
                        InputFile::memory(
                            video,
                        ),
                    )
                        .reply_to_message_id(message.id)
                        .await?;
                    Ok(())
                }
                Image(image) => {
                    bot.send_photo(
                        message.chat.id,
                        InputFile::memory(
                            image,
                        ),
                    )
                        .reply_to_message_id(message.id)
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
async fn exec_command(bot: &Bot, message: &Message) -> Result<(), HandlerError> {
    let data = get_text(message).unwrap_or("");
    let match_cmd = CMD_REGEX
        .captures(&*data)
        .ok_or(HandlerError::new(String::from("Invalid command")))?;
    let cmd = match_cmd
        .get(1)
        .map(|data| data.as_str())
        .or_else(|| Some(""))
        .unwrap();

    async fn get_help(bot: &Bot, msg: &Message) -> Result<(), HandlerError> {
        bot.send_message(msg.chat.id, TEXTS.get_tg("group_help_with_db", msg))
            .reply_to_message_id(msg.id)
            .await?;
        Ok(())
    }

    async fn get_words(bot: &Bot, msg: &Message) -> Result<(), HandlerError> {
        let resp = DBConn::new().await?.get_words(msg.chat.id.0).await?;
        if resp.is_empty() {
            bot.send_message(msg.chat.id, TEXTS.get_tg("empty_list_message", msg))
                .reply_to_message_id(msg.id)
                .await?;
        } else {
            bot.send_message(msg.chat.id, resp)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Ok(())
    }

    async fn get_all_contents(
        bot: &Bot, msg: &Message,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        let items = DBConn::new()
            .await?
            .get_all_contents(msg.chat.id.0, is_image)
            .await?;
        if items.is_empty() {
            bot.send_message(msg.chat.id, TEXTS.get_tg("empty_list_message", msg))
                .reply_to_message_id(msg.id)
                .await?;
        } else {
            let resp = items
                .iter()
                .map(|i| format!("{} - {}", i.name, i.words))
                .collect::<Vec<String>>()
                .join("\n");
            bot.send_message(msg.chat.id, resp)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Ok(())
    }

    async fn rm_content(
        match_cmd: Captures<'_>,
        bot: &Bot, msg: &Message,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            bot.send_message(msg.chat.id, TEXTS.get_tg("invalid_arguments", msg))
                .reply_to_message_id(msg.id)
                .await?;
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
            .rm_content(msg.chat.id.0, is_image, String::from(args))
            .await
        {
            Ok(_) => {
                bot.send_message(msg.chat.id, TEXTS.get_tg("rm_content_success", msg))
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            Err(_) => {
                bot.send_message(msg.chat.id, TEXTS.get_tg("rm_content_error", msg))
                    .reply_to_message_id(msg.id)
                    .await?;
            }
        }
        Ok(())
    }

    async fn add_audio(
        bot: &Bot, msg: &Message,
        words: String,
    ) -> Result<(), HandlerError> {
        if let MessageKind::Common(item) = &msg.kind {
            if let MediaKind::Audio(audio) = &item.media_kind {
                if let Some(file_name) = &audio.audio.file_name {
                    if let Some(data) =
                        download_file(bot, audio.audio.file.id.clone()).await
                    {
                        DBConn::new()
                            .await?
                            .add_content(ContentModel::from(
                                msg.chat.id.0,
                                false,
                                words,
                                file_name.clone(),
                                data,
                            ))
                            .await?;
                        bot.send_message(msg.chat.id, TEXTS.get_tg("audio_add_success", msg))
                            .reply_to_message_id(msg.id)
                            .await?;
                        return Ok(());
                    }
                    bot.send_message(msg.chat.id, TEXTS.get_tg("audio_add_dw_error", msg))
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Err(HandlerError::from_str("Invalid file load"));
                }
            }
        }
        bot.send_message(msg.chat.id, TEXTS.get_tg("audio_add_format_error", msg))
            .reply_to_message_id(msg.id)
            .await?;
        return Err(HandlerError::from_str("Invalid document"));
    }

    async fn add_image(
        bot: &Bot, msg: &Message,
        words: String,
    ) -> Result<(), HandlerError> {
        if let MessageKind::Common(item) = &msg.kind {
            if let MediaKind::Document(doc) = &item.media_kind {
                if doc.document.mime_type == Some(mime::IMAGE_JPEG) {
                    if let Some(file_name) = &doc.document.file_name {
                        if let Some(data) =
                            download_file(bot, doc.document.file.id.clone()).await
                        {
                            DBConn::new()
                                .await?
                                .add_content(ContentModel::from(
                                    msg.chat.id.0,
                                    true,
                                    words,
                                    file_name.clone(),
                                    data,
                                ))
                                .await?;
                            bot.send_message(msg.chat.id, TEXTS.get_tg("image_add_success", msg))
                                .reply_to_message_id(msg.id)
                                .await?;
                            return Ok(());
                        }
                        bot.send_message(msg.chat.id, TEXTS.get_tg("image_add_dw_error", msg))
                            .reply_to_message_id(msg.id)
                            .await?;
                        return Err(HandlerError::from_str("Invalid file load"));
                    }
                }
                bot.send_message(msg.chat.id, TEXTS.get_tg("image_add_format_invalid", msg))
                    .reply_to_message_id(msg.id)
                    .await?;
                return Err(HandlerError::from_str("Invalid image format"));
            }
        }
        bot.send_message(msg.chat.id, TEXTS.get_tg("image_add_format_error", msg))
            .reply_to_message_id(msg.id)
            .await?;
        return Err(HandlerError::from_str("Invalid document"));
    }

    async fn add_content(
        match_cmd: Captures<'_>,
        bot: &Bot, msg: &Message,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            bot.send_message(msg.chat.id, TEXTS.get_tg("invalid_arguments", msg))
                .reply_to_message_id(msg.id)
                .await?;
            // TODO: wtf is this? why it is error?
            return Err(HandlerError::from_str("Args invalid"));
        }
        let args = match_cmd
            .get(3)
            .map(|data| data.as_str())
            .or_else(|| Some(""))
            .unwrap()
            .trim();
        if !WORDS_REGEX.is_match(args) {
            bot.send_message(msg.chat.id, TEXTS.get_tg("keyword_error", msg))
                .reply_to_message_id(msg.id)
                .await?;
            return Err(HandlerError::from_str("Invalid keywoeds"));
        }
        match is_image {
            true => add_image(bot, msg,String::from(args)).await,
            false => add_audio(bot, msg, String::from(args)).await,
        }
    }

    async fn change_words(
        match_cmd: Captures<'_>,
        bot: &Bot, msg: &Message,
        is_image: bool,
    ) -> Result<(), HandlerError> {
        if match_cmd.len() <= 2 {
            bot.send_message(msg.chat.id, TEXTS.get_tg("invalid_arguments", msg))
                .reply_to_message_id(msg.id)
                .await?;
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
                    msg.chat.id.0,
                    is_image,
                    String::from(file_name),
                    String::from(new_words),
                )
                .await?;
            bot.send_message(msg.chat.id, TEXTS.get_tg("done_msg", msg))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(());
        }
        bot.send_message(msg.chat.id, TEXTS.get_tg("error_msg", msg))
            .reply_to_message_id(msg.id)
            .await?;
        Err(HandlerError::from_str("Args invalid"))
    }

    match cmd {
        HELP => get_help(bot, message).await?,
        LIST_IMAGE => get_all_contents(bot, message, true).await?,
        LIST_AUDIO => get_all_contents(bot, message, false).await?,
        LIST_WORDS => get_words(bot, message).await?,
        ADD_IMAGE => add_content(match_cmd, bot, message, true).await?,
        ADD_AUDIO => add_content(match_cmd, bot, message, false).await?,
        RM_IMAGE => rm_content(match_cmd, bot, message, true).await?,
        RM_AUDIO => rm_content(match_cmd, bot, message, false).await?,
        EDIT_IMAGE => change_words(match_cmd, bot, message, true).await?,
        EDIT_AUDIO => change_words(match_cmd, bot, message, false).await?,
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
    fn get_tg(&self, key: &str, msg: &Message) -> String {
        let lang = &msg
            .from()
            .and_then(|u| u.language_code.clone())
            .unwrap_or(String::from("en"));
        self.get(&*format!("tg_{}", key), lang.as_str())
    }
}
