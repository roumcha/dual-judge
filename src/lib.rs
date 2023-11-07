pub mod config;
pub mod console_styles;
pub mod lambda;
pub mod local;
pub mod submission_state;
pub mod summary;

#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

use base64::engine::{general_purpose, Engine as _};
use flate2::{
    write::{DeflateDecoder, DeflateEncoder},
    Compression,
};

pub fn now() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono_tz::Japan)
        .to_string()
}

pub fn comma_sep_int(number: i128) -> String {
    let mut s = number.to_string().into_bytes();
    s.reverse();
    let mut res = vec![];
    for i in 0..s.len() {
        if i > 0 && i % 3 == 0 {
            res.push(b',');
        }
        res.push(s[i]);
    }
    res.reverse();
    String::from_utf8_lossy(&res).into_owned()
}

pub fn run_command(commandline: &str) -> Result<()> {
    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", commandline]).spawn()?
    } else {
        Command::new("sh").arg("-c").arg(commandline).spawn()?
    };

    match child.wait()?.success() {
        true => Ok(()),
        false => Err(anyhow!("コマンドが異常終了しました")),
    }
}

pub fn encode_file(path: &Path) -> Result<String> {
    let data = fs::read(path)
        .with_context(|| format!("エンコード対象ファイル {path:?} が読み取れません"))?;
    crate::encode(&data).with_context(|| format!("ファイル {path:?} をエンコードできません"))
}

pub fn encode(data: &[u8]) -> Result<String> {
    let mut gz = DeflateEncoder::new(vec![], Compression::default());
    gz.write_all(&data)?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(gz.finish()?))
}

pub fn decode_file(data: &str, path: &Path) -> Result<()> {
    let mut f = File::create(path)
        .with_context(|| format!("デコード先ファイル {path:?} が作成できません"))?;

    let decoded_bytes = crate::decode(data.as_bytes())
        .with_context(|| format!("データのデコードが失敗しました（パス: {path:?}）"))?;

    f.write_all(&decoded_bytes)
        .with_context(|| format!("デコード先ファイル {path:?} に書き込めません"))
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    let bytes = general_purpose::STANDARD_NO_PAD.decode(data)?;
    let mut gz = DeflateDecoder::new(vec![]);
    gz.write_all(&bytes)?;
    Ok(gz.finish()?)
}
