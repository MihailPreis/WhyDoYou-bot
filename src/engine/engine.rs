//! Engine
//!
//! Checking text for finding trigger words and creating a meme quote from a custom (or random) image and sound.

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::io::{BufWriter, Cursor};
use std::str;
use std::time::Duration;

use image::{ColorType, EncodableLayout, GenericImageView, ImageFormat, Rgb, RgbImage};
use imageproc::definitions::HasWhite;
use imageproc::drawing::{draw_text, Canvas};
use lazy_static::lazy_static;
use log::{debug, error, info};
use regex::Regex;
use reqwest::{multipart, Client};
use rusttype::{Font, Scale};

use crate::engine::default_images::get_rand_image;
use crate::engine::engine::VData::{Image, Video};
use crate::engine::local_ffmpeg::{check_ffmpeg_exist, encode_video_local};
use crate::models::error::HandlerError;
use crate::models::text_size_box::TextSizeBox;
use crate::models::v_data::VData;
use crate::utils::size_utils::aspect_resize;
use crate::utils::string_utils::{batch, contains_in};
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use std::future::Future;

const WORDS_KEY: &str = "WORDS";
const CONVERTER_URL_KEY: &str = "CONVERTER_URL";
const IMAGE_SIZE: u32 = 1024;
const PHOTO_W: u32 = 768;
const PHOTO_H: u32 = 512;
const PADDING: u32 = 128;
const STRING_BATCH_SIZE: usize = 30;
const MAX_ROWS: usize = 6;
const FONT_H: u32 = 50;

const FONT_BYTES: &[u8] = include_bytes!("../../assets/font.ttf");

lazy_static! {
    static ref REGEX_VALUE: Regex = regex::Regex::new("/gen (.*)").unwrap();
    static ref CONVERTER_URL: String = std::env::var(CONVERTER_URL_KEY)
        .unwrap_or("https://why-do-you-converter.herokuapp.com/process".to_string())
        .to_string();
    static ref TRIGGER_WORDS: Vec<String> = std::env::var(WORDS_KEY)
        .unwrap_or(String::from("fuck,dick,cum,cock,https://www.youtube.com/watch?v=ak16XxnJK0g,https://youtu.be/ak16XxnJK0g"))
        .split(",")
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    static ref CLIENT: Client = reqwest::Client::new();
    static ref FONT_SIZE: Scale = Scale::uniform(64.0);
}

/// Create meme-quote if needs with optional image and audio
///
/// Parameters:
///  - res:             text of message
///  - custom_words:    optional trigger words
///  - image_handler:   async closure that returns an optional binary image
///  - audio_handler:   async closure that returns an optional binary audio
///
/// Return: Result with VData or HandlerError
pub async fn build_message(
    res: &str,
    custom_words: Option<String>,
    image_handler: impl Future<Output = Option<Vec<u8>>>,
    audio_handler: impl Future<Output = Option<Vec<u8>>>,
) -> Result<VData, HandlerError> {
    let message: &str;
    if let Some(words) = custom_words {
        info!("Found trigger words in DB: {:?}", words);
        let words = contains_in(words, String::from(res));
        if words.is_empty() {
            return Err(HandlerError::empty());
        }
        info!("Trigger words: {:?}", words);
        message = res;
    } else if !TRIGGER_WORDS.is_empty() {
        info!("Found trigger words: {:?}", *TRIGGER_WORDS);
        if !*&(*TRIGGER_WORDS)
            .iter()
            .fold(false, |acc, item| acc || res.contains(item))
        {
            return Err(HandlerError::empty());
        }
        message = res;
    } else {
        info!("Trigger words not found. Use '/gen <message>' checker.");
        let match_res = REGEX_VALUE
            .captures(res)
            .ok_or(HandlerError::new(String::from("Invalid command")))?
            .get(1)
            .map(|data| data.as_str())
            .unwrap_or("");
        message = match_res;
    }
    let mut input_image: Vec<u8> = get_rand_image();
    if let Some(user_image) = image_handler.await {
        input_image = user_image;
    }
    let image = create_image(message, input_image.clone()).await?;
    let custom_audio = audio_handler.await;
    match encode_video(image.clone(), custom_audio).await {
        Ok(video) => Ok(Video(video)),
        Err(e) => {
            error!("error encoding video {:?}", e);
            Ok(Image(image))
        },
    }
}

