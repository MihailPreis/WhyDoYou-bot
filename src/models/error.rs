use fern::InitError;
use image::ImageError;
use log::SetLoggerError;
use mime::FromStrError;
use reqwest::Error;
#[cfg(feature = "db")]
use sqlx::migrate::MigrateError;
use std::num::TryFromIntError;
#[cfg(feature = "tg")]
use teloxide::{DownloadError, RequestError};

#[derive(Debug, PartialEq)]
pub struct HandlerError {
    pub message: Option<String>,
}

/// Custom error for all cases of life
impl HandlerError {
    pub fn empty() -> Self {
        HandlerError { message: None }
    }

    pub fn new(data: String) -> Self {
        HandlerError {
            message: data.into(),
        }
    }

    pub fn from_str(data: &str) -> Self {
        HandlerError {
            message: String::from(data).into(),
        }
    }
}

impl<T> From<Option<T>> for HandlerError {
    fn from(_: Option<T>) -> Self {
        HandlerError::new(String::from("Can not get data from opt"))
    }
}

#[cfg(feature = "tg")]
impl From<RequestError> for HandlerError {
    fn from(e: RequestError) -> Self {
        HandlerError::new(format!("Teloxide request error: {:?}", e).to_string())
    }
}

impl From<ImageError> for HandlerError {
    fn from(e: ImageError) -> Self {
        HandlerError::new(format!("Image error: {:?}", e).to_string())
    }
}

#[cfg(feature = "db")]
impl From<sqlx::Error> for HandlerError {
    fn from(e: sqlx::Error) -> Self {
        HandlerError::new(format!("Sqlx error: {:?}", e).to_string())
    }
}

#[cfg(feature = "tg")]
impl From<DownloadError> for HandlerError {
    fn from(e: DownloadError) -> Self {
        HandlerError::new(format!("Teloxide download error: {:?}", e).to_string())
    }
}

impl From<FromStrError> for HandlerError {
    fn from(e: FromStrError) -> Self {
        HandlerError::new(format!("Mime error: {:?}", e).to_string())
    }
}

#[cfg(feature = "db")]
impl From<MigrateError> for HandlerError {
    fn from(e: MigrateError) -> Self {
        HandlerError::new(format!("Migrate error: {:?}", e).to_string())
    }
}

impl From<std::io::Error> for HandlerError {
    fn from(e: std::io::Error) -> Self {
        HandlerError::new(format!("IO error: {:?}", e).to_string())
    }
}

impl From<reqwest::Error> for HandlerError {
    fn from(e: Error) -> Self {
        HandlerError::new(
            format!("Network request was finished up with error: {:?}", e).to_string(),
        )
    }
}

impl From<fern::InitError> for HandlerError {
    fn from(e: InitError) -> Self {
        HandlerError::new(format!("Logger init error: {:?}", e).to_string())
    }
}

impl From<log::SetLoggerError> for HandlerError {
    fn from(e: SetLoggerError) -> Self {
        HandlerError::new(format!("Logger set error: {:?}", e).to_string())
    }
}

impl From<TryFromIntError> for HandlerError {
    fn from(_: TryFromIntError) -> Self {
        HandlerError::new("Can not convert value to int!!".to_string())
    }
}
