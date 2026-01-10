mod wstd_util;

use crate::wstd_util::JsonRequest as _;
use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use wstd::http::body::Body;
use wstd::http::{Client, Error, Request, Response, StatusCode};
use wstd::http::{
    Method,
    request::{self},
};

const API_BASE_URL: &str = "https://api.machines.dev/v1";
const FLY_API_TOKEN: &str = "FLY_API_TOKEN";

fn request_with_api_token() -> Result<request::Builder, anyhow::Error> {
    let api_token = std::env::var(FLY_API_TOKEN).context("cannot obtain `FLY_API_TOKEN`")?;
    Ok(Request::builder().header("Authorization", &format!("Bearer {api_token}")))
}

async fn put_secret(
    app_name: String,
    secret_name: String,
    value: String,
) -> Result<(), anyhow::Error> {
    #[derive(Serialize)]
    struct PutBody {
        value: String,
    }
    let client = Client::new();
    let body = PutBody { value };
    let request = request_with_api_token()?
        .method(Method::POST)
        .uri(format!(
            "{API_BASE_URL}/apps/{app_name}/secrets/{secret_name}"
        ))
        .json(&body)?;

    let response = client.send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        let mut response = response.into_body();
        let error_body = response.str_contents().await?;
        Err(anyhow!(
            "failed to put secret '{secret_name}' for app '{app_name}' with status {error_status}: {error_body}",
        ))
    }
}

#[derive(Deserialize)]
struct Secret {
    app_name: String,
    name: String,
    value: String,
}

#[wstd::http_server]
async fn main(request: Request<Body>) -> Result<Response<Body>, Error> {
    // Must be configured as POST in obelisk.toml
    assert_eq!(Method::POST, request.method());
    let secret: Secret = request.into_body().json().await.unwrap();
    put_secret(secret.app_name, secret.name, secret.value)
        .await
        .unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .map_err(Error::from)
}
