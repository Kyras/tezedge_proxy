FROM kyras/tezedge_base:latest as builder
WORKDIR /home/appuser/
COPY . .
RUN cargo install --bins --path . --root .

FROM ubuntu:latest
RUN apt-get -qy update && apt-get -qy install openssl
WORKDIR /home/appuser/
COPY --from=builder /home/appuser/bin ./
#CMD ["./tezedge-debugger"]