mod app;
mod machine;

use anyhow::Context;
use wit_bindgen::generate;
use wstd::http::{Request, request};

const API_BASE_URL: &str = "https://api.machines.dev/v1";
const FLY_API_TOKEN: &str = "FLY_API_TOKEN";

generate!({ generate_all, additional_derives: [serde::Deserialize] });
struct Component;
export!(Component);

fn request_with_api_token() -> Result<request::Builder, anyhow::Error> {
    let api_token = std::env::var(FLY_API_TOKEN).context("cannot obtain `FLY_API_TOKEN`")?;
    Ok(Request::builder().header("Authorization", &format!("Bearer {api_token}")))
}
