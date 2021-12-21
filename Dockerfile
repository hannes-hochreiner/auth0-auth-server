FROM fedora:34 AS builder
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN dnf install gcc openssl-devel -y
RUN mkdir -p /opt/auth0-auth-server
COPY src /opt/auth0-auth-server/src
COPY Cargo.* /opt/auth0-auth-server/
RUN source $HOME/.cargo/env && cd /opt/auth0-auth-server && cargo build --release --locked

FROM fedora:34
MAINTAINER Hannes Hochreiner <hannes@hochreiner.net>
COPY --from=builder /opt/auth0-auth-server/target/release/auth0-auth-server /opt/auth0-auth-server
EXPOSE 8888
VOLUME /var/auth0-auth-server/config.json
ENV AUTH0_CONFIG /var/auth0-auth-server/config.json
ENV AUTH0_BIND_ADDRESS 127.0.0.1:8888
CMD ["/opt/auth0-auth-server"]
