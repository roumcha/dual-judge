use std::{
    env,
    fs::{self, OpenOptions},
    io::Write as _,
    path::PathBuf,
    process,
};

use console::Style;
use getopts::{Matches, Options};

use dual_judge::{config::Config, console_styles::ConsoleStyles, lambda, local};

#[tokio::main]
async fn main() {
    let args: Vec<_> = env::args().collect();
    let opt = parse_options(&args[1..]);

    let cs = if opt.opt_present("no-color") {
        ConsoleStyles {
            def: Style::default(),
            cyan: Style::default(),
            dim: Style::default(),
            red: Style::default(),
        }
    } else {
        ConsoleStyles {
            def: Style::default(),
            cyan: Style::new().cyan(),
            dim: Style::new().dim(),
            red: Style::new().red(),
        }
    };

    println!("{}", cs.cyan.apply_to("=> judge_config.yaml を読込＆更新"));
    let config = Config::load_and_rotate_id("./judge_config.yaml").unwrap();

    println!("{}", cs.cyan.apply_to("=> 結果フォルダの作成"));
    let subm_dir_name = format!("results/s_{:0>4}", config.subm_id);
    fs::create_dir_all(&subm_dir_name).expect(&format!("フォルダが作成できません {subm_dir_name}"));

    println!("{}", cs.cyan.apply_to("=> テストケースを決定"));
    let casefiles = get_casefiles(&opt, &config, &cs);

    println!("{}", cs.cyan.apply_to("=> 結果フォルダ作成"));
    let subm_dir = PathBuf::from(format!("results/s_{:0>4}", config.subm_id));
    fs::create_dir_all(&subm_dir).unwrap();

    let final_summary = if opt.opt_present("local") {
        println!("{}", cs.cyan.apply_to("=> ローカルで並列実行"));
        local::run_all(&casefiles, &subm_dir, &config, &cs).await
    } else if opt.opt_present("lambda") {
        println!("{}", cs.cyan.apply_to("=> AWS Lambda で並列実行"));
        // TODO?: warmup
        lambda::run_all(&casefiles, &subm_dir, &config, &cs).await
    } else {
        panic!("予期せぬエラー: --lambda / --local を1つ指定してください")
    };

    println!("{}", cs.cyan.apply_to("=> 要約の表示"));
    print!("{}", final_summary);
    let mut summary_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&subm_dir.join("summary.txt"))
        .unwrap();
    summary_file
        .write_all(&final_summary.to_string().as_bytes())
        .unwrap();
}

fn get_casefiles(opt: &Matches, config: &Config, cs: &ConsoleStyles) -> Vec<PathBuf> {
    let caseopts = opt.opt_strs("case");

    let casefiles: Vec<PathBuf> = if caseopts.len() > 0 {
        casefiles_selected(caseopts, config)
    } else {
        casefiles_auto(config)
    };

    let prints = format!(
        "{}{}\n計 {} ファイル",
        casefiles
            .iter()
            .map(|p| p.display().to_string())
            .take(casefiles.len().min(5))
            .collect::<Vec<_>>()
            .join(" "),
        (if casefiles.len() <= 5 { "" } else { "\n..." }),
        casefiles.len()
    );

    println!("{}", cs.dim.apply_to(prints));
    casefiles
}

fn casefiles_selected(caseopts: Vec<String>, config: &Config) -> Vec<PathBuf> {
    caseopts
        .iter()
        .map(|name| {
            let path = PathBuf::from(&config.case_dir).join(name);
            if path.is_file() {
                Ok(path)
            } else {
                Err(format!("ファイルがありません: {}", path.display()))
            }
        })
        .collect::<Result<_, _>>()
        .unwrap()
}

fn casefiles_auto(config: &Config) -> Vec<PathBuf> {
    PathBuf::from(&config.case_dir)
        .read_dir()
        .expect(
            format!(
                "テストケースフォルダが読み込めません: {}",
                config.case_dir.display()
            )
            .as_str(),
        )
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_file().then_some(path)
        })
        .collect()
}

fn parse_options(args: &[String]) -> Matches {
    let mut opts = Options::new();
    opts.optflag("", "local", "このパソコン上で実行（デフォルト）");
    opts.optflag("", "lambda", "AWS Lambda で実行");
    opts.optmulti("c", "case", "テストケースをファイル名で指定", "<name>");
    opts.optflag("", "no-color", "出力に色を付けない");
    opts.optflag("h", "help", "このヘルプを表示");

    let usage = opts.usage(&format!("Usage: judge [Options]"));

    let opt_match = opts.parse(&args[..]).unwrap_or_else(|e| {
        println!("{usage}");
        eprintln!("{e:?}");
        panic!("オプションが誤っています");
    });

    if opt_match.opt_present("help") {
        println!("{usage}");
        process::exit(0);
    }

    if opt_match.opt_count("lambda") + opt_match.opt_count("local") != 1 {
        println!("{usage}");
        panic!("--lambda / --local を1つ指定してください");
    }

    opt_match
}
