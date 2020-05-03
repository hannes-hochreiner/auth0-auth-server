![Testing](https://github.com/hannes-hochreiner/auth0-auth-server/workflows/Testing/badge.svg)

# Auth0 Authentication Server
An authentication server using Auth0 as a backend.

The server takes the scopes from the JSON Web Token (JWT) and compares them against a configuration.
If one or more scopes are found that match the configuration, the request succeeds (status 200) and the scopes are reported back in the custom header "x-groups" as comma separated string.
Otherwise, the request fails (status 403).

## Usage
The authentication server is meant to be used for authentication sub-queries (e.g. from nginx).
It expects a configuration file as follows:

```json
{
  "jwksUri": "<string>",
  "audience": "<string>",
  "issuer": "<string>",
  "algorithms": ["<algorithm1>", "<algorithm2>"],
  "auth": {
    "<path1>": {
      "<verb1 e.g. GET>": ["role/scope 1", "role/scope 2"],
      "<verb2 e.g. POST>": ["role/scope 3"]
    }
  }
}
```
The server can be started using the following command:
```shell
node bld/main.js -c config.json
```
The server binds to port 127.0.0.1:8888.

Example nginx configuration:
```
server {
  ...

  location /auth {
    internal;
    proxy_pass              http://server_auth:8888;
    proxy_pass_request_body off;
    proxy_set_header        Content-Length "";
    proxy_set_header        X-Original-URI $request_uri;
    proxy_set_header        X-Original-METHOD $request_method;
  }

  location /api {
    auth_request     /auth;
    auth_request_set        $auth_user $upstream_http_x_id;
    auth_request_set        $auth_groups $upstream_http_x_groups;
    proxy_set_header        X-Auth-UserName $auth_user;
    proxy_set_header        X-Auth-Roles $auth_groups;
    ...
  }
}
```

## Path/Role Resolution
All configured paths are compared with the starting portion of the requested path.
The longest matching path is selected and the roles associated with the requested verb are used to compare against the scopes from the token.

### Example
Configuration:
```json
{
  "jwksUri": "https://<tenant>.<region>.auth0.com/.well-known/jwks.json",
  "audience": "https://<audience>.net",
  "issuer": "https://<tenant>.<region>.auth0.com/",
  "algorithms": ["RS256"],
  "auth": {
    "/": {
      "GET": ["read:user"],
    },
    "/api": {
      "GET": ["read:admin"],
      "POST": ["write:admin"]
    }
  }
}
```
Request 1
  * Request: GET /image/xyz (is matched to "/")
  * Response: 200 (if token contains "read:user"), 403 (otherwise)

Request 2
  * Request: POST /api/objects (is matched to "/api")
  * Response: 200 (if token contains "write:admin"), 403 (otherwise)