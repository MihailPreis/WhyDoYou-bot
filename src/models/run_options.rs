use crate::utils::version::VERSION_STRING;
use clap::Parser;

#[derive(Parser)]
#[command(version = VERSION_STRING)]
pub struct RunOptions {
    #[arg(short, long)]
    pub debug: bool,
}

impl RunOptions {
    pub fn new() -> Self {
        Self::parse()
    }
}
