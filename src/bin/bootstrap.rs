#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    env,
    fmt::Write as _,
    fs::{self, File},
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

use lambda_runtime::{service_fn, LambdaEvent};

use dual_judge::{
    lambda::{CollectedItem, Request, Response},
    now,
};

const TMP_DIR: &str = "/tmp/";
const RUN_DIR: &str = "/tmp/runner/";

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    return lambda_runtime::run(service_fn(handler)).await;
}

async fn handler(event: LambdaEvent<Request>) -> Result<Response> {
    let request = event.payload;
    let mut log = String::new();

    writeln!(log, "[AWS][{}] ディレクトリ: {RUN_DIR}", now())?;
    fs::create_dir_all(RUN_DIR)?;
    env::set_current_dir(RUN_DIR)?;

    writeln!(log, "[AWS][{}] ファイルの展開", now())?;
    for sent in &request.send {
        dual_judge::decode_file(&sent.data, &sent.path).or_else(|e| {
            writeln!(
                log,
                "[AWS][{}][IE] {:?} に展開できません: {e:?}",
                now(),
                sent.path,
            )
        })?;
    }

    writeln!(log, "[AWS][{}] 実行ディレクトリに実行権限を付与", now())?;
    chmod_rec(Path::new(RUN_DIR)).or_else(|e| {
        writeln!(
            log,
            "[AWS][{}][IE] {RUN_DIR} の権限が変更できません: {e:?}",
            now()
        )
    })?;

    writeln!(log, "[AWS][{}] コマンドの実行", now())?;
    exec_start_sh().or_else(|e| {
        writeln!(
            log,
            "[AWS][{}][IE] {RUN_DIR} start.sh を正常に実行できません: {e:?}",
            now()
        )
    })?;

    writeln!(log, "[AWS][{}] ファイルの回収", now())?;
    let mut collected = vec![];
    for path in &request.collect {
        match dual_judge::encode_file(&path) {
            Ok(s) => collected.push(CollectedItem {
                path: path.clone(),
                data: s,
            }),
            Err(e) => {
                writeln!(
                    log,
                    "[AWS][{}] {path:?} が回収できませんが続行します: {e:?}",
                    now()
                )?;
            }
        }
    }

    env::set_current_dir(TMP_DIR)?;
    fs::remove_dir_all(RUN_DIR)?;
    writeln!(log, "[AWS][{}] 実行完了", now())?;

    Ok(Response {
        message: log,
        collected,
    })
}

fn chmod_rec(path: &Path) -> Result<()> {
    let output = Command::new("chmod")
        .arg("-R")
        .arg("777")
        .arg(format!("{}", path.display()))
        .spawn()?
        .wait_with_output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "\"chmod 777 -R {}\" failed: \n{}\n\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn exec_start_sh() -> Result<()> {
    let mut outfile = File::create("./start_out.txt").context("start_out.txt が作成できません")?;
    let mut errfile = File::create("./start_err.txt").context("start_err.txt が作成できません")?;

    let output = Command::new("bash")
        .arg("./start.sh")
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
