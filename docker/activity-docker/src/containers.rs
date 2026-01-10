use crate::docker_cli;
use crate::generated::exports::obelisk_docker::activity_docker::containers::{
    ContainerConfig, ContainerInfo, ContainerSummary, Guest,
};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use wstd::runtime::block_on;

// Structures for parsing Docker JSON output
#[derive(Deserialize)]
struct DockerInspectContainer {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "State")]
    state: DockerState,
}

#[derive(Deserialize)]
struct DockerState {
    #[serde(rename = "Status")]
    status: String,
}

#[derive(Deserialize)]
struct DockerPsEntry {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Names")]
    name: String, // Docker returns "name1,name2" string in PS usually
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Status")]
    status: String,
}

async fn run_container(name: String, config: ContainerConfig) -> Result<String, anyhow::Error> {
    // Build docker run command
    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        name.clone(),
    ];

    // Environment
    for (key, val) in config.env {
        args.push("-e".to_string());
        args.push(format!("{}={}", key, val));
    }

    // Ports
    for port in config.ports {
        args.push("-p".to_string());
        args.push(format!(
            "{}:{}/{}",
            port.host_port, port.container_port, port.protocol
        ));
    }

    // Mounts
    for mount in config.mounts {
        args.push("-v".to_string());
        let mode = if mount.readonly { "ro" } else { "rw" };
        args.push(format!("{}:{}:{}", mount.source, mount.target, mode));
    }

    // Network
    if let Some(net) = config.network {
        args.push("--network".to_string());
        args.push(net);
    }

    // Image
    args.push(config.image);

    // Command
    if let Some(cmd_parts) = config.cmd {
        args.extend(cmd_parts);
    }

    // Execute run
    match docker_cli::exec(args).await {
        Ok(id) => Ok(id),
        Err(e) => {
            let err_msg = e.to_string();
            // Check for conflict (container name already in use)
            if err_msg.contains("Conflict") || err_msg.contains("is already in use") {
                // Idempotency check: Is it the container we want, and is it running?
                if let Some(info) = inspect_container(name.clone()).await? {
                    if info.state == "running" {
                        return Ok(info.id);
                    } else {
                        return Err(anyhow!(
                            "Container '{}' exists but is in state '{}'. Use 'start' to resume or 'rm' to replace.",
                            name,
                            info.state
                        ));
                    }
                }
            }
            Err(e)
        }
    }
}

async fn start_container(name: String) -> Result<(), anyhow::Error> {
    // check existence first to avoid weird errors or handle idempotency
    let inspect = inspect_container(name.clone()).await?;
    if let Some(info) = inspect {
        if info.state == "running" {
            return Ok(());
        }
    } else {
        return Err(anyhow!("Container '{}' not found", name));
    }

    docker_cli::exec(vec!["start".to_string(), name]).await?;
    Ok(())
}

async fn stop_container(name: String) -> Result<(), anyhow::Error> {
    if !docker_cli::check_exists("container", &name).await? {
        return Ok(());
    }
    let _ = docker_cli::exec(vec!["stop".to_string(), name]).await;
    Ok(())
}

async fn rm_container(name: String, force: bool) -> Result<(), anyhow::Error> {
    if !docker_cli::check_exists("container", &name).await? {
        return Ok(());
    }

    let mut args = vec!["rm".to_string()];
    if force {
        args.push("-f".to_string());
    }
    args.push(name);

    docker_cli::exec(args).await?;
    Ok(())
}

async fn inspect_container(name: String) -> Result<Option<ContainerInfo>, anyhow::Error> {
    let args = vec!["inspect".to_string(), name];
    match docker_cli::exec(args).await {
        Ok(json_output) => {
            let details: Vec<DockerInspectContainer> =
                serde_json::from_str(&json_output).context("Failed to parse inspect output")?;
            if let Some(c) = details.first() {
                Ok(Some(ContainerInfo {
                    id: c.id.clone(),
                    state: c.state.status.clone(),
                }))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

async fn list_containers(all: bool) -> Result<Vec<ContainerSummary>, anyhow::Error> {
    let mut args = vec![
        "ps".to_string(),
        "--format".to_string(),
        "{{json .}}".to_string(),
    ];
    if all {
        args.push("-a".to_string());
    }

    let output = docker_cli::exec(args).await?;

    // docker ps with format json outputs one JSON object per line, not a JSON array.
    // We need to parse line by line.
    let mut containers = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: DockerPsEntry = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse ps entry: {}", line))?;

        containers.push(ContainerSummary {
            id: entry.id,
            name: entry.name,
            image: entry.image,
            state: entry.state,
            status: entry.status,
        });
    }

    Ok(containers)
}

impl Guest for crate::Component {
    fn run(name: String, config: ContainerConfig) -> Result<String, String> {
        block_on(run_container(name, config)).map_err(|e| e.to_string())
    }

    fn start(name: String) -> Result<(), String> {
        block_on(start_container(name)).map_err(|e| e.to_string())
    }

    fn stop(name: String) -> Result<(), String> {
        block_on(stop_container(name)).map_err(|e| e.to_string())
    }

    fn rm(name: String, force: bool) -> Result<(), String> {
        block_on(rm_container(name, force)).map_err(|e| e.to_string())
    }

    fn inspect(name: String) -> Result<Option<ContainerInfo>, String> {
        block_on(inspect_container(name)).map_err(|e| e.to_string())
    }

    fn list(all: bool) -> Result<Vec<ContainerSummary>, String> {
        block_on(list_containers(all)).map_err(|e| e.to_string())
    }
}
