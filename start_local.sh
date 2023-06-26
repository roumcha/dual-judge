#!/usr/bin/env bash
set -xe
export DOTNET_CLI_TELEMETRY_OPTOUT=1

# time ./main <in.txt >out.txt 2>err.txt
# ./tester ./main <in.txt >out.txt
# ./vis in.txt out.txt

sleep 4

cat <<EOS
Score	1234567890 pt
real    0m1.002s
Rate:50.24%
[MLE]
EOS
