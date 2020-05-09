FROM node:lts-alpine
RUN mkdir -p /opt/auth0-auth-server
COPY src /opt/auth0-auth-server/src
COPY package*.json /opt/auth0-auth-server/
COPY babel.config.json /opt/auth0-auth-server/babel.config.json
RUN cd /opt/auth0-auth-server && npm install && npm run build

FROM node:lts-alpine
MAINTAINER Hannes Hochreiner <hannes@hochreiner.net>
COPY --from=0 /opt/auth0-auth-server/bld /opt/auth0-auth-server
COPY --from=0 /opt/auth0-auth-server/package*.json /opt/auth0-auth-server/
RUN cd /opt/auth0-auth-server && npm install --production
EXPOSE 8888
VOLUME /var/auth0-auth-server/config.json
CMD ["node", "/opt/auth0-auth-server/main", "-c", "/var/auth0-auth-server/config.json"]
