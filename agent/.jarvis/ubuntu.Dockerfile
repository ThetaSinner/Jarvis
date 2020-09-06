FROM ubuntu:latest

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl build-essential

RUN curl https://sh.rustup.rs -sSf > rust-installer.sh && \
    chmod +x rust-installer.sh && \
    ./rust-installer.sh -y

ENV PATH="/root/.cargo/bin:$PATH"
