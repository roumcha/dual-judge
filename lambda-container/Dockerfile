FROM ubuntu:kinetic

# 使用する言語の実行時に必要なものをインストール
RUN apt-get update && apt-get install -y dotnet-runtime-7.0

COPY bootstrap /function/

CMD [ "/function/bootstrap" ]
