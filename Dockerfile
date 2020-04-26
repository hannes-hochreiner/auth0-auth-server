FROM alpine:latest
MAINTAINER Hannes Hochreiner <hannes@hochreiner.net>
RUN apk add --no-cache nodejs nodejs-npm
RUN mkdir -p /opt/auth0-auth-server
COPY src /opt/auth0-auth-server/src
COPY package.json /opt/auth0-auth-server/package.json
COPY babel.config.json /opt/auth0-auth-server/babel.config.json
RUN cd /opt/auth0-auth-server && npm install && npm run build
EXPOSE 8888
VOLUME /var/auth0-auth-server/config.json
CMD ["node", "/opt/auth0-auth-server/bld/main", "-c", "/var/auth0-auth-server/config.json"]
