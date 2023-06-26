#!/usr/bin/env bash
set -xe
export AWS_LAMBDA=1
export DOTNET_EnableWriteXorExecute=0
export DOTNET_CLI_TELEMETRY_OPTOUT=1

# chmod 777 ./main && time ./main <in.txt >out.txt 2>err.txt
# chmod 777 .tester && ./tester ./main <in.txt >out.txt
# chmod 777 ./vis && ./vis in.txt out.txt

cat <<EOS
Score	1234567890 pt
real =1234ms
Rate:50.24%
[TLE]
EOS
