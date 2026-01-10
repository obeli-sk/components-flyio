use crate::docker_cli;
use crate::generated::exports::obelisk_docker::activity_docker::networks::Guest;
use wstd::runtime::block_on;

async fn create_network(name: String, driver: Option<String>) -> Result<String, anyhow::Error> {
    // Idempotency: Check existence
    if docker_cli::check_exists("network", &name).await? {
        // Return name (ID is harder to get without inspect, but name is sufficient for docker CLI ref)
        return Ok(name);
    }

    let mut args = vec!["network".to_string(), "create".to_string()];
    if let Some(d) = driver {
        args.push("--driver".to_string());
        args.push(d);
    }
    args.push(name.clone());

    let id = docker_cli::exec(args).await?;
    Ok(id)
}

async fn rm_network(name: String) -> Result<(), anyhow::Error> {
    if !docker_cli::check_exists("network", &name).await? {
        return Ok(());
    }
    docker_cli::exec(vec!["network".to_string(), "rm".to_string(), name]).await?;
    Ok(())
}

async fn prune_networks() -> Result<(), anyhow::Error> {
    docker_cli::exec(vec![
        "network".to_string(),
        "prune".to_string(),
        "-f".to_string(),
    ])
    .await?;
    Ok(())
}

impl Guest for crate::Component {
    fn create(name: String, driver: Option<String>) -> Result<String, String> {
        block_on(create_network(name, driver)).map_err(|e| e.to_string())
    }

    fn rm(name: String) -> Result<(), String> {
        block_on(rm_network(name)).map_err(|e| e.to_string())
    }

    fn prune() -> Result<(), String> {
        block_on(prune_networks()).map_err(|e| e.to_string())
    }
}
