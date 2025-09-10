use crate::exports::activity_flyio::fly_http::machine::{
    ExecResponse, Guest, MachineConfig, MachineRegion,
};
use crate::machine::ser::MachineRegionSer;
use crate::{API_BASE_URL, Component, request_with_api_token};
use anyhow::{Context, anyhow, bail, ensure};
use ser::{
    ExecResponseSer, GuestConfigSer, InitConfigSer, MachineConfigSer, MachineCreateRequestSer,
    MachineCreateResponseSer, MachineRestartSer, MachineUpdateRequestSer, ResponseErrorSer,
    StopConfigSer,
};
use wstd::http::request::JsonRequest;
use wstd::http::{Client, IntoBody as _, Method, StatusCode};
use wstd::runtime::block_on;

// These structs are internal implementation details. They are designed to serialize
// into the exact JSON format expected by the Fly.io Machines API.
pub(crate) mod ser {
    use crate::exports::activity_flyio::fly_http::machine::{
        CpuKind, ExecResponse, GuestConfig, InitConfig, MachineConfig, MachineRegion,
        MachineRestart, RestartPolicy, StopConfig,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineCreateRequestSer {
        pub(crate) name: String,
        pub(crate) config: MachineConfigSer,
        pub(crate) region: Option<MachineRegionSer>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineUpdateRequestSer {
        pub(crate) config: MachineConfigSer,
        pub(crate) region: Option<MachineRegionSer>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct MachineConfigSer {
        pub(crate) image: String,
        pub(crate) guest: Option<GuestConfigSer>,
        pub(crate) auto_destroy: Option<bool>,
        pub(crate) init: Option<InitConfigSer>,
        pub(crate) env: Option<HashMap<String, String>>,
        pub(crate) restart: Option<MachineRestartSer>,
        pub(crate) stop_config: Option<StopConfigSer>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct GuestConfigSer {
        pub(crate) cpu_kind: Option<CpuKindSer>,
        pub(crate) cpus: Option<u64>,
        pub(crate) memory_mb: Option<u64>,
        pub(crate) kernel_args: Option<Vec<String>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub(crate) enum CpuKindSer {
        Shared,
        Performance,
    }

    impl From<CpuKind> for CpuKindSer {
        fn from(value: CpuKind) -> Self {
            match value {
                CpuKind::Performance => CpuKindSer::Performance,
                CpuKind::Shared => CpuKindSer::Shared,
            }
        }
    }
    impl From<CpuKindSer> for CpuKind {
        fn from(value: CpuKindSer) -> CpuKind {
            match value {
                CpuKindSer::Performance => CpuKind::Performance,
                CpuKindSer::Shared => CpuKind::Shared,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct InitConfigSer {
        pub(crate) cmd: Option<Vec<String>>,
        pub(crate) entrypoint: Option<Vec<String>>,
        pub(crate) exec: Option<Vec<String>>,
        pub(crate) kernel_args: Option<Vec<String>>,
        pub(crate) swap_size_mb: Option<u64>,
        pub(crate) tty: Option<bool>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct MachineRestartSer {
        pub(crate) max_retries: Option<u32>,
        pub(crate) policy: RestartPolicySer,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub(crate) enum RestartPolicySer {
        No,
        Always,
        // must be serialized as `on-failure`
        OnFailure,
    }
    impl From<RestartPolicy> for RestartPolicySer {
        fn from(value: RestartPolicy) -> RestartPolicySer {
            match value {
                RestartPolicy::No => Self::No,
                RestartPolicy::Always => Self::Always,
                RestartPolicy::OnFailure => Self::OnFailure,
            }
        }
    }
    impl From<RestartPolicySer> for RestartPolicy {
        fn from(value: RestartPolicySer) -> RestartPolicy {
            match value {
                RestartPolicySer::No => RestartPolicy::No,
                RestartPolicySer::Always => RestartPolicy::Always,
                RestartPolicySer::OnFailure => RestartPolicy::OnFailure,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct StopConfigSer {
        pub(crate) signal: Option<String>,
        pub(crate) timeout: Option<u64>,
    }

    #[derive(Deserialize)]
    pub(crate) struct MachineCreateResponseSer {
        pub(crate) id: String,
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct ResponseErrorSer {
        error: String,
    }

    impl ResponseErrorSer {
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
    pub(crate) struct ExecResponseSer {
        exit_code: Option<i32>,
        exit_signal: Option<i32>,
        stderr: Option<String>,
        stdout: Option<String>,
    }
    impl From<ExecResponseSer> for ExecResponse {
        fn from(value: ExecResponseSer) -> Self {
            ExecResponse {
                exit_code: value.exit_code,
                exit_signal: value.exit_signal,
                stderr: value.stderr,
                stdout: value.stdout,
            }
        }
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineRegionSer(String);

    impl From<MachineRegion> for MachineRegionSer {
        fn from(region: MachineRegion) -> MachineRegionSer {
            MachineRegionSer(serde_json::to_string(&region).unwrap().to_lowercase())
        }
    }
    impl From<MachineRegionSer> for MachineRegion {
        fn from(region: MachineRegionSer) -> MachineRegion {
            let capitalized = region.0.to_uppercase();
            serde_json::from_str(&capitalized).unwrap()
        }
    }

    impl From<MachineConfig> for MachineConfigSer {
        fn from(wit: MachineConfig) -> MachineConfigSer {
            // Copy the list of environment variable tuples.
            let env = wit.env.map(|vec| vec.into_iter().collect());

            // Transform the guest config.
            let guest = wit.guest.map(|g| GuestConfigSer {
                cpu_kind: g.cpu_kind.map(|cpu_kind| cpu_kind.into()),
                cpus: g.cpus,
                memory_mb: g.memory_mb,
                kernel_args: g.kernel_args,
            });

            // Transform the restart policy.
            let restart = wit.restart.map(|r| MachineRestartSer {
                max_retries: r.max_retries,
                policy: r.policy.into(),
            });

            // Transform the init config.
            let init = wit.init.map(|i| InitConfigSer {
                cmd: i.cmd,
                entrypoint: i.entrypoint,
                exec: i.exec,
                kernel_args: i.kernel_args,
                swap_size_mb: i.swap_size_mb,
                tty: i.tty,
            });

            // Transform the stop config.
            let stop_config = wit.stop_config.map(|s| StopConfigSer {
                signal: s.signal,
                timeout: s.timeout,
            });

            MachineConfigSer {
                image: wit.image,
                auto_destroy: wit.auto_destroy,
                env,
                guest,
                restart,
                init,
                stop_config,
            }
        }
    }

    impl From<MachineConfigSer> for MachineConfig {
        fn from(ser: MachineConfigSer) -> MachineConfig {
            // Revert the environment variable tuple list back to a Vec<Option<(String, String)>>.
            let env = ser.env.map(|vec| vec.into_iter().collect());

            // Revert the guest config.
            let guest = ser.guest.map(|g| GuestConfig {
                cpu_kind: g.cpu_kind.map(|cpu_kind| cpu_kind.into()),
                cpus: g.cpus,
                memory_mb: g.memory_mb,
                kernel_args: g.kernel_args,
            });

            // Revert the restart policy.
            let restart = ser.restart.map(|r| MachineRestart {
                max_retries: r.max_retries,
                policy: r.policy.into(),
            });

            // Revert the init config.
            let init = ser.init.map(|i| InitConfig {
                cmd: i.cmd,
                entrypoint: i.entrypoint,
                exec: i.exec,
                kernel_args: i.kernel_args,
                swap_size_mb: i.swap_size_mb,
                tty: i.tty,
            });

            // Revert the stop config.
            let stop_config = ser.stop_config.map(|s| StopConfig {
                signal: s.signal,
                timeout: s.timeout,
            });

            MachineConfig {
                image: ser.image,
                auto_destroy: ser.auto_destroy,
                env,
                guest,
                restart,
                init,
                stop_config,
            }
        }
    }
}
}

async fn create(
    app_name: String,
    machine_name: String,
    machine_config: MachineConfig,
    region: Option<MachineRegion>,
) -> Result<String, anyhow::Error> {
    {
        let region = region.map(MachineRegionSer::from);
        let fly_config = MachineConfigSer::from(machine_config);
        let request_payload = MachineCreateRequestSer {
            name: machine_name,
            config: fly_config,
            region,
        };
        let body = serde_json::to_string(&request_payload).expect("must be serializable");

        let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(body.into_body())?;

        let response = Client::new().send(request).await?;
        if response.status().is_success() {
            let body = response.into_body().bytes().await?;
            let resp: MachineCreateResponseSer =
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
            let error: ResponseErrorSer =
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
    region: Option<MachineRegion>,
) -> Result<(), anyhow::Error> {
    {
        let region = region.map(MachineRegionSer::from);
        let machine_config = MachineConfigSer::from(machine_config);
        let request_payload = MachineUpdateRequestSer {
            config: machine_config,
            region,
        };
        let body = serde_json::to_string(&request_payload).expect("must be serializable");

        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(body.into_body())?;

        let response = Client::new().send(request).await?;
        if response.status().is_success() {
            let body = response.into_body().bytes().await?;
            let resp: MachineCreateResponseSer =
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
        let response: ExecResponseSer = serde_json::from_slice(&response).inspect_err(|_| {
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
        region: Option<MachineRegion>,
    ) -> Result<String, String> {
        block_on(create(app_name, machine_name, machine_config, region))
            .map_err(|err| err.to_string())
    }

    fn update(
        app_name: String,
        machine_id: String,
        machine_config: MachineConfig,
        region: Option<MachineRegion>,
    ) -> Result<(), String> {
        block_on(update(app_name, machine_id, machine_config, region))
            .map_err(|err| err.to_string())
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

#[cfg(test)]
mod tests {
    use super::{ser::ResponseErrorSer, transform_region};
    use crate::exports::activity_flyio::fly_http::machine::MachineRegion;
    use serde_json::json;

    #[test]
    fn transform_region_should_print_the_region_in_lower_case() {
        assert_eq!("ams", transform_region(MachineRegion::Ams));
    }

    #[test]
    fn get_machine_id_on_creation_conflict_should_work() {
        let response = json!({"error": "already_exists: unique machine name violation, machine ID 32876249a30918 already exists with name \"foo\""});
        let response: ResponseErrorSer = serde_json::from_value(response).unwrap();
        let id = response.get_machine_id_on_creation_conflict().unwrap();
        assert_eq!("32876249a30918", id);
    }
}
