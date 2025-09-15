use crate::exports::obelisk_flyio::activity_fly_http::volumes::{Volume, VolumeCreateRequest};
use crate::machine::ser::ToLowerWrapper;
use crate::{API_BASE_URL, Component, request_with_api_token};
use anyhow::{Context, anyhow, bail};
use ser::{VolumeCreateRequestSer, VolumeSer};
use wstd::http::request::JsonRequest;
use wstd::http::{Client, Method};
use wstd::runtime::block_on;

// These structs are internal implementation details. They are designed to serialize
// into the exact JSON format expected by the Fly.io Volumes API.
pub(crate) mod ser {
    use crate::machine::ser::ToLowerWrapper;
    use crate::{
        exports::obelisk_flyio::activity_fly_http::volumes::Volume,
        obelisk_flyio::activity_fly_http::regions::Region,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Debug)]
    pub(crate) struct VolumeCreateRequestSer {
        pub(crate) name: String,
        pub(crate) size_gb: u32,
        pub(crate) region: ToLowerWrapper<Region>,
        #[serde(rename = "require_unique_zone")]
        pub(crate) require_unique_zone: Option<bool>,
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct VolumeSer {
        pub(crate) id: String,
        pub(crate) name: String,
        pub(crate) state: String,
        pub(crate) region: ToLowerWrapper<Region>,
        pub(crate) size_gb: u32,
        pub(crate) encrypted: bool,
        pub(crate) attached_machine_id: Option<String>,
        pub(crate) host_status: String,
        pub(crate) created_at: String,
        pub(crate) blocks: u32,
        pub(crate) block_size: u32,
        pub(crate) blocks_free: u32,
        pub(crate) blocks_avail: u32,
        pub(crate) bytes_used: u32,
        pub(crate) bytes_total: u32,
    }

    impl From<VolumeSer> for Volume {
        fn from(value: VolumeSer) -> Volume {
            Volume {
                id: value.id,
                name: value.name,
                state: value.state,
                region: value.region.0,
                size_gb: value.size_gb,
                encrypted: value.encrypted,
                attached_machine_id: value.attached_machine_id,
                host_status: value.host_status,
                created_at: value.created_at,
                blocks: value.blocks,
                block_size: value.block_size,
                blocks_free: value.blocks_free,
                blocks_avail: value.blocks_avail,
                bytes_used: value.bytes_used,
                bytes_total: value.bytes_total,
            }
        }
    }
}

async fn list(app_name: String) -> Result<Vec<Volume>, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(wstd::io::empty())?;
    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let response_ser: Vec<VolumeSer> =
            serde_json::from_slice(&response_body).inspect_err(|_| {
                eprintln!(
                    "cannot deserialize: {}",
                    String::from_utf8_lossy(&response_body)
                )
            })?;
        Ok(response_ser.into_iter().map(Volume::from).collect())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn create(app_name: String, request: VolumeCreateRequest) -> Result<Volume, anyhow::Error> {
    let fly_request = VolumeCreateRequestSer {
        name: request.name,
        size_gb: request.size_gb,
        region: ToLowerWrapper(request.region),
        require_unique_zone: request.require_unique_zone,
    };
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes");
    let http_request = request_with_api_token()?
        .method(Method::POST)
        .uri(url)
        .json(&fly_request)?;

    let response = Client::new().send(http_request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let volume_ser: VolumeSer = serde_json::from_slice(&response_body).with_context(|| {
            format!(
                "Deserialization of response failed: `{}`",
                String::from_utf8_lossy(&response_body)
            )
        })?;
        Ok(Volume::from(volume_ser))
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        bail!("{error_status} - {}", String::from_utf8_lossy(&error_body))
    }
}

async fn get(app_name: String, volume_id: String) -> Result<Volume, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes/{volume_id}");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(wstd::io::empty())?;
    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let volume_ser: VolumeSer = serde_json::from_slice(&response_body).inspect_err(|_| {
            eprintln!(
                "cannot deserialize: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;
        Ok(Volume::from(volume_ser))
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn delete(app_name: String, volume_id: String) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes/{volume_id}");
    let request = request_with_api_token()?
        .method(Method::DELETE)
        .uri(url)
        .body(wstd::io::empty())?;

    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn extend(
    app_name: String,
    volume_id: String,
    new_size_gb: u32,
) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes/{volume_id}/extend");
    let body = serde_json::json!({
        "size_gb": new_size_gb,
    });
    let request = request_with_api_token()?
        .method(Method::PUT)
        .uri(url)
        .json(&body)?;

    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

// Implementation of the volumes interface for the component.
impl crate::exports::obelisk_flyio::activity_fly_http::volumes::Guest for Component {
    fn list(app_name: String) -> Result<Vec<Volume>, String> {
        block_on(list(app_name)).map_err(|err| err.to_string())
    }

    fn create(app_name: String, request: VolumeCreateRequest) -> Result<Volume, String> {
        block_on(create(app_name, request)).map_err(|err| err.to_string())
    }

    fn get(app_name: String, volume_id: String) -> Result<Volume, String> {
        block_on(get(app_name, volume_id)).map_err(|err| err.to_string())
    }

    fn delete(app_name: String, volume_id: String) -> Result<(), String> {
        block_on(delete(app_name, volume_id)).map_err(|err| err.to_string())
    }

    fn extend(app_name: String, volume_id: String, new_size_gb: u32) -> Result<(), String> {
        block_on(extend(app_name, volume_id, new_size_gb)).map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::exports::obelisk_flyio::activity_fly_http::volumes::Volume;

    use super::ser::VolumeSer;
    use insta::assert_debug_snapshot;

    #[test]
    fn volume_deserialization() {
        let json = r#"
        {
            "id": "vol_vjeylkgg6gll7j94",
            "name": "my_app_vol",
            "state": "created",
            "size_gb": 1,
            "region": "ams",
            "zone": "119a",
            "encrypted": true,
            "attached_machine_id": null,
            "attached_alloc_id": null,
            "created_at": "2025-09-13T09:27:18.803Z",
            "blocks": 0,
            "block_size": 0,
            "blocks_free": 0,
            "blocks_avail": 0,
            "bytes_used": 0,
            "bytes_total": 0,
            "fstype": "ext4",
            "snapshot_retention": 5,
            "auto_backup_enabled": true,
            "host_status": "ok",
            "host_dedication_key": ""
        }
        "#;
        let volume: VolumeSer = serde_json::from_str(json).unwrap();
        let volume = Volume::from(volume);
        assert_debug_snapshot!(volume)
    }
}
