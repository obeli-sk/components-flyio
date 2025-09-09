use crate::exports::activity_flyio::fly_http::machine::{
    ExecResponse, Guest, MachineConfig, MachineRegion,
};
use crate::{API_BASE_URL, Component, request_with_api_token};
use anyhow::{Context, anyhow, bail, ensure};
use ser::{
    FlyExecResponse, FlyGuestConfig, FlyInitConfig, FlyMachineConfig, FlyMachineCreateRequest,
    FlyMachineCreateResponse, FlyMachineRestart, FlyMachineUpdateRequest, FlyResponseError,
    FlyStopConfig,
};
use wstd::http::request::JsonRequest;
use wstd::http::{Client, IntoBody as _, Method, StatusCode};
use wstd::runtime::block_on;

// These structs are internal implementation details. They are designed to serialize
// into the exact JSON format expected by the Fly.io Machines API.
pub(crate) mod ser {
    use crate::exports::activity_flyio::fly_http::machine::{CpuKind, ExecResponse, RestartPolicy};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyMachineCreateRequest {
        pub(crate) name: String,
        pub(crate) config: FlyMachineConfig,
        pub(crate) region: Option<String>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyMachineUpdateRequest {
        pub(crate) config: FlyMachineConfig,
        pub(crate) region: Option<String>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyMachineConfig {
        pub(crate) image: String,
        pub(crate) guest: Option<FlyGuestConfig>,
        pub(crate) auto_destroy: Option<bool>,
        pub(crate) init: Option<FlyInitConfig>,
        pub(crate) env: Option<HashMap<String, String>>,
        pub(crate) restart: Option<FlyMachineRestart>,
        pub(crate) stop_config: Option<FlyStopConfig>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyGuestConfig {
        pub(crate) cpu_kind: Option<FlyCpuKind>,
        pub(crate) cpus: Option<u64>,
        pub(crate) memory_mb: Option<u64>,
        pub(crate) kernel_args: Option<Vec<String>>,
    }

    #[derive(Serialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub(crate) enum FlyCpuKind {
        Shared,
        Performance,
    }

    impl From<CpuKind> for FlyCpuKind {
        fn from(value: CpuKind) -> Self {
            match value {
                CpuKind::Performance => FlyCpuKind::Performance,
                CpuKind::Shared => FlyCpuKind::Shared,
            }
        }
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyInitConfig {
        pub(crate) cmd: Option<Vec<String>>,
        pub(crate) entrypoint: Option<Vec<String>>,
        pub(crate) exec: Option<Vec<String>>,
        pub(crate) kernel_args: Option<Vec<String>>,
        pub(crate) swap_size_mb: Option<u64>,
        pub(crate) tty: Option<bool>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyMachineRestart {
        pub(crate) max_retries: Option<u32>,
        pub(crate) policy: FlyRestartPolicy,
    }

    #[derive(Serialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub(crate) enum FlyRestartPolicy {
        No,
        Always,
        // must be serialized as `on-failure`
        OnFailure,
    }
    impl From<RestartPolicy> for FlyRestartPolicy {
        fn from(value: RestartPolicy) -> Self {
            match value {
                RestartPolicy::No => Self::No,
                RestartPolicy::Always => Self::Always,
                RestartPolicy::OnFailure => Self::OnFailure,
            }
        }
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct FlyStopConfig {
        pub(crate) signal: Option<String>,
        pub(crate) timeout: Option<u64>,
    }

    #[derive(Deserialize)]
    pub(crate) struct FlyMachineCreateResponse {
        pub(crate) id: String,
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct FlyResponseError {
        error: String,
    }

    impl FlyResponseError {
        pub(crate) fn get_machine_id_on_creation_conflict(&self) -> Option<&str> {
            const PREFIX: &str = "already_exists: unique machine name violation, machine ID ";
            const SUFFIX: &str = " already exists with name ";
            if let Some(0) = self.error.find(PREFIX) {
                let start_idx = PREFIX.len();
                if let Some(end_idx) = self.error[start_idx..].find(SUFFIX) {
                    return Some(&self.error[start_idx..start_idx + end_idx]);
                }
            }
            None
        }
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct FlyExecResponse {
        exit_code: Option<i32>,
        exit_signal: Option<i32>,
        stderr: Option<String>,
        stdout: Option<String>,
    }
    impl From<FlyExecResponse> for ExecResponse {
        fn from(value: FlyExecResponse) -> Self {
            ExecResponse {
                exit_code: value.exit_code,
                exit_signal: value.exit_signal,
                stderr: value.stderr,
                stdout: value.stdout,
            }
        }
    }
}

async fn create(
    app_name: String,
    machine_name: String,
    machine_config: MachineConfig,
) -> Result<String, anyhow::Error> {
    {
        let region = machine_config.region.map(transform_region);
        let fly_config = transform_config(machine_config);
        let request_payload = FlyMachineCreateRequest {
            name: machine_name,
            config: fly_config,
            region,
        };
        let body = serde_json::to_string(&request_payload).expect("must be serializable");

        println!("Sending {body}");

        let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(body.into_body())?;

        let response = Client::new().send(request).await?;
        if response.status().is_success() {
            let body = response.into_body().bytes().await?;
            let resp: FlyMachineCreateResponse =
                serde_json::from_slice(&body).with_context(|| {
                    format!(
                        "Deserialization of response failed: `{}`",
                        String::from_utf8_lossy(&body)
                    )
                })?;
            return Ok(resp.id);
        }
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        eprintln!("Got error status {error_status}");
        if error_status == StatusCode::CONFLICT {
            let error: FlyResponseError =
                serde_json::from_slice(&error_body).with_context(|| {
                    format!(
                        "cannot parse error response: `{}`",
                        String::from_utf8_lossy(&error_body)
                    )
                })?;
            let machine_id = error.get_machine_id_on_creation_conflict().with_context(
                || "machine id cannot be parsed from 409 error response: `{error:?}`",
            )?;
            Ok(machine_id.to_string())
        } else {
            Err(anyhow!(
                "{error_status} - {}",
                String::from_utf8_lossy(&error_body)
            ))
        }
    }
}

async fn update(
    app_name: String,
    machine_id: String,
    machine_config: MachineConfig,
) -> Result<(), anyhow::Error> {
    {
        let region = machine_config.region.map(transform_region);
        let fly_config = transform_config(machine_config);
        let request_payload = FlyMachineUpdateRequest {
            config: fly_config,
            region,
        };
        let body = serde_json::to_string(&request_payload).expect("must be serializable");

        println!("Sending {body}");

        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(body.into_body())?;

        let response = Client::new().send(request).await?;
        if response.status().is_success() {
            let body = response.into_body().bytes().await?;
            let resp: FlyMachineCreateResponse =
                serde_json::from_slice(&body).with_context(|| {
                    format!(
                        "Deserialization of response failed: `{}`",
                        String::from_utf8_lossy(&body)
                    )
                })?;
            ensure!(
                resp.id == machine_id,
                "unexpected id returned, expected {machine_id} got {id}",
                id = resp.id
            );
            return Ok(());
        }
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        bail!("{error_status} - {}", String::from_utf8_lossy(&error_body))
    }
}

async fn exec(
    app_name: String,
    machine_id: String,
    command: Vec<String>,
) -> Result<ExecResponse, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}/exec");
    let body = serde_json::json!({
        "command": command,
    });
    let request = request_with_api_token()?
        .method(Method::POST)
        .uri(url)
        .json(&body)?;
    let response = Client::new().send(request).await?;
    if response.status().is_success() {
        let response = response.into_body().bytes().await?;
        let response: FlyExecResponse = serde_json::from_slice(&response).inspect_err(|_| {
            eprintln!("cannot deserialize: {}", String::from_utf8_lossy(&response))
        })?;
        Ok(response.into())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        eprintln!("Got error status {error_status}");
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

async fn change_machine(
    app_name: &str,
    machine_id: &str,
    url_suffix: &str,
) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}/{url_suffix}");
    send_request(&url, Method::POST).await
}

async fn send_request(url: &str, method: Method) -> Result<(), anyhow::Error> {
    let request = request_with_api_token()?
        .method(method)
        .uri(url)
        .body(wstd::io::empty())?;

    let response = Client::new().send(request).await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_status = response.status();
        let error_body = response.into_body().bytes().await?;
        eprintln!("Got error status {error_status}");
        Err(anyhow!(
            "failed with status {error_status}: {}",
            String::from_utf8_lossy(&error_body)
        ))
    }
}

// Implementation of the vm interface for the component.
impl Guest for Component {
    fn create(
        app_name: String,
        machine_name: String,
        machine_config: MachineConfig,
    ) -> Result<String, String> {
        block_on(create(app_name, machine_name, machine_config)).map_err(|err| err.to_string())
    }

    fn update(
        app_name: String,
        machine_id: String,
        machine_config: MachineConfig,
    ) -> Result<(), String> {
        block_on(update(app_name, machine_id, machine_config)).map_err(|err| err.to_string())
    }

    fn stop(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(&app_name, &machine_id, "stop")).map_err(|err| err.to_string())
    }

    fn suspend(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(&app_name, &machine_id, "suspend")).map_err(|err| err.to_string())
    }

    fn start(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(&app_name, &machine_id, "start")).map_err(|err| err.to_string())
    }

