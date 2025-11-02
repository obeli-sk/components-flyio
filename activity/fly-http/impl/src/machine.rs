use crate::generated::exports::obelisk_flyio::activity_fly_http::machines::{
    ExecResponse, Guest, Machine, MachineConfig,
};
use crate::generated::obelisk_flyio::activity_fly_http::regions::Region;
use crate::{API_BASE_URL, AppName, Component, MachineId, request_with_api_token};
use anyhow::{Context, anyhow, bail, ensure};
use ser::{
    ExecResponseSer, MachineCreateRequestSer, MachineCreateResponseSer, MachineUpdateRequestSer,
    ResponseErrorSer,
};
use wstd::http::{Body, Client, Method, StatusCode};
use wstd::runtime::block_on;

pub(crate) mod ser {
    use crate::generated::exports::obelisk_flyio::activity_fly_http::machines::{
        ExecResponse, MachineConfig,
    };
    use crate::generated::obelisk_flyio::activity_fly_http::regions::Region;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineCreateRequestSer {
        pub(crate) name: String,
        pub(crate) config: MachineConfig,
        pub(crate) region: Option<Region>,
    }

    #[derive(Serialize, Debug)]
    pub(crate) struct MachineUpdateRequestSer {
        pub(crate) config: MachineConfig,
        pub(crate) region: Option<Region>,
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
}

async fn list(app_name: AppName) -> Result<Vec<Machine>, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(Body::empty())?;
    let response = Client::new().send(request).await?;
    let resp_status = response.status();
    let mut response = response.into_body();
    let response = response.str_contents().await?;

    if resp_status.is_success() {
        let response: Vec<Machine> = serde_json::from_str(response)
            .inspect_err(|_| eprintln!("cannot deserialize: {response}"))?;
        Ok(response)
    } else {
        eprintln!("Got error status {resp_status}");
        Err(anyhow!("failed with status {resp_status}: {response}"))
    }
}

async fn get(app_name: AppName, machine_id: MachineId) -> Result<Option<Machine>, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}");
    let request = request_with_api_token()?
        .method(Method::GET)
        .uri(url)
        .body(Body::empty())?;
    let response = Client::new().send(request).await?;
    let resp_status = response.status();
    let mut response = response.into_body();
    let response = response.str_contents().await?;

    if resp_status.is_success() {
        let response: Machine = serde_json::from_str(response)
            .inspect_err(|_| eprintln!("cannot deserialize: {response}"))?;
        Ok(Some(response))
    } else if resp_status == StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        eprintln!("Got error status {resp_status}");
        Err(anyhow!("failed with status {resp_status}: {response}"))
    }
}

async fn create(
    app_name: AppName,
    machine_name: String,
    machine_config: MachineConfig,
    region: Option<Region>,
) -> Result<String, anyhow::Error> {
    {
        let request_payload = MachineCreateRequestSer {
            name: machine_name,
            config: machine_config,
            region,
        };
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .body(Body::from_json(&request_payload)?)?;

        let response = Client::new().send(request).await?;
        let resp_status = response.status();
        let mut response = response.into_body();
        let response = response.str_contents().await?;

        if resp_status.is_success() {
            let resp: MachineCreateResponseSer = serde_json::from_str(response)
                .with_context(|| format!("Deserialization of response failed: `{response}`"))?;
            return Ok(resp.id);
        }
        eprintln!("Got error status {resp_status}");
        if resp_status == StatusCode::CONFLICT {
            let error: ResponseErrorSer = serde_json::from_str(response)
                .with_context(|| format!("cannot parse error response: `{response}`"))?;
            let machine_id = error.get_machine_id_on_creation_conflict().with_context(
                || "machine id cannot be parsed from 409 error response: `{error:?}`",
            )?;
            Ok(machine_id.to_string())
        } else {
            Err(anyhow!("{resp_status} - {response}"))
        }
    }
}

async fn update(
    app_name: AppName,
    machine_id: MachineId,
    machine_config: MachineConfig,
    region: Option<Region>,
) -> Result<(), anyhow::Error> {
    {
        let request_payload = MachineUpdateRequestSer {
            config: machine_config,
            region,
        };
        let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}");
        let request = request_with_api_token()?
            .method(Method::POST)
            .uri(url)
            .body(Body::from_json(&request_payload)?)?;

        let response = Client::new().send(request).await?;
        let resp_status = response.status();
        let mut response = response.into_body();
        let response = response.str_contents().await?;

        if resp_status.is_success() {
            let resp: MachineCreateResponseSer = serde_json::from_str(response)
                .with_context(|| format!("Deserialization of response failed: `{response}`"))?;
            ensure!(
                resp.id == machine_id.as_ref(),
                "unexpected id returned, expected {machine_id} got {id}",
                id = resp.id
            );
            return Ok(());
        }
        bail!("{resp_status} - {response}")
    }
}

