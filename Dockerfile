FROM kyras/tezedge_base:latest as builder
WORKDIR /home/appuser/

# https://blog.mgattozzi.dev/caching-rust-docker-builds/
# Prepare empty binaries and all the dependencies that we have in Cargo.toml
RUN mkdir -p ./src/bin && \
    echo "fn main() {}" > ./src/bin/debugger.rs && \
    echo "fn main() {}" > ./src/bin/drone_test_server.rs && \
    echo "fn main() {}" > ./src/bin/drone_test_client.rs
COPY Cargo.lock .
COPY Cargo.toml .
# This step cache's our deps!
RUN cargo install --bins --path . --root . && rm -R ./src
# Copy the rest of the files into the container
COPY . .
# Now this only builds our changes to things like src
RUN cargo install --bins --path . --root .
# On my machine 7 minutes is reduced to 50 seconds, still slow, but much better

FROM ubuntu:latest
RUN apt-get -qy update && apt-get -qy install openssl
WORKDIR /home/appuser/
COPY --from=builder /home/appuser/bin ./
#CMD ["./tezedge-debugger"]