    fn delete(app_name: String, machine_id: String, force: bool) -> Result<(), String> {
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}?force={force}");
        block_on(send_request(&url, Method::DELETE)).map_err(|err| err.to_string())
    }

    fn exec(
        app_name: String,
        machine_id: String,
        command: Vec<String>,
    ) -> Result<ExecResponse, String> {
        block_on(exec(app_name, machine_id, command)).map_err(|err| err.to_string())
    }
}

fn transform_region(region: MachineRegion) -> String {
    format!("{region:?}")
        .strip_prefix("MachineRegion::")
        .expect("must start with `MachineRegion::`")
        .to_lowercase()
}

/// A helper function to transform the WIT-generated MachineConfig struct into a
/// serializable FlyMachineConfig struct that matches the Fly.io API.
fn transform_config(wit: MachineConfig) -> FlyMachineConfig {
    // Copy the list of environment variable tuples.
    let env = wit.env.map(|vec| vec.into_iter().collect());

    // Transform the guest config.
    let guest = wit.guest.map(|g| FlyGuestConfig {
        cpu_kind: g.cpu_kind.map(|cpu_kind| cpu_kind.into()),
        cpus: g.cpus,
        memory_mb: g.memory_mb,
        kernel_args: g.kernel_args,
    });

    // Transform the restart policy.
    let restart = wit.restart.map(|r| FlyMachineRestart {
        max_retries: r.max_retries,
        policy: r.policy.into(),
    });

    // Transform the init config.
    let init = wit.init.map(|i| FlyInitConfig {
        cmd: i.cmd,
        entrypoint: i.entrypoint,
        exec: i.exec,
        kernel_args: i.kernel_args,
        swap_size_mb: i.swap_size_mb,
        tty: i.tty,
    });

    // Transform the stop config.
    let stop_config = wit.stop_config.map(|s| FlyStopConfig {
        signal: s.signal,
        timeout: s.timeout,
    });

    FlyMachineConfig {
        image: wit.image,
        auto_destroy: wit.auto_destroy,
        env,
        guest,
        restart,
        init,
        stop_config,
    }
}

#[cfg(test)]
mod tests {
    use super::{ser::FlyResponseError, transform_region};
    use crate::exports::activity_flyio::fly_http::machine::MachineRegion;
    use serde_json::json;

    #[test]
    fn transform_region_should_print_the_region_in_lower_case() {
        assert_eq!("ams", transform_region(MachineRegion::Ams));
    }

    #[test]
    fn get_machine_id_on_creation_conflict_should_work() {
        let response = json!({"error": "already_exists: unique machine name violation, machine ID 32876249a30918 already exists with name \"foo\""});
        let response: FlyResponseError = serde_json::from_value(response).unwrap();
        let id = response.get_machine_id_on_creation_conflict().unwrap();
        assert_eq!("32876249a30918", id);
    }
}
