# Signet Server in a container: only the shared world (no doors, no game data).
#   docker build -t signet .
#   docker run -d --name signet -p 127.0.0.1:7800:7800 [-e SIGNET_TOKEN=...] signet
#   another world: -v ~/my-world.txt:/world.txt ... signet server --bind 0.0.0.0:7800 --world /world.txt
# Publish the port on the address your players reach through YOUR VPN (127.0.0.1 = this machine only).
FROM rust:1-alpine AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY worlds ./worlds
RUN cargo build --release && strip target/release/signet

FROM scratch
COPY --from=build /src/target/release/signet /signet
COPY --from=build /src/worlds /worlds
USER 65534:65534
EXPOSE 7800/tcp
ENTRYPOINT ["/signet"]
CMD ["server", "--bind", "0.0.0.0:7800"]
