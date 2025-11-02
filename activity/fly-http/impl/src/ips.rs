use std::collections::HashSet;

use crate::generated::exports::obelisk_flyio::activity_fly_http::ips::{
    self, IpVariant, Ipv4Config, Ipv6Config,
};
use crate::generated::obelisk_flyio::activity_fly_http::regions::Region;
use crate::wstd_util::JsonRequest as _;
use crate::{API_BASE_URL, AppName, request_with_api_token};
use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize};
use wstd::http::{Body, Client, Method, StatusCode};
use wstd::runtime::block_on;

async fn allocate_ip(app_name: &AppName, config: &IpVariant) -> Result<String, anyhow::Error> {
    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum FlyIpType {
        V4,
        V6,
        PrivateV6,
        SharedV4,
    }
    #[derive(Serialize)]
    struct AssignIpBody {
        #[serde(rename = "type")]
        ip_type: FlyIpType,
        #[serde(skip_serializing_if = "Option::is_none")]
        region: Option<Region>,
    }

    let (ip_type, region) = match config {
        ips::IpVariant::Ipv4(Ipv4Config {
            shared: false,
            region,
        }) => (FlyIpType::V4, *region),
        ips::IpVariant::Ipv4(Ipv4Config {
            shared: true,
            region,
        }) => (FlyIpType::SharedV4, *region),
        ips::IpVariant::Ipv6(Ipv6Config { region }) => (FlyIpType::V6, *region),
        ips::IpVariant::Ipv6Private => (FlyIpType::PrivateV6, None),
    };

    let body = AssignIpBody { ip_type, region };

    let request = request_with_api_token()?
        .method(Method::POST)
        .uri(format!("{API_BASE_URL}/apps/{app_name}/ip_assignments"))
        .json(&body)?;

    let response = Client::new().send(request).await?;
    let resp_status = response.status();
    let mut response = response.into_body();
    let response = response.str_contents().await?;

    if resp_status.is_success() {
        #[derive(Deserialize)]
        struct AssignIpResponse {
            ip: String,
        }

        let response: AssignIpResponse = serde_json::from_str(response)
            .inspect_err(|_| eprintln!("cannot deserialize: {response}"))?;
        Ok(response.ip)
    } else {
        Err(anyhow!("failed with status {resp_status}: {response}",))
    }
}

async fn list_ips(app_name: &AppName) -> Result<Vec<ips::IpDetail>, anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(format!("{API_BASE_URL}/apps/{app_name}/ip_assignments"))
        .body(Body::empty())?;

    let mut response = Client::new().send(request).await?;

    if response.status().is_success() {
        #[derive(Deserialize)]
        struct FlyIpDetail {
            ip: String,
            #[serde(deserialize_with = "deserialize_optional_region")]
            region: Option<Region>,
            shared: Option<bool>,
        }

        #[derive(Deserialize)]
        struct ListIpsResponse {
            ips: Vec<FlyIpDetail>,
        }

        let list_response: ListIpsResponse = response.body_mut().json().await?;
        let ip_details: Vec<ips::IpDetail> = list_response
            .ips
            .into_iter()
            .map(|fly_ip| {
                let ip_variant = if fly_ip.ip.contains(':') {
                    // IPv6
                    if fly_ip.ip.starts_with("fdaa") {
                        ips::IpVariant::Ipv6Private
                    } else {
                        ips::IpVariant::Ipv6(ips::Ipv6Config {
                            region: fly_ip.region,
                        })
                    }
                } else {
                    // IPv4
                    ips::IpVariant::Ipv4(ips::Ipv4Config {
                        shared: fly_ip.shared.unwrap_or(false),
                        region: fly_ip.region,
                    })
                };
                ips::IpDetail {
                    ip: fly_ip.ip,
                    ip_variant,
                }
            })
            .collect();
        Ok(ip_details)
    } else {
        let error_status = response.status();
        let mut response = response.into_body();
        let error_body = response.str_contents().await?;
        Err(anyhow!("failed with status {error_status}: {error_body}",))
    }
}

async fn release_ip(app_name: &AppName, ip: &str) -> Result<(), anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::DELETE)
        .uri(format!(
            "{API_BASE_URL}/apps/{app_name}/ip_assignments/{ip}"
        ))
        .body(Body::empty())?;

    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        if error_status == StatusCode::NOT_FOUND {
            // Idempotency: if IP does not exist, return Ok, as this might be a retry.
            return Ok(());
        }
        let mut response = response.into_body();
        let error_body = response.str_contents().await?;
        Err(anyhow!("failed with status {error_status}: {error_body}",))
    }
}

// Custom deserializer for region in the `list` API - "global" shoud be deserialized as None
fn deserialize_optional_region<'de, D>(deserializer: D) -> Result<Option<Region>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(region_str) => {
            if region_str.eq_ignore_ascii_case("global") {
                Ok(None) // Treat "global" as if the region was not set
            } else {
                let lowercased = region_str.to_ascii_lowercase();
                Region::deserialize(serde::de::value::StrDeserializer::new(lowercased.as_str()))
                    .map(Some)
            }
        }
        None => Ok(None),
    }
}

async fn allocate_ip_idempotently(
    app_name: AppName,
    config: ips::IpVariant,
    pre_existing: Vec<ips::IpDetail>,
) -> Result<String, anyhow::Error> {
    let allocated = allocate_ip(&app_name, &config).await?;
    let mut expected: HashSet<_> = pre_existing.into_iter().map(|detail| detail.ip).collect();
    expected.insert(allocated.clone());

    let post_existing: HashSet<_> = list_ips(&app_name)
        .await?
        .into_iter()
        .map(|detail| detail.ip)
        .collect();
    for redundant in post_existing.symmetric_difference(&expected) {
        release_ip(&app_name, redundant).await?;
    }
    Ok(allocated)
}

impl ips::Guest for crate::Component {
    fn allocate(
        app_name: String,
        config: ips::IpVariant,
        pre_existing: Vec<ips::IpDetail>,
    ) -> Result<ips::IpAddress, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(allocate_ip_idempotently(app_name, config, pre_existing))
        })()
        .map_err(|err| err.to_string())
    }

    fn list(app_name: String) -> Result<Vec<ips::IpDetail>, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(async move { list_ips(&app_name).await })
        })()
        .map_err(|err| err.to_string())
    }

    fn release(app_name: String, ip: ips::IpAddress) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(async move { release_ip(&app_name, &ip).await })
        })()
        .map_err(|err| err.to_string())
    }
}
