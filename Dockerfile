FROM rust:alpine3.13
RUN apk add openssl-dev
RUN mkdir -p /opt/auth0-auth-server
COPY src /opt/auth0-auth-server/src
COPY Cargo.* /opt/auth0-auth-server/
RUN cd /opt/auth0-auth-server && cargo build --release --locked

FROM alpine:3.13
MAINTAINER Hannes Hochreiner <hannes@hochreiner.net>
COPY --from=0 /opt/auth0-auth-server/target/release/auth0-auth-server /opt/auth0-auth-server
EXPOSE 8888
VOLUME /var/auth0-auth-server/config.json
CMD ["/opt/auth0-auth-server"]
