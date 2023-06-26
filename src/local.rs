#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    fmt::Write as _,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use tempfile::TempDir;
use tokio::sync::Semaphore;

use chrono::Local;

use crate::{
    config::Config,
    console_styles::ConsoleStyles,
    submission_state::SubmissionStateSingle::*,
    summary::{CaseSummary, FinalSummary},
};

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
    if let Some(commandline) = &config.local.pre {
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
        semaphore: Semaphore::new(config.local.parallel),
    });

    let parallel: Vec<_> = casefiles
        .iter()
        .map(|casefile| create_parallel(casefile.clone(), arg.clone()))
        .collect();

    for p in parallel {
        p.await.unwrap();
    }

    if let Some(commandline) = &config.local.post {
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

    let temp_dir = TempDir::new().unwrap();
    writeln!(msg, "[CLI] {} で実行されます", temp_dir.path().display()).unwrap();

    let mut summary = match local_request(&temp_dir, config, casefile, &resdir, &mut msg).await {
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

async fn local_request(
    temp_dir: &TempDir,
    config: &Config,
    casefile: &Path,
    resdir: &&Path,
    msg: &mut String,
) -> Result<(), ()> {
    writeln!(msg, "[CLI] ファイルの送信").unwrap();
    for t in &config.local.send {
        if let Err(e) = fs::copy(
            &t.from
                .to_string_lossy()
                .replace("$casefile", &casefile.to_string_lossy()),
            temp_dir.path().join(&t.to),
        ) {
            writeln!(
                msg,
                "[CLI] [IE] ファイル {} を {} にコピーできません{e:?}",
                t.from.display(),
                temp_dir.path().join(&t.to).display()
            )
            .unwrap();
            return Err(());
        }
    }

    writeln!(msg, "[CLI] コマンドの実行").unwrap();
    if let Err(e) = exec_start_sh(temp_dir) {
        writeln!(msg, "[CLI] [IE] start.sh を正常に実行できません: {e:?}").unwrap();
    }

    writeln!(msg, "[CLI] ファイルの回収").unwrap();
    for t in &config.local.collect {
        if let Err(e) = fs::copy(temp_dir.path().join(&t.from), resdir.join(&t.to)) {
            writeln!(
                msg,
                "[CLI] ファイル {} を {} にコピーできません{e:?}",
                temp_dir.path().join(&t.from).display(),
                resdir.join(&t.to).display()
            )
            .unwrap();
        }
    }

    writeln!(msg, "[CLI] 実行完了").unwrap();
    Ok(())
}

fn exec_start_sh(temp_dir: &TempDir) -> Result<()> {
    let mut outfile = File::create(temp_dir.path().join("start_out.txt"))
        .context("start_out.txt が作成できません")?;
    let mut errfile = File::create(temp_dir.path().join("start_err.txt"))
        .context("start_err.txt が作成できません")?;

    let output = Command::new("bash")
        .arg(temp_dir.path().join("start.sh"))
        .stdout(Stdio::from(
            outfile
                .try_clone()
                .context("start_out.txt に接続できません")?,
        ))
        .stderr(Stdio::from(
            errfile
                .try_clone()
                .context("start_err.txt に接続できません")?,
        ))
        .spawn()
        .context("プロセスが起動できません")?
        .wait_with_output()
        .context("bash の待機中にエラーが発生しました")?;

    let out_res = outfile
        .write_all(&output.stdout)
        .context("start_out.txt に書き込めません");

    let err_res = errfile
        .write_all(&output.stderr)
        .context("start_err.txt に書き込めません");

    ensure!(output.status.success(), "start.sh が異常終了しました");

    out_res
        .and(err_res)
        .context("start.sh の出力が最後まで取得できませんでした")
}
