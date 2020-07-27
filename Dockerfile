FROM ubuntu:latest
RUN apt-get -qy update && \
    apt-get -qy install openssl && \
    apt-get -qy install ca-certificates
WORKDIR /home/appuser/
COPY --from=builder /home/appuser/bin ./
#CMD ["./tezedge-debugger"]
