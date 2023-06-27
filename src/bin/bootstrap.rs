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

use lambda_runtime::{run, service_fn, LambdaEvent};

use dual_judge::lambda::{CollectedItem, Request, Response};

#[tokio::main]
async fn main() -> ! {
    loop {
        fs::create_dir_all("/tmp/runner/").unwrap();
        env::set_current_dir("/tmp/runner/").unwrap();

        run(service_fn(main_each))
            .await
            .unwrap_or_else(|e| eprintln!("{e:?}"));

        env::set_current_dir("/tmp/").unwrap();
        fs::remove_dir_all("/tmp/runner/").unwrap();
    }
}

async fn main_each(event: LambdaEvent<Request>) -> Result<Response> {
    let mut message = String::new();
    let collected = handler(event, &mut message).await.unwrap_or(vec![]);
    Ok(Response { message, collected })
}

async fn handler(event: LambdaEvent<Request>, msg: &mut String) -> Result<Vec<CollectedItem>, ()> {
    let request = event.payload;

    writeln!(msg, "[実行環境] ファイルの展開").unwrap();
    for sent in &request.send {
        if let Err(e) = dual_judge::decode_file(&sent.data, &sent.path) {
            writeln!(
                msg,
                "[実行環境] [IE] 受信ファイル {:?} が展開できません: {e:?}",
                sent.path
            )
            .unwrap();
            return Err(());
        }
    }

    writeln!(msg, "[実行環境] 実行ディレクトリに実行権限を付与").unwrap();
    if let Err(e) = chmod_rec(Path::new("/tmp/runner/")) {
        writeln!(
            msg,
            "[実行環境] [IE] chmod -R 777 /tmp/runner/ ができません: {e:?}"
        )
        .unwrap();
        return Err(());
    }

    writeln!(msg, "[実行環境] コマンドの実行").unwrap();
    if let Err(e) = exec_start_sh() {
        writeln!(
            msg,
            "[実行環境] [IE] start.sh を正常に実行できません: {e:?}"
        )
        .unwrap();
    }

    writeln!(msg, "[実行環境] ファイルの回収").unwrap();
    let mut collected = vec![];
    for path in &request.collect {
        match dual_judge::encode_file(&path) {
            Ok(s) => collected.push(CollectedItem {
                path: path.clone(),
                data: s,
            }),
            Err(e) => {
                writeln!(msg, "[実行環境] ファイル {path:?} が回収できません: {e:?}").unwrap();
            }
        }
    }

    Ok(collected)
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
