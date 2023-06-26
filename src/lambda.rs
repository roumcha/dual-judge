#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    fmt::Write as _,
    fs::{self, File},
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;

use aws_sdk_lambda::{primitives::Blob, Client};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::{
    config::{Config, FileTransferConfig},
    console_styles::ConsoleStyles,
    submission_state::SubmissionStateSingle::*,
    summary::{CaseSummary, FinalSummary},
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SendItem {
    pub path: PathBuf,
    pub data: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub send: Vec<SendItem>,
    pub collect: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectedItem {
    pub path: PathBuf,
    pub data: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub message: String,
    pub collected: Vec<CollectedItem>,
}

struct ParallelArg {
    subm_dir: PathBuf,
    config: Config,
    cs: ConsoleStyles,
    summary: Mutex<FinalSummary>,
    semaphore: Semaphore,
}

pub async fn run_all(
    casefiles: &[PathBuf],
    subm_dir: &Path,
    config: &Config,
    cs: &ConsoleStyles,
) -> FinalSummary {
    if let Some(commandline) = &config.lambda.pre {
        println!("{}", cs.cyan.apply_to("=> pre コマンドの実行"));
        if let Err(e) = crate::run_command(commandline) {
            println!("pre の実行に失敗しました: {e:?}");
        }
    };

    let arg = Arc::new(ParallelArg {
        subm_dir: subm_dir.to_path_buf(),
        config: config.clone(),
        cs: cs.clone(),
        summary: Mutex::new(FinalSummary::zero(config.subm_id)),
        semaphore: Semaphore::new(config.lambda.parallel),
    });

    let parallel: Vec<_> = casefiles
        .iter()
        .map(|casefile| create_parallel(casefile.clone(), arg.clone()))
        .collect();

    for p in parallel {
        p.await.unwrap();
    }

    if let Some(commandline) = &config.lambda.post {
        println!("{}", cs.cyan.apply_to("=> post コマンドの実行"));
        if let Err(e) = crate::run_command(commandline) {
            println!("post の実行に失敗しました: {e:?}");
        }
    };

    arg.clone().summary.lock().unwrap().to_owned()
}

fn create_parallel(casefile: PathBuf, arg: Arc<ParallelArg>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let _permit = arg.semaphore.acquire().await.unwrap();
        let casename = casefile.file_stem().unwrap().to_str().unwrap();
        let casefile = casefile.to_path_buf();
        let casedir = PathBuf::from(arg.subm_dir.join(format!("c_{casename}")));
        fs::create_dir_all(&casedir).unwrap();

        let case_summary = run_each(casename, &casefile, &casedir, &arg.config).await;

        if case_summary.state == AC as u32 {
            println!("{}", case_summary);
        } else {
            println!("{}", arg.cs.red.apply_to(&case_summary));
        }

        let mut summary = arg.summary.lock().unwrap();
        *summary = summary.next_case(&case_summary);
    })
}

async fn run_each(casename: &str, casefile: &Path, resdir: &Path, config: &Config) -> CaseSummary {
    let mut msg = String::new();
    writeln!(msg, "[CLI] 提出ID: {}, ケース: {casename}", config.subm_id).unwrap();
    writeln!(msg, "[CLI] 開始時刻: {}", Local::now()).unwrap();

    let mut summary = match lambda_request(config, casefile, &resdir, &mut msg).await {
        Ok(()) => CaseSummary::zero(casename, AC as u32),
        Err(()) => CaseSummary::zero(casename, IE as u32),
    };

    let mut msgfile = File::create(resdir.join("message.txt")).unwrap();
    msgfile.write_all(msg.as_bytes()).unwrap();
    drop(msgfile);

    for file in &config.parse_result.files {
        if let Ok(s2) = &CaseSummary::parse_file(casename, &resdir.join(file), config) {
            summary = summary.merge(s2);
        }
    }

    summary
}

async fn lambda_request(
    config: &Config,
    casefile: &Path,
    resdir: &Path,
    msg: &mut String,
) -> Result<(), ()> {
    let mut send = vec![];

    for t in &config.lambda.send {
        match prepare_send(t, casefile) {
            Ok(senditem) => send.push(senditem),
            Err(e) => {
                writeln!(
                    msg,
                    "[CLI] [IE] 送信ファイルを圧縮できません: {t:#?}\n{e:#?}"
                )
                .unwrap();
                return Err(());
            }
        };
    }

    let collect = config
        .lambda
        .collect
        .iter()
        .map(|t| t.from.clone())
        .collect();

    let request = Request { send, collect };

    let request_json = match serde_json::to_vec(&request) {
        Ok(v) => v,
        Err(e) => {
            writeln!(msg, "[CLI] [IE] JSON化できません: {request:?}\n{e:#?}").unwrap();
            return Err(());
        }
    };

    let sdk_config = aws_config::from_env().load().await;

    let output = match Client::new(&sdk_config)
        .invoke()
        .function_name(&config.lambda.function_name)
        .payload(Blob::new(request_json.clone()))
        .send()
        .await
    {
        Ok(x) => x,
        Err(e) => {
            writeln!(msg, "[CLI] [IE] 通信エラー: {e:#?}").unwrap();
            return Err(());
        }
    };

    let response_payload = match output.payload() {
        Some(blob) => blob,
        None => {
            writeln!(msg, "[CLI] [IE] AWS Lambda からの応答が空です").unwrap();
            return Err(());
        }
    }
    .to_owned()
    .into_inner();

    let response: Response = match serde_json::from_slice(&response_payload) {
        Ok(r) => r,
        Err(e) => {
            writeln!(msg, "[CLI] [IE] AWS Lambda からの応答が空です: {e:#?}").unwrap();
            return Err(());
        }
    };

    writeln!(msg, "{}", response.message).unwrap();

    for item in &response.collected {
        if let Err(e) = save_collected(item, resdir, config) {
            writeln!(
                msg,
                "[CLI] 回収したファイル {} が保存できませんが続行します: {e:#?}",
                item.path.display()
            )
            .unwrap();
        }
    }

    Ok(())
}

fn prepare_send(transfer_config: &FileTransferConfig, casefile: &Path) -> Result<SendItem> {
    let from = Path::new(&transfer_config.from)
        .to_string_lossy()
        .replace("$casefile", &casefile.to_string_lossy());

    Ok(SendItem {
        path: transfer_config.to.clone(),
        data: crate::encode_file(&PathBuf::from(&from))?,
    })
}

fn save_collected(collected: &CollectedItem, resdir: &Path, config: &Config) -> Result<PathBuf> {
    let to = &config
        .lambda
        .collect
        .iter()
        .find(|x| x.from == collected.path)
        .ok_or(io::Error::from(io::ErrorKind::NotFound))?
        .to;

    crate::decode_file(&collected.data, &resdir.join(to))?;
    Ok(to.into())
}
