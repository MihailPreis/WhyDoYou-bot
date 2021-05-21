//! Logger section

use crate::models::error::HandlerError;
use crate::models::run_options::RunOptions;

const LOG_FILE_KEY: &str = "LOG_FILE";

/// Configuring logger
///
/// Parameters:
///  - args: commandline arguments
///
/// Return: result of void or HandlerError
pub fn setup_logger(args: &RunOptions) -> Result<(), HandlerError> {
    let level = if args.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let mut builder = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout());
    if let Some(log_file_path) = std::env::var(LOG_FILE_KEY).ok() {
        println!("Apply logging to {}", log_file_path);
        builder = builder.chain(fern::log_file(log_file_path)?);
    }
    builder.apply()?;
    Ok(())
}
