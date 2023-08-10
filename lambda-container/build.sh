parent=$(dirname ${0})

cd "$parent/../"
cargo lambda build --bin bootstrap --release
cp -f "target/lambda/bootstrap/bootstrap" "$parent/bootstrap"
chmod 755 bootstrap

cd $parent
docker build --tag dual-judge-lambda .
