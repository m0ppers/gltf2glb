FROM debian:buster as builder

RUN apt-get update && apt-get install -y curl build-essential \
        && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain stable -y

ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /src

COPY . .

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/gltf2glb /gltf2glb

CMD ["/gltf2glb"]