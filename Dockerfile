FROM rust:slim
RUN apt update && apt install librust-openssl-dev -y
RUN mkdir -p /opt/auth0-auth-server
COPY src /opt/auth0-auth-server/src
COPY Cargo.* /opt/auth0-auth-server/
RUN cd /opt/auth0-auth-server && cargo build --release --locked

FROM debian:stable-slim
MAINTAINER Hannes Hochreiner <hannes@hochreiner.net>
RUN apt update && apt install openssl ca-certificates -y
COPY --from=0 /opt/auth0-auth-server/target/release/auth0-auth-server /opt/auth0-auth-server
EXPOSE 8888
VOLUME /var/auth0-auth-server/config.json
ENV AUTH0_CONFIG /var/auth0-auth-server/config.json
ENV AUTH0_BIND_ADDRESS 127.0.0.1:8888
CMD ["/opt/auth0-auth-server"]
