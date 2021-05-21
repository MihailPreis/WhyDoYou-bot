use std::ops::Deref;
use std::sync::{Arc, Mutex};
use why_do_you_bot::engine::engine::build_message;
use why_do_you_bot::models::error::HandlerError;
use why_do_you_bot::models::v_data::VData;

#[tokio::test]
async fn engine_not_match() {
    let words: Option<String> = Some(String::from("test"));
    let image_handler = async move { return None };
    let audio_handler = async move { return None };
    match build_message("wow", words, image_handler, audio_handler).await {
        Ok(_) => {
            panic!("Can't be Ok(_)")
        }
        Err(err) => {
            assert_eq!(err, HandlerError::empty())
        }
    }
}

#[tokio::test]
async fn engine_match() {
    std::env::set_var("CONVERTER_URL", "");

    let words: Option<String> = Some(String::from("test"));

    let mut _is_call_image_handler = Arc::new(Mutex::new(false));
    let _is_call_image_handler_writer = Arc::clone(&_is_call_image_handler);
    let image_handler = async move {
        *_is_call_image_handler_writer.lock().unwrap() = true;
        return None;
    };

    let mut _is_call_audio_handler = Arc::new(Mutex::new(false));
    let _is_call_audio_handler_writer = Arc::clone(&_is_call_audio_handler);
    let audio_handler = async move {
        *_is_call_audio_handler_writer.lock().unwrap() = true;
        return None;
    };

    match build_message("test", words, image_handler, audio_handler).await {
        Ok(v_data) => match v_data {
            VData::Image(c) => {
                assert!(c.len() > 0, "Image is empty");
                assert!(
                    _is_call_image_handler.lock().unwrap().deref(),
                    "Image handler was not called"
                );
                assert!(
                    _is_call_audio_handler.lock().unwrap().deref(),
                    "Audio handler was not called"
                );
            }
            VData::Video(_) => {
                panic!("Can't be Video(_)")
            }
        },
        Err(_) => {
            panic!("Can't be Err(_)")
        }
    }
}
