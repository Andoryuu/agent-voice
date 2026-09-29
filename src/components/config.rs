use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub model_path: String,
    pub config_path: String,
    pub monitored_path: String,
    pub speaker_id: i64,
}

impl Config {
    pub fn new() -> Config {
        let config_file = fs::read("config.yaml").unwrap();

        serde_yaml::from_slice(&config_file).unwrap()
    }
}
