use crate::models::error::HandlerError;
use log::debug;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

pub fn encode_video_local(frame: Vec<u8>, audio: Option<Vec<u8>>) -> Result<Vec<u8>, HandlerError> {
    debug!("--->>> encode_video LOCAL");

    let uuid = Uuid::new_v4()
        .hyphenated()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned();
    let jpg_file = uuid.clone() + ".jpg";
    let mp4_file = uuid.clone() + ".mp4";
    let mp3_file = uuid.clone() + ".mp3";

    safe_remove(jpg_file.as_str());
    safe_remove(mp4_file.as_str());
    safe_remove(mp3_file.as_str());

    File::create(jpg_file.as_str())
        .expect("Unable to create image file")
        .write_all(&*frame)
        .expect("Unable to write image data");

    let mut args: Vec<&str> = Vec::from([
        "ffmpeg",
        "-loop",
        "1",
        "-i",
        jpg_file.as_str(),
        "-i",
        "assets/input.mp3",
    ]);
    if let Some(audio) = audio {
        File::create(mp3_file.as_str())
            .expect("Unable to create image file")
            .write_all(&*audio)
            .expect("Unable to write image data");
        args.extend_from_slice(&["-i", mp3_file.as_str()]);
    } else {
        args.extend_from_slice(&["-i", "assets/input.mp3"]);
    }
    args.extend_from_slice(&[
        "-c:v",
        "libx264",
        "-tune",
        "stillimage",
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        "-pix_fmt",
        "yuv420p",
        "-shortest",
        "-t",
        "10",
        mp4_file.as_str(),
    ]);
    let is_success = eval(&*args)
        .expect("FFMPEG exit with error.")
        .status
        .success();

    if is_success {
        debug!("--->>> encode_video LOCAL :: success");
        let result = std::fs::read("out.mp4")?;
        safe_remove(jpg_file.as_str());
        safe_remove(mp4_file.as_str());
        Ok(result)
    } else {
        debug!("--->>> encode_video LOCAL :: error");
        safe_remove(jpg_file.as_str());
        safe_remove(mp4_file.as_str());
        Err(HandlerError::empty())
    }
}

pub fn check_ffmpeg_exist() -> bool {
    return match eval(&["ffmpeg -version"]) {
        Err(_) => false,
        Ok(output) => output.status.success(),
    };
}

fn eval(args: &[&str]) -> io::Result<Output> {
    return if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").args(args).output()
    } else {
        Command::new("sh").arg("-c").args(args).output()
    };
}

fn safe_remove(file_name: &str) {
    if Path::new(file_name).exists() {
        debug!("--->>> encode_video LOCAL :: remove file {}", file_name);
        std::fs::remove_file(file_name).unwrap();
    }
}
