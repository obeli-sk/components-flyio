use crate::exports::obelisk_flyio::activity_fly_http::apps;
use crate::{API_BASE_URL, AppName, OrgSlug, request_with_api_token};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use wstd::http::request::JsonRequest as _;
use wstd::http::{Client, Method, StatusCode};
use wstd::runtime::block_on;

async fn get(app_name: AppName) -> Result<Option<apps::App>, anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(format!("{API_BASE_URL}/apps/{app_name}"))
        .body(wstd::io::empty())?;
    let mut response = Client::new().send(request).await?;

    if response.status().is_success() {
        let app: apps::App = response.body_mut().json().await?;
        Ok(Some(app))
    } else if response.status() == StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        let error_status = response.status();
        let error_body = response.body_mut().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn put(org_slug: OrgSlug, app_name: AppName) -> Result<apps::App, anyhow::Error> {
    let client = Client::new();

    // Attempt to create the app
    #[derive(Serialize)]
    struct CreateAppRequest<'a> {
        app_name: &'a str,
        org_slug: &'a str,
    }

    let request_body = CreateAppRequest {
        app_name: app_name.as_ref(),
        org_slug: org_slug.as_ref(),
    };

    let post_request = request_with_api_token()?
        .method(Method::POST)
        .uri(format!("{API_BASE_URL}/apps"))
        .json(&request_body)?;

    let mut response = client.send(post_request).await?;

    if response.status().is_success() {
        #[derive(Deserialize)]
        struct AppResponse {
            id: String,
        }
        let app_response: AppResponse = response.body_mut().json().await?;
        return Ok(apps::App {
            name: app_name.to_string(),
            id: app_response.id,
        });
    }

    // Investigate if the app already exists
    let original_post_status = response.status();
    let original_post_error = response.into_body().bytes().await;

    if original_post_status == StatusCode::UNPROCESSABLE_ENTITY {
        // Prepare a GET request to check for the existing app.
        let get_request = request_with_api_token()?
            .method(Method::GET)
            .uri(format!("{API_BASE_URL}/apps/{app_name}"))
            .body(wstd::io::empty())?;

        let mut get_response = client.send(get_request).await?;

        if get_response.status().is_success() {
            // The app exists. Now, deserialize the response and check the org slug.
            #[derive(Deserialize)]
            struct OrgDetails {
                slug: String,
            }
            #[derive(Deserialize)]
            struct AppDetails {
                id: String,
                name: String,
                organization: OrgDetails,
            }

            let app_details: AppDetails = get_response.body_mut().json().await?;

            // Verify the organization slug matches
            if app_details.organization.slug == org_slug.as_ref() {
                // Idempotency success: App exists and is in the correct org.
                return Ok(apps::App {
                    id: app_details.id,
                    name: app_details.name,
                });
            } else {
                // Error: App name is taken by a different organization.
                return Err(anyhow!(
                    "app '{app_name}' already exists but belongs to organization '{}', not the requested '{org_slug}'.",
                    app_details.organization.slug,
                ));
            }
        }
    }
    // The GET request failed, so the app doesn't exist.
    // The original error from the POST request is the true cause of failure.
    Err(anyhow!(
        "failed with status {original_post_status}: {}",
        original_post_error
            .map(|vec| String::from_utf8_lossy(&vec).to_string())
            .unwrap_or_else(|e| e.to_string())
    ))
}

async fn list(org_slug: OrgSlug) -> Result<Vec<apps::App>, anyhow::Error> {
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(format!("{API_BASE_URL}/apps?org_slug={org_slug}"))
        .body(wstd::io::empty())?;
    let mut response = Client::new().send(request).await?;

    if response.status().is_success() {
        #[derive(Deserialize)]
        struct AppsResponse {
            apps: Vec<apps::App>,
        }
        let apps_response: AppsResponse = response.body_mut().json().await?;
        Ok(apps_response.apps)
    } else {
        let error_status = response.status();
        let error_body = response.body_mut().bytes().await?;
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn delete(app_name: AppName, force: bool) -> Result<(), anyhow::Error> {
    let mut url = format!("{API_BASE_URL}/apps/{app_name}");
    if force {
        url.push_str("?force=true");
    }
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

impl apps::Guest for crate::Component {
    fn get(app_name: String) -> Result<Option<apps::App>, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(get(app_name))
        })()
        .map_err(|err| err.to_string())
    }

    fn put(org_slug: String, app_name: String) -> Result<apps::App, String> {
        (|| {
            let org_slug = OrgSlug::new(org_slug)?;
            let app_name = AppName::new(app_name)?;
            block_on(put(org_slug, app_name))
        })()
        .map_err(|err| err.to_string())
    }

    fn list(org_slug: String) -> Result<Vec<apps::App>, String> {
        (|| {
            let org_slug = OrgSlug::new(org_slug)?;
            block_on(list(org_slug))
        })()
        .map_err(|err| err.to_string())
    }

    fn delete(app_name: String, force: bool) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(delete(app_name, force))
        })()
        .map_err(|err| err.to_string())
    }
}
