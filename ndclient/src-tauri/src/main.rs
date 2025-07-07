// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    if cfg!(debug_assertions) {
        dotenv::from_filename(".env.dev").unwrap();
    } else {
        dotenv().unwrap();
    }

    ndclient_lib::run()
}