async fn create_image(message: &str, input: Vec<u8>) -> Result<Vec<u8>, HandlerError> {
    let font = match Font::try_from_vec(Vec::from(FONT_BYTES)) {
        None => {
            return Err(HandlerError::new(String::from("Can not instantiate font")));
        }
        Some(data) => data,
    };
    let reader = Cursor::new(input);
    let start_image = image::load(reader, ImageFormat::Jpeg)?
        .as_rgb8()
        .unwrap()
        .clone();
    let mut out: Vec<u8> = Vec::new();
    let (_, _, start_image_w, start_image_h) = start_image.bounds();
    let (new_w, new_h) = aspect_resize(start_image_w, start_image_h, PHOTO_W, PHOTO_H);
    let res = image::imageops::resize(&start_image, new_w, new_h, FilterType::Gaussian);
    let cursor = BufWriter::new(&mut out);

    let mut subs: Vec<String> = Vec::new();
    if message.contains('\n') {
        let list: Vec<&str> = message.split("\n").collect();
        for item in list {
            subs.append(&mut batch(item, STRING_BATCH_SIZE))
        }
    } else {
        subs = batch(message, STRING_BATCH_SIZE);
    }

    let mut y = PHOTO_H + PADDING;
    let rect_list: Vec<TextSizeBox> = subs
        .iter()
        .map(|msg| TextSizeBox::from(msg.as_str(), font.borrow(), *FONT_SIZE))
        .collect();

    let (_subs, _) = if subs.len() < MAX_ROWS {
        (subs.as_slice(), subs.as_slice())
    } else {
        subs.split_at(MAX_ROWS)
    };

    if _subs.len() < MAX_ROWS {
        let h: u32 = rect_list.iter().fold(0, |sum, val| {
            return if sum > 0 {
                sum + val.h + 10
            } else {
                sum + val.h
            };
        });
        y += ((IMAGE_SIZE - y) - h) / 2 + 10;
    }

    let mut image: RgbImage = RgbImage::new(IMAGE_SIZE, IMAGE_SIZE);
    for (ind, msg) in _subs.iter().enumerate() {
        let rect = rect_list.get(ind).unwrap();
        image = draw_text(
            &mut image,
            Rgb::white(),
            i32::try_from((IMAGE_SIZE - rect.w) / 2)?,
            i32::try_from(y)?,
            *FONT_SIZE,
            &font,
            msg.as_str(),
        );
        y += FONT_H + 10;
    }

    let (_, _, img_x_stride, img_y_stride) = res.bounds();
    let x_offset = (IMAGE_SIZE - img_x_stride) / 2;
    let y_offset: u32;
    if img_y_stride < PHOTO_H {
        y_offset = PADDING + ((PHOTO_H - img_y_stride) / 2);
    } else {
        y_offset = PADDING
    };
    res.enumerate_pixels().into_iter().for_each(|px| {
        image.draw_pixel(px.0 + x_offset, px.1 + y_offset, px.2.clone());
    });
    PngEncoder::new(cursor).encode(image.as_bytes(), IMAGE_SIZE, IMAGE_SIZE, ColorType::Rgb8)?;
    Ok(out)
}

async fn encode_video(frame: Vec<u8>, audio: Option<Vec<u8>>) -> Result<Vec<u8>, HandlerError> {
    debug!("--->>> encode_video start");
    if check_ffmpeg_exist() {
        encode_video_local(frame, audio)
    } else {
        encode_video_remote(frame, audio).await
    }
}

async fn encode_video_remote(
    frame: Vec<u8>,
    audio: Option<Vec<u8>>,
) -> Result<Vec<u8>, HandlerError> {
    debug!("--->>> encode_video REMOTE");
    let mut data =
        multipart::Form::new().part("data", multipart::Part::bytes(frame).file_name("data.png"));
    if let Some(audio) = audio {
        data = data.part(
            "audio",
            multipart::Part::bytes(audio).file_name("audio.mp3"),
        );
    }
    let response = CLIENT
        .post(CONVERTER_URL.as_str())
        .timeout(Duration::from_secs(300))
        .multipart(data)
        .send()
        .await?;
    Ok(response.bytes().await?.to_vec())
}
