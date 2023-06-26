FROM ubuntu:kinetic as judge

RUN apt-get update && apt-get install -y dotnet-runtime-7.0
COPY bootstrap /function/

CMD [ "/function/bootstrap" ]
