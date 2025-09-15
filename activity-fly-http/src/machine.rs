use crate::exports::obelisk_flyio::activity_fly_http::machines::{
    ExecResponse, Guest, Machine, MachineConfig,
};
use crate::obelisk_flyio::activity_fly_http::regions::Region;

use crate::machine::ser::{MachineSer, ToLowerWrapper};
use crate::{API_BASE_URL, Component, request_with_api_token};
use anyhow::{Context, anyhow, bail, ensure};
use ser::{
    ExecResponseSer, MachineConfigSer, MachineCreateRequestSer, MachineCreateResponseSer,
    MachineUpdateRequestSer, ResponseErrorSer,
};
use wstd::http::request::JsonRequest;
use wstd::http::{Client, Method, StatusCode};
use wstd::runtime::block_on;

// These structs are internal implementation details. They are designed to serialize
// into the exact JSON format expected by the Fly.io Machines API.
pub(crate) mod ser {
    use crate::exports::obelisk_flyio::activity_fly_http::machines::{
        CpuKind, ExecResponse, GuestConfig, HostStatus, InitConfig, Machine, MachineConfig,
        MachineRestart, Mount, PortConfig, PortHandler, RestartPolicy, ServiceConfig,
        ServiceProtocol, StopConfig,
    };
    use crate::obelisk_flyio::activity_fly_http::regions::Region;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    use std::collections::HashMap;

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineCreateRequestSer {
        pub(crate) name: String,
        pub(crate) config: MachineConfigSer,
        pub(crate) region: Option<ToLowerWrapper<Region>>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineUpdateRequestSer {
        pub(crate) config: MachineConfigSer,
        pub(crate) region: Option<ToLowerWrapper<Region>>,
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct MachineSer {
        config: MachineConfigSer,
        created_at: String,
        updated_at: String,
        id: String,
        instance_id: String,
        name: String,
        state: String,
        region: ToLowerWrapper<Region>,
        host_status: ToLowerWrapper<HostStatus>,
    }
    impl From<MachineSer> for Machine {
        fn from(value: MachineSer) -> Machine {
            Machine {
                config: MachineConfig::from(value.config),
                created_at: value.created_at,
                updated_at: value.updated_at,
                id: value.id,
                instance_id: value.instance_id,
                name: value.name,
                state: value.state,
                region: value.region.0,
                host_status: value.host_status.0,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct MachineConfigSer {
        image: String,
        guest: Option<GuestConfigSer>,
        auto_destroy: Option<bool>,
        init: Option<InitConfigSer>,
        env: Option<HashMap<String, String>>,
        restart: Option<MachineRestartSer>,
        stop_config: Option<StopConfig>,
        mounts: Option<Vec<Mount>>,
        services: Option<Vec<ServiceConfigSer>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct GuestConfigSer {
        cpu_kind: Option<CpuKindWrapper>,
        cpus: Option<u64>,
        memory_mb: Option<u64>,
        kernel_args: Option<Vec<String>>,
    }

    type CpuKindWrapper = ToLowerWrapper<CpuKind>;

    impl From<CpuKindWrapper> for CpuKind {
        fn from(value: CpuKindWrapper) -> CpuKind {
            value.0
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct InitConfigSer {
        cmd: Option<Vec<String>>,
        entrypoint: Option<Vec<String>>,
        exec: Option<Vec<String>>,
        kernel_args: Option<Vec<String>>,
        swap_size_mb: Option<u64>,
        tty: Option<bool>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct MachineRestartSer {
        max_retries: Option<u32>,
        policy: RestartPolicyWrapper,
    }

    type RestartPolicyWrapper = ToLowerWrapper<RestartPolicy>;
    impl From<RestartPolicyWrapper> for RestartPolicy {
        fn from(value: RestartPolicyWrapper) -> RestartPolicy {
            value.0
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct MountSer {
        volume: String,
        path: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct PortConfigSer {
        port: u16,
        handlers: Vec<ToLowerWrapper<PortHandler>>,
    }
    impl From<PortConfig> for PortConfigSer {
        fn from(wit: PortConfig) -> Self {
            PortConfigSer {
                port: wit.port,
                handlers: wit.handlers.into_iter().map(ToLowerWrapper).collect(),
            }
        }
    }
    impl From<PortConfigSer> for PortConfig {
        fn from(ser: PortConfigSer) -> PortConfig {
            PortConfig {
                port: ser.port,
                handlers: ser.handlers.into_iter().map(|wrapper| wrapper.0).collect(),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct ServiceConfigSer {
        internal_port: u16,
        protocol: ToLowerWrapper<ServiceProtocol>,
        ports: Vec<PortConfigSer>,
    }
    impl From<ServiceConfig> for ServiceConfigSer {
        fn from(wit: ServiceConfig) -> Self {
            ServiceConfigSer {
                internal_port: wit.internal_port,
                protocol: ToLowerWrapper(wit.protocol),
                ports: wit.ports.into_iter().map(PortConfigSer::from).collect(),
            }
        }
    }
    impl From<ServiceConfigSer> for ServiceConfig {
        fn from(ser: ServiceConfigSer) -> ServiceConfig {
            ServiceConfig {
                internal_port: ser.internal_port,
                protocol: ser.protocol.0,
                ports: ser.ports.into_iter().map(PortConfig::from).collect(),
            }
        }
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

    use std::fmt::Debug;
    #[derive(derive_more::Debug, derive_more::From)]
    #[debug("{_0:?}")] // Transparent debug
    // FIXME: Remove once wit-bindgen supports path-specific derives
    pub(crate) struct ToLowerWrapper<T: Debug + Serialize + DeserializeOwned>(pub(crate) T);

    impl<T: Debug + Serialize + DeserializeOwned> Serialize for ToLowerWrapper<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let debug_str = format!("{:?}", self.0);
            let expected_type = std::any::type_name::<T>().rsplit("::").next().unwrap();
            let debug_str = debug_str
                .strip_prefix(expected_type)
                .expect("prefix is generated by wit-bindgen")
                .strip_prefix("::")
                .expect(":: delimiter is generated by wit-bindgen");
            let kebab_str = heck::ToKebabCase::to_kebab_case(debug_str);
            serializer.serialize_str(&kebab_str)
        }
    }

    impl<'de, T: Debug + Serialize + DeserializeOwned> Deserialize<'de> for ToLowerWrapper<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_string(RegionVisitor {
                _phantom_data: Default::default(),
            })
        }
    }

    struct RegionVisitor<T: Debug + Serialize + DeserializeOwned> {
        _phantom_data: std::marker::PhantomData<T>,
    }

    impl<'de, T: Debug + Serialize + DeserializeOwned> serde::de::Visitor<'de> for RegionVisitor<T> {
        type Value = ToLowerWrapper<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            let expected_type = std::any::type_name::<T>().rsplit("::").next().unwrap();
            formatter.write_str("a lowercase string representing a ")?;
            formatter.write_str(expected_type)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let camel_cased = heck::AsUpperCamelCase(value.to_owned());

            let serde_value = serde_json::value::Value::String(camel_cased.to_string());
            serde_json::from_value::<T>(serde_value)
                .map(|inner| ToLowerWrapper(inner))
                .map_err(E::custom)
        }
    }

    impl From<MachineConfig> for MachineConfigSer {
        fn from(wit: MachineConfig) -> MachineConfigSer {
            // Copy the list of environment variable tuples.
            let env = wit.env.map(|vec| vec.into_iter().collect());

            // Transform the guest config.
            let guest = wit.guest.map(|g| GuestConfigSer {
                cpu_kind: g.cpu_kind.map(|cpu_kind_wit| cpu_kind_wit.into()),
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

            MachineConfigSer {
                image: wit.image,
                auto_destroy: wit.auto_destroy,
                env,
                guest,
                restart,
                init,
                stop_config: wit.stop_config,
                mounts: wit.mounts,
                services: wit
                    .services
                    .map(|vec| vec.into_iter().map(ServiceConfigSer::from).collect()),
            }
        }
    }

    impl From<MachineConfigSer> for MachineConfig {
        fn from(ser: MachineConfigSer) -> MachineConfig {
            // Revert the environment variable tuple list back to a Vec<Option<(String, String)>>.
            let env = ser.env.map(|vec| vec.into_iter().collect());

            // Revert the guest config.
            let guest = ser.guest.map(|g| GuestConfig {
                cpu_kind: g.cpu_kind.map(|cpu_kind_ser| cpu_kind_ser.into()),
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
                mounts: ser.mounts,
                services: ser
                    .services
                    .map(|vec| vec.into_iter().map(ServiceConfig::from).collect()),
            }
        }
    }
}

async fn list(app_name: String) -> Result<Vec<Machine>, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(wstd::io::empty())?;
    let response = Client::new().send(request).await?;
    if response.status().is_success() {
        let response = response.into_body().bytes().await?;
        let response: Vec<MachineSer> = serde_json::from_slice(&response).inspect_err(|_| {
            eprintln!("cannot deserialize: {}", String::from_utf8_lossy(&response))
        })?;
        Ok(response.into_iter().map(Machine::from).collect())
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

async fn create(
    app_name: String,
    machine_name: String,
    machine_config: MachineConfig,
    region: Option<Region>,
) -> Result<String, anyhow::Error> {
    {
        let region = region.map(ToLowerWrapper);
        let fly_config = MachineConfigSer::from(machine_config);
        let request_payload = MachineCreateRequestSer {
            name: machine_name,
            config: fly_config,
            region,
        };
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .json(&request_payload)?;

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
    region: Option<Region>,
) -> Result<(), anyhow::Error> {
    {
        let region = region.map(ToLowerWrapper);
        let machine_config = MachineConfigSer::from(machine_config);
        let request_payload = MachineUpdateRequestSer {
            config: machine_config,
            region,
        };
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .json(&request_payload)?;

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
    app_name: String,
    machine_id: String,
    url_suffix: &'static str,
) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}/{url_suffix}");
    send_request(url, Method::POST).await
}

async fn send_request(url: String, method: Method) -> Result<(), anyhow::Error> {
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
    fn list(app_name: String) -> Result<Vec<Machine>, String> {
        block_on(list(app_name)).map_err(|err| err.to_string())
    }

    fn create(
        app_name: String,
        machine_name: String,
        machine_config: MachineConfig,
        region: Option<Region>,
    ) -> Result<String, String> {
        block_on(create(app_name, machine_name, machine_config, region))
            .map_err(|err| err.to_string())
    }

    fn update(
        app_name: String,
        machine_id: String,
        machine_config: MachineConfig,
        region: Option<Region>,
    ) -> Result<(), String> {
        block_on(update(app_name, machine_id, machine_config, region))
            .map_err(|err| err.to_string())
    }

    fn stop(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(app_name, machine_id, "stop")).map_err(|err| err.to_string())
    }

    fn suspend(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(app_name, machine_id, "suspend")).map_err(|err| err.to_string())
    }

    fn start(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(app_name, machine_id, "start")).map_err(|err| err.to_string())
    }

    fn restart(app_name: String, machine_id: String) -> Result<(), String> {
        block_on(change_machine(app_name, machine_id, "restart")).map_err(|err| err.to_string())
    }

    fn delete(app_name: String, machine_id: String, force: bool) -> Result<(), String> {
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}?force={force}");
        block_on(send_request(url, Method::DELETE)).map_err(|err| err.to_string())
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
    use super::ser::ResponseErrorSer;
    use crate::{
        exports::obelisk_flyio::activity_fly_http::machines::{Machine, Region},
        machine::ser::{MachineSer, ToLowerWrapper},
    };
    use insta::assert_debug_snapshot;
    use serde_json::json;

    #[test]
    fn region_ser() {
        assert_eq!(
            "\"ams\"",
            serde_json::to_string(&ToLowerWrapper(Region::Ams)).unwrap()
        );
    }

    #[test]
    fn region_de() {
        assert_matches::assert_matches!(
            serde_json::from_str("\"ams\"").unwrap(),
            ToLowerWrapper(Region::Ams)
        );
    }

    #[test]
    fn get_machine_id_on_creation_conflict_should_work() {
        let response = json!({"error": "already_exists: unique machine name violation, machine ID 32876249a30918 already exists with name \"foo\""});
        let response: ResponseErrorSer = serde_json::from_value(response).unwrap();
        let id = response.get_machine_id_on_creation_conflict().unwrap();
        assert_eq!("32876249a30918", id);
    }

    #[test]
    fn machine_deserialization() {
        let json = r#"
        {
            "id": "080155df097248",
            "name": "machine",
            "state": "started",
            "region": "ams",
            "instance_id": "01K4SR42ZPDHHCN70QNZKVPK48",
            "private_ip": "fdaa:0:fcc8:a7b:32c:3a59:29d5:2",
            "config": {
              "init": {
                "swap_size_mb": 256
              },
              "guest": {
                "cpu_kind": "shared",
                "cpus": 1,
                "memory_mb": 256
              },
              "image": "getobelisk/obelisk:0.24.1-ubuntu",
              "restart": {
                "policy": "on-failure"
              }
            },
            "incomplete_config": null,
            "image_ref": {
              "registry": "docker-hub-mirror.fly.io",
              "repository": "getobelisk/obelisk",
              "tag": "0.24.1-ubuntu",
              "digest": "sha256:041f936be0d2494aca338e43efe052ee087c1e2520385c6f4640efa9e92ab06a",
              "labels": {
                "org.opencontainers.image.ref.name": "ubuntu",
                "org.opencontainers.image.version": "24.04"
              }
            },
            "created_at": "2025-09-10T12:03:04Z",
            "updated_at": "2025-09-10T12:03:07Z",
            "events": [
              {
                "id": "01K4SR45V7PBDQ7HBHEAJ6C9YA",
                "type": "start",
                "status": "started",
                "request": {},
                "source": "flyd",
                "timestamp": 1757505787751
              },
              {
                "id": "01K4SR432JJXA85KC2RB63ANTA",
                "type": "launch",
                "status": "created",
                "source": "user",
                "timestamp": 1757505784914
              }
            ],
            "host_status": "ok"
          }
        "#;
        let machine: MachineSer = serde_json::from_str(json).unwrap();
        let machine = Machine::from(machine);
        assert_debug_snapshot!(machine)
    }
}
