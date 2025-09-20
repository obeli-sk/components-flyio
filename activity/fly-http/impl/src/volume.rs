use crate::generated::exports::obelisk_flyio::activity_fly_http::volumes::{
    Volume, VolumeCreateRequest,
};
use crate::{API_BASE_URL, AppName, Component, VolumeId, request_with_api_token};
use anyhow::{Context, anyhow, bail};
use wstd::http::request::JsonRequest;
use wstd::http::{Client, Method};
use wstd::runtime::block_on;

async fn list(app_name: AppName) -> Result<Vec<Volume>, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(wstd::io::empty())?;
    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let response: Vec<Volume> = serde_json::from_slice(&response_body).inspect_err(|_| {
            eprintln!(
                "cannot deserialize: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;
        Ok(response)
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn create(app_name: AppName, request: VolumeCreateRequest) -> Result<Volume, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes");
    let http_request = request_with_api_token()?
        .method(Method::POST)
        .uri(url)
        .json(&request)?;

    let response = Client::new().send(http_request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let volume: Volume = serde_json::from_slice(&response_body).with_context(|| {
            format!(
                "Deserialization of response failed: `{}`",
                String::from_utf8_lossy(&response_body)
            )
        })?;
        Ok(volume)
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        bail!("{error_status} - {}", String::from_utf8_lossy(&error_body))
    }
}

async fn get(app_name: AppName, volume_id: VolumeId) -> Result<Volume, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/volumes/{volume_id}");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(wstd::io::empty())?;
    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        let response_body = response.into_body().bytes().await?;
        let volume: Volume = serde_json::from_slice(&response_body).inspect_err(|_| {
            eprintln!(
                "cannot deserialize: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;
        Ok(volume)
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn delete(app_name: AppName, volume_id: VolumeId) -> Result<(), anyhow::Error> {
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
    app_name: AppName,
    volume_id: VolumeId,
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
impl crate::generated::exports::obelisk_flyio::activity_fly_http::volumes::Guest for Component {
    fn list(app_name: String) -> Result<Vec<Volume>, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(list(app_name))
        })()
        .map_err(|err| err.to_string())
    }

    fn create(app_name: String, request: VolumeCreateRequest) -> Result<Volume, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(create(app_name, request))
        })()
        .map_err(|err| err.to_string())
    }

    fn get(app_name: String, volume_id: String) -> Result<Volume, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let volume_id = VolumeId::new(volume_id)?;
            block_on(get(app_name, volume_id))
        })()
        .map_err(|err| err.to_string())
    }

    fn delete(app_name: String, volume_id: String) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let volume_id = VolumeId::new(volume_id)?;
            block_on(delete(app_name, volume_id))
        })()
        .map_err(|err| err.to_string())
    }

    fn extend(app_name: String, volume_id: String, new_size_gb: u32) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let volume_id = VolumeId::new(volume_id)?;
            block_on(extend(app_name, volume_id, new_size_gb))
        })()
        .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::generated::exports::obelisk_flyio::activity_fly_http::volumes::Volume;
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
        let volume: Volume = serde_json::from_str(json).unwrap();
        assert_debug_snapshot!(volume)
    }
}
