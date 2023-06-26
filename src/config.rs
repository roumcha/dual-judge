#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{fs, path::PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config {
    pub subm_id: u32,
    pub case_dir: PathBuf,
    pub allow_non_ac: usize,
    pub local: LocalConfig,
    pub lambda: LambdaConfig,
    pub parse_result: ParseResultConfig,
}

impl Config {
    pub fn load_and_rotate_id(path: &str) -> Result<Config> {
        let yaml = fs::read_to_string(path)?;

        let config: Config = serde_yaml::from_str(&yaml).context("設定ファイルが誤っています")?;

        let next_yaml = Regex::new(r"subm_id: ?[0-9]+").unwrap().replace(
            &yaml,
            format!("subm_id: {}", (config.subm_id + 1).to_string()),
        );

        fs::write(path, next_yaml.as_bytes()).context("設定ファイルを上書きできません")?;

        Ok(config)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LocalConfig {
    pub pre: Option<String>,
    pub parallel: usize,
    pub send: Vec<FileTransferConfig>,
    pub collect: Vec<FileTransferConfig>,
    pub post: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LambdaConfig {
    pub pre: Option<String>,
    pub parallel: usize,
    pub function_name: String,
    pub send: Vec<FileTransferConfig>,
    pub collect: Vec<FileTransferConfig>,
    pub post: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct FileTransferConfig {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ParseResultConfig {
    pub files: Vec<PathBuf>,
    pub time_regex: String,
    pub time_multiplier: f64,
    pub score_regex: String,
    pub score_multiplier: f64,
    pub rate_regex: String,
    pub rate_multiplier: f64,
    pub force_ac_regex: String,
    pub ie_regex: String,
    pub ce_regex: String,
    pub re_regex: String,
    pub qle_regex: String,
    pub ole_regex: String,
    pub wa_regex: String,
    pub tle_regex: String,
    pub mle_regex: String,
}