async fn exec(
    app_name: AppName,
    machine_id: MachineId,
    command: Vec<String>,
) -> Result<ExecResponse, anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}/exec");
    let body = serde_json::json!({
        "command": command,
    });
    let request = request_with_api_token()?
        .method(Method::POST)
        .uri(url)
        .body(Body::from_json(&body)?)?;
    let response = Client::new().send(request).await?;
    let resp_status = response.status();
    let mut response = response.into_body();
    let response = response.str_contents().await?;

    if resp_status.is_success() {
        let response: ExecResponseSer = serde_json::from_str(response)
            .inspect_err(|_| eprintln!("cannot deserialize: {response}"))?;
        Ok(response.into())
    } else {
        eprintln!("Got error status {resp_status}");
        Err(anyhow!("failed with status {resp_status}: {response}"))
    }
}

async fn change_machine(
    app_name: AppName,
    machine_id: MachineId,
    url_suffix: &'static str,
) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}/{url_suffix}");
    send_request(url, Method::POST).await
}

async fn delete(
    app_name: AppName,
    machine_id: MachineId,
    force: bool,
) -> Result<(), anyhow::Error> {
    let url = format!("{API_BASE_URL}/apps/{app_name}/machines/{machine_id}?force={force}");
    send_request(url, Method::DELETE).await
}

async fn send_request(url: String, method: Method) -> Result<(), anyhow::Error> {
    let request = request_with_api_token()?
        .method(method)
        .uri(url)
        .body(Body::empty())?;

    let response = Client::new().send(request).await?;
    let resp_status = response.status();
    let mut response = response.into_body();
    let response = response.str_contents().await?;

    if resp_status.is_success() {
        Ok(())
    } else {
        Err(anyhow!("failed with status {resp_status}: {response}",))
    }
}

// Implementation of the vm interface for the component.
impl Guest for Component {
    fn list(app_name: String) -> Result<Vec<Machine>, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(list(app_name))
        })()
        .map_err(|err| err.to_string())
    }

    fn get(app_name: String, machine_id: String) -> Result<Option<Machine>, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(get(app_name, machine_id))
        })()
        .map_err(|err| err.to_string())
    }

    fn create(
        app_name: String,
        machine_name: String,
        machine_config: MachineConfig,
        region: Option<Region>,
    ) -> Result<String, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            block_on(create(app_name, machine_name, machine_config, region))
        })()
        .map_err(|err| err.to_string())
    }

    fn update(
        app_name: String,
        machine_id: String,
        machine_config: MachineConfig,
        region: Option<Region>,
    ) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(update(app_name, machine_id, machine_config, region))
        })()
        .map_err(|err| err.to_string())
    }

    fn stop(app_name: String, machine_id: String) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(change_machine(app_name, machine_id, "stop"))
        })()
        .map_err(|err| err.to_string())
    }

    fn suspend(app_name: String, machine_id: String) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(change_machine(app_name, machine_id, "suspend"))
        })()
        .map_err(|err| err.to_string())
    }

    fn start(app_name: String, machine_id: String) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(change_machine(app_name, machine_id, "start"))
        })()
        .map_err(|err| err.to_string())
    }

    fn restart(app_name: String, machine_id: String) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(change_machine(app_name, machine_id, "restart"))
        })()
        .map_err(|err| err.to_string())
    }

    fn delete(app_name: String, machine_id: String, force: bool) -> Result<(), String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(delete(app_name, machine_id, force))
        })()
        .map_err(|err| err.to_string())
    }

    fn exec(
        app_name: String,
        machine_id: String,
        command: Vec<String>,
    ) -> Result<ExecResponse, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(exec(app_name, machine_id, command))
        })()
        .map_err(|err| err.to_string())
    }

    fn exec_check_success(
        app_name: String,
        machine_id: String,
        command: Vec<String>,
    ) -> Result<ExecResponse, String> {
        (|| {
            let app_name = AppName::new(app_name)?;
            let machine_id = MachineId::new(machine_id)?;
            block_on(async {
                let resp = exec(app_name, machine_id, command).await?;
                if resp.exit_code == Some(0) {
                    Ok(resp)
                } else {
                    bail!("non-successful exit status - {resp:?}")
                }
            })
        })()
        .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ser::ResponseErrorSer;
    use crate::generated::{
        exports::obelisk_flyio::activity_fly_http::machines::Machine,
        obelisk_flyio::activity_fly_http::regions::Region,
    };
    use insta::assert_debug_snapshot;
    use serde_json::json;

    #[test]
    fn region_ser() {
        assert_eq!("\"ams\"", serde_json::to_string(&Region::Ams).unwrap());
    }

    #[test]
    fn region_de() {
        assert_matches::assert_matches!(serde_json::from_str("\"ams\"").unwrap(), Region::Ams);
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
        let machine: Machine = serde_json::from_str(json).unwrap();
        assert_debug_snapshot!(machine)
    }
}
