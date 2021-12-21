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
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{env, error::Error};
use tokio::sync::{mpsc, oneshot};
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
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config_filename = get_env("AUTH0_CONFIG").unwrap_or(String::from("config.json"));
    let file = File::open(config_filename)?;
    let reader = BufReader::new(file);
    let config: Configuration = serde_json::from_reader(reader)?;
    let issuer = config.issuer.clone();
    let address = get_env("AUTH0_BIND_ADDRESS").unwrap_or(String::from("127.0.0.1:8888"));
    let addr = (&*address).parse()?;
    info!("Starting server at {}", address);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<oneshot::Sender<JWKS>>(100);

    tokio::spawn(async move {
        let mut jwks = get_jwks(&*issuer).await.unwrap();
        let mut update_time = Utc::now();

        while let Some(response) = cmd_rx.recv().await {
            let now = Utc::now();

            if now - update_time > chrono::Duration::hours(1) {
                update_time = now;
                jwks = get_jwks(&*issuer).await.unwrap();
            }

            response.send(jwks.clone()).unwrap();
        }
    });

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

fn get_env(key: &str) -> Result<String, AuthServerError> {
    env::var(key).map_err(|e| AuthServerError::from((key, e)))
}

async fn get_jwks(issuer: &str) -> Result<JWKS, Box<dyn Error>> {
    info!("Updating JWKS from {}.well-known/jwks.json", issuer);
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    let mut resp = client
        .get(format!("{}.well-known/jwks.json", issuer).parse()?)
        .await?;

    let mut body_vec: Vec<u8> = vec![];

    while let Some(chunk) = resp.body_mut().data().await {
        body_vec.append(&mut chunk?.to_vec());
    }

    let jwks: JWKS = serde_json::from_str(&*String::from_utf8(body_vec)?)?;

    Ok(jwks)
}

struct AuthorizationService {
    sender: mpsc::Sender<oneshot::Sender<JWKS>>,
    configuration: Configuration,
}

impl Service<Request<Body>> for AuthorizationService {
    type Response = Response<Body>;
    type Error = hyper::Error;
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
            sender.send(resp_tx).await.ok().unwrap();
            let jwks = resp_rx.await.unwrap();
            debug!("Obtained JWKS");

            // get token
            let authorization = match get_header_value("authorization", req.headers()) {
                Ok(val) => val,
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())
                        .unwrap());
                }
            };
            let token = match authorization.split(" ").last() {
                Some(token) => token,
                None => {
                    error!("Could not find token");
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::empty())
                        .unwrap());
                }
            };
            let kid = token_kid(token)
                .expect("Error finding key id")
                .expect("Error decoding key");
            debug!("Decoded token key");
            let jwk = jwks.find(&kid).expect("Key not found in set");
            debug!("Found key");
            let validations = vec![
                Validation::Issuer(config.issuer.to_string()),
                Validation::Audience(config.audience.to_string()),
                Validation::SubjectPresent,
                Validation::NotExpired,
            ];
            let res = validate(token, jwk, validations).expect("Validation failed");
            let claims = res.claims;
            debug!("Found claims: {:?}", claims);
            let scopes: Vec<&str> = claims["scope"]
                .as_str()
                .expect("no scope found")
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
                        .body(Body::empty())
                        .unwrap());
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
                        .body(Body::empty())
                        .unwrap());
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
                        .body(Body::empty())
                        .unwrap());
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
                        .body(Body::empty())
                        .unwrap());
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
                    .body(Body::empty())
                    .unwrap());
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
                    HeaderName::from_bytes(group_name.as_bytes()).unwrap(),
                    relevant_scopes.join(","),
                )
                .header(
                    HeaderName::from_bytes(id_name.as_bytes()).unwrap(),
                    claims["sub"].as_str().expect("no scope found"),
                )
                .body(Body::empty())
                .unwrap();
            Ok(resp)
        };

        // Return the response as an immediate future
        Box::pin(fut)
    }
}

fn get_header_value(header: &str, header_map: &HeaderMap<HeaderValue>) -> Result<String, ()> {
    let header_value = match header_map.get(header) {
        Some(val) => val,
        _ => {
            error!("Did not find the header field \"{}\"", header);
            return Err(());
        }
    };

    let header_str = match header_value.to_str() {
        Ok(str) => str,
        Err(_) => {
            error!(
                "Could not convert the value of the header field \"{}\" into a string",
                header
            );
            return Err(());
        }
    };

    Ok(String::from(header_str))
}

struct ServiceFactory {
    sender: mpsc::Sender<oneshot::Sender<JWKS>>,
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
        let config = self.configuration.clone();
        let fut = async move {
            Ok(AuthorizationService {
                sender: sender,
                configuration: config,
            })
        };
        Box::pin(fut)
    }
}
