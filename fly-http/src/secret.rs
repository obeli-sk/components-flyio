use crate::exports::activity_flyio::fly_http::secrets;
use crate::{API_BASE_URL, request_with_api_token};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use wstd::http::{Client, IntoBody as _, Method};
use wstd::runtime::block_on;

async fn list_secrets(app_name: String) -> Result<Vec<secrets::Secret>, anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(format!("{API_BASE_URL}/apps/{app_name}/secrets"))
        .body(wstd::io::empty())?;
    let mut response = Client::new().send(request).await?;
    if response.status().is_success() {
        #[derive(Deserialize)]
        struct ListSecretsResponse {
            secrets: Vec<secrets::Secret>,
        }
        let list_response: ListSecretsResponse = response.body_mut().json().await?;
        Ok(list_response.secrets)
    } else {
        let error_status = response.status();
        let error_body = response.body_mut().bytes().await?;
        Err(anyhow!(
            "failed to list secrets for app '{app_name}' with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn put_secret(
    app_name: String,
    secret_name: String,
    value: String,
) -> Result<secrets::Secret, anyhow::Error> {
    #[derive(Serialize)]
    struct PutBody {
        value: String,
    }
    let client = Client::new();
    let body = PutBody { value };
    let body = serde_json::to_vec(&body)?;
    let request = request_with_api_token()?
        .method(Method::POST)
        .uri(format!(
            "{API_BASE_URL}/apps/{app_name}/secrets/{secret_name}"
        ))
        .header("Content-Type", "application/json")
        .body(body.into_body())?;

    let mut response = client.send(request).await?;

    if response.status().is_success() {
        #[derive(Deserialize)]
        struct SecretResponse {
            name: String,
            digest: String,
        }
        let secret_response: SecretResponse = response.body_mut().json().await?;
        Ok(secrets::Secret {
            name: secret_response.name,
            digest: secret_response.digest,
        })
    } else {
        let error_status = response.status();
        let error_body = response.body_mut().bytes().await?;
        Err(anyhow!(
            "failed to put secret '{secret_name}' for app '{app_name}' with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn delete_secret(app_name: String, secret_name: String) -> Result<(), anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::DELETE)
        .uri(format!(
            "{API_BASE_URL}/apps/{app_name}/secrets/{secret_name}"
        ))
        .body(wstd::io::empty())?;

    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        Err(anyhow!(
            "failed to delete secret '{secret_name}' for app '{app_name}' with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

impl secrets::Guest for crate::Component {
    /// List all secrets for a given app.
    fn list(app_name: String) -> Result<Vec<secrets::Secret>, String> {
        block_on(list_secrets(app_name)).map_err(|err| err.to_string())
    }

    /// Set a secret for a given app.
    fn put(
        app_name: String,
        secret_name: String,
        value: String,
    ) -> Result<secrets::Secret, String> {
        block_on(put_secret(app_name, secret_name, value)).map_err(|err| err.to_string())
    }

    /// Delete a secret from a given app.
    fn delete(app_name: String, secret_name: String) -> Result<(), String> {
        block_on(delete_secret(app_name, secret_name)).map_err(|err| err.to_string())
    }
}
