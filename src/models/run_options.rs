use crate::utils::version::VERSION_STRING;
use clap::{AppSettings, Clap};

#[derive(Clap)]
#[clap(version = VERSION_STRING)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct RunOptions {
    #[clap(short, long)]
    pub debug: bool,
}

impl RunOptions {
    pub fn new() -> Self {
        Self::parse()
    }
}
