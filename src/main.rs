#[macro_use]
extern crate log;
use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use chrono::prelude::*;
use hyper::body::HttpBody;
use hyper::service::Service;
use hyper::{
    header::HeaderName, header::HeaderValue, Body, Client, HeaderMap, Request, Response, Server,
    StatusCode,
};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
mod error;
use error::AuthServerError;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct Configuration {
    audience: String,
    issuer: String,
    headerNames: HashMap<String, String>,
    auth: HashMap<String, HashMap<String, Vec<String>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config_filename = get_env("AUTH0_CONFIG").unwrap_or(String::from("config.json"));
    let file = File::open(config_filename)?;
    let reader = BufReader::new(file);
    let config: Configuration = serde_json::from_reader(reader)?;
    let issuer = config.issuer.clone();
    let address = get_env("AUTH0_BIND_ADDRESS").unwrap_or(String::from("127.0.0.1:8888"));
    let addr = (&*address).parse()?;
    info!("Starting server at {}", address);
    let (cmd_tx, cmd_rx) = mpsc::channel::<oneshot::Sender<anyhow::Result<JWKS>>>(100);

    tokio::spawn(get_jwks_wrapper(issuer, cmd_rx));

    let server = Server::bind(&addr).serve(ServiceFactory {
        sender: cmd_tx,
        configuration: config,
    });

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

async fn get_jwks_wrapper(
    issuer: String,
    mut cmd_rx: mpsc::Receiver<oneshot::Sender<anyhow::Result<JWKS>>>,
) -> anyhow::Result<()> {
    let mut jwks = get_jwks(&issuer).await;
    let mut update_time = Utc::now();

    while let Some(response) = cmd_rx.recv().await {
        let time_since_update = Utc::now() - update_time;

        log::debug!("time since JWKS udpate: {}", time_since_update);

        if jwks.is_err() || time_since_update > chrono::Duration::hours(1) {
            update_time = Utc::now();
            jwks = get_jwks(&*issuer).await;
        }

        response
            .send(match &jwks {
                Ok(value) => Ok(value.clone()),
                Err(e) => Err(anyhow::anyhow!("error retrieving JWKS: {:?}", e)),
            })
            .map_err(|e| anyhow::anyhow!("error sending response for JWKS: {:?}", e))?;
    }

    Ok(())
}

fn get_env(key: &str) -> Result<String, AuthServerError> {
    env::var(key).map_err(|e| AuthServerError::from((key, e)))
}

async fn get_jwks(issuer: &str) -> anyhow::Result<JWKS> {
    info!("Updating JWKS from {}.well-known/jwks.json", issuer);
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    let mut resp = timeout(
        std::time::Duration::from_secs(1),
        client.get(format!("{}.well-known/jwks.json", issuer).parse()?),
    )
    .await??;

    let mut body_vec: Vec<u8> = vec![];

    while let Some(chunk) = resp.body_mut().data().await {
        body_vec.append(&mut chunk?.to_vec());
    }

    let jwks: JWKS = serde_json::from_str(&*String::from_utf8(body_vec)?)?;

    Ok(jwks)
}

struct AuthorizationService {
    sender: mpsc::Sender<oneshot::Sender<anyhow::Result<JWKS>>>,
    configuration: Configuration,
}

impl Service<Request<Body>> for AuthorizationService {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let sender = self.sender.clone();
        let config = self.configuration.clone();

        // create a response in a future.
        let fut = async move {
            debug!("Requesting JWKS");
            let (resp_tx, resp_rx) = oneshot::channel();
            sender.send(resp_tx).await?;
            let jwks = resp_rx.await??;
            debug!("Obtained JWKS");

            // get token
            let authorization = match get_header_value("authorization", req.headers()) {
                Ok(val) => val,
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };
            let token = match authorization.split(" ").last() {
                Some(token) => token,
                None => {
                    error!("Could not find token");
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };
            let kid = token_kid(token)
                .map_err(|e| anyhow::anyhow!("error finding key: {:?}", e))?
                .ok_or(anyhow::anyhow!("error decoding key"))?;
            debug!("Decoded token key");
            let jwk = jwks
                .find(&kid)
                .ok_or(anyhow::anyhow!("key not found in set"))?;
            debug!("Found key");
            let validations = vec![
                Validation::Issuer(config.issuer.to_string()),
                Validation::Audience(config.audience.to_string()),
                Validation::SubjectPresent,
                Validation::NotExpired,
            ];
            let res = validate(token, jwk, validations)
                .map_err(|e| anyhow::anyhow!("validation failed: {:?}", e))?;
            let claims = res.claims;
            debug!("Found claims: {:?}", claims);
            let scopes: Vec<&str> = claims["scope"]
                .as_str()
                .ok_or(anyhow::anyhow!("no scope found"))?
                .split(" ")
                .collect();
            debug!("Found scopes: {:?}", scopes);

            let method_name = config
                .headerNames
                .get("method")
                .unwrap_or(&String::from("x-original-method"))
                .clone();
            let method = match get_header_value(&*method_name, req.headers()) {
                Ok(val) => {
                    debug!(
                        "Found forwarded method \"{}\" in header \"{}\"",
                        val, method_name
                    );
                    val
                }
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };

            let uri_name = config
                .headerNames
                .get("uri")
                .unwrap_or(&String::from("x-original-uri"))
                .clone();
            let uri = match get_header_value(&*uri_name, req.headers()) {
                Ok(val) => {
                    debug!("Found forwarded uri \"{}\" in header \"{}\"", val, uri_name);
                    val
                }
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };

            let mut matched_path: Option<&String> = None;

            for path in config.auth.keys() {
                if !uri.starts_with(path) {
                    continue;
                }

                match matched_path {
                    Some(m) => {
                        if m.len() < path.len() {
                            matched_path = Some(path);
                        }
                    }
                    None => matched_path = Some(path),
                }
            }

            let matched_path = match matched_path {
                Some(m) => {
                    debug!("Found matched path \"{}\"", m);
                    m
                }
                None => {
                    error!("No matched path found");
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };

            let required = match config.auth[matched_path].get(&*method) {
                Some(m) => {
                    debug!(
                        "Found required permissions for verb \"{}\": {:?}",
                        method, m
                    );
                    m
                }
                None => {
                    error!("No verb \"{}\" not found for path", method);
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())?);
                }
            };

            let mut relevant_scopes: Vec<&str> = vec![];

            for req in required {
                if scopes.contains(&&**req) {
                    relevant_scopes.push(req);
                }
            }

            if relevant_scopes.len() == 0 {
                error!("No relevant scopes found");
                return Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::empty())?);
            }

            let id_name = config
                .headerNames
                .get("id")
                .unwrap_or(&String::from("x-id"))
                .clone();
            debug!("Using \"{}\" as the id header name", id_name);
            let group_name = config
                .headerNames
                .get("groups")
                .unwrap_or(&String::from("x-groups"))
                .clone();
            debug!("Using \"{}\" as the group header name", group_name);

            // Create the HTTP response
            let resp = Response::builder()
                .status(StatusCode::OK)
                .header(
                    HeaderName::from_bytes(group_name.as_bytes())?,
                    relevant_scopes.join(","),
                )
                .header(
                    HeaderName::from_bytes(id_name.as_bytes())?,
                    claims["sub"]
                        .as_str()
                        .ok_or(anyhow::anyhow!("no scope found"))?,
                )
                .body(Body::empty())?;
            Ok(resp)
        };

        // Return the response as an immediate future
        Box::pin(fut)
    }
}

fn get_header_value(header: &str, header_map: &HeaderMap<HeaderValue>) -> anyhow::Result<String> {
    let header_value = match header_map.get(header) {
        Some(val) => val,
        _ => {
            return Err(anyhow::anyhow!(
                "Did not find the header field \"{}\"",
                header
            ));
        }
    };

    let header_str = match header_value.to_str() {
        Ok(str) => str,
        Err(_) => {
            return Err(anyhow::anyhow!(
                "Could not convert the value of the header field \"{}\" into a string",
                header
            ));
        }
    };

    Ok(String::from(header_str))
}

struct ServiceFactory {
    sender: mpsc::Sender<oneshot::Sender<anyhow::Result<JWKS>>>,
    configuration: Configuration,
}

impl<T> Service<T> for ServiceFactory {
    type Response = AuthorizationService;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let sender = self.sender.clone();
        let configuration = self.configuration.clone();

        Box::pin(async move {
            Ok(AuthorizationService {
                sender,
                configuration,
            })
        })
    }
}
