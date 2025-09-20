mod app;
mod machine;
mod secret;
mod serde;
mod volume;

use std::marker::PhantomData;

use anyhow::{Context, bail};
use wit_bindgen::generate;
use wstd::http::{Request, request};

const API_BASE_URL: &str = "https://api.machines.dev/v1";
const FLY_API_TOKEN: &str = "FLY_API_TOKEN";

generate!({ generate_all, additional_derives: [serde::Deserialize, serde::Serialize] });
struct Component;
export!(Component);

fn request_with_api_token() -> Result<request::Builder, anyhow::Error> {
    let api_token = std::env::var(FLY_API_TOKEN).context("cannot obtain `FLY_API_TOKEN`")?;
    Ok(Request::builder().header("Authorization", &format!("Bearer {api_token}")))
}

#[derive(derive_more::Display)]
#[display("{value}")]
struct SafeUrlPart<T> {
    value: String,
    _phantom_data: PhantomData<T>,
}
impl<T> SafeUrlPart<T> {
    fn new(s: String) -> Result<SafeUrlPart<T>, anyhow::Error> {
        if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            Ok(SafeUrlPart {
                value: s,
                _phantom_data: PhantomData::default(),
            })
        } else {
            bail!("illegal slug")
        }
    }
}
impl<T> AsRef<str> for SafeUrlPart<T> {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

struct AppMarker;
type AppName = SafeUrlPart<AppMarker>;
struct OrgMarker;
type OrgSlug = SafeUrlPart<OrgMarker>;
struct SecretKeyMarker;
type SecretKey = SafeUrlPart<SecretKeyMarker>;
struct VolumeIdMarker;
type VolumeId = SafeUrlPart<VolumeIdMarker>;
struct MachineIdMarker;
type MachineId = SafeUrlPart<MachineIdMarker>;
