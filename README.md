## ビルド (WIP)

### ローカルツールのビルド

```shell
cargo build --release --bin judge
```

### Lambda コンテナ

- WSL2 に Ubuntu (22.04) をインストール
- Rust をインストール
- docker をインストール
- cargo-lambda をインストール
- `lambda-container/build.sh` を実行

## 使い方 (WIP)

- Windows 上の AWS CLI で、 ECR プライベートリポジトリと AWS Lambda の権限があるアカウントにログイン
- [コンテナイメージ](https://hub.docker.com/repository/docker/roumcha/dual-judge-lambda/general) を pull し、そのまま Amazon ECR の**プライベート**レポジトリに上げる\
  （.NET 以外のランタイムが必要な言語を使うなら、[リリース](https://github.com/roumcha/dual-judge/releases)から bootstrap をダウンロードして Ubuntu イメージを新しく作る。bootstrap には実行権限を付与しておく。）
- ECR のコンテナイメージから Lambda 関数を作り、メモリを 1800 MB、実行時間を 10 秒くらいに設定
- WSL 上の Ubuntu 22 以上で、[リリース](https://github.com/roumcha/dual-judge/releases)の `dual-judge-...zip` をダウンロード、展開し、コンテストフォルダとする
- judge-config.yaml を適宜書き換える
- `judge` を呼び出して並列テスト

## 更新リリース

- Cargo.toml でバージョンを更新
- Lambda コンテナイメージをビルドして Docker Hub にプッシュ

```shell
cargo build --release --bin judge

cp -f ./target/release/judge ./judge

chmod 755 start_lambda.sh start_local.sh judge

tar zcvf dual-judge-20__._._.tar.gz in/ start_lambda.sh start_local.sh judge_config.yaml judge
```
