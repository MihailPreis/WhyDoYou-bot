use crate::models::db_conn::setup_db;
use crate::models::run_options::RunOptions;
use crate::utils::logger::setup_logger;

pub mod bots;
pub mod engine;
pub mod models;
pub mod utils;

cfg_if::cfg_if! {
    if #[cfg(feature = "tg")] {
        async fn run() {
            use crate::bots::tg::run_tg_bot;
            run_tg_bot().await;
        }
    } else {
        async fn run() {
            println!("No selected feature");
        }
    }
}

pub async fn start() {
    dotenv::dotenv().ok();
    setup_logger(&RunOptions::new()).unwrap();
    setup_db().await.unwrap();
    run().await;
}
