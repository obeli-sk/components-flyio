use crate::docker_cli;
use crate::generated::exports::obelisk_docker::activity_docker::volumes::Guest;
use wstd::runtime::block_on;

async fn create_volume(name: String) -> Result<String, anyhow::Error> {
    if docker_cli::check_exists("volume", &name).await? {
        return Ok(name);
    }

    let args = vec!["volume".to_string(), "create".to_string(), name.clone()];
    // Output is usually the volume name
    let _ = docker_cli::exec(args).await?;
    Ok(name)
}

async fn rm_volume(name: String) -> Result<(), anyhow::Error> {
    if !docker_cli::check_exists("volume", &name).await? {
        return Ok(());
    }
    docker_cli::exec(vec!["volume".to_string(), "rm".to_string(), name]).await?;
    Ok(())
}

async fn exists_volume(name: String) -> Result<bool, anyhow::Error> {
    docker_cli::check_exists("volume", &name).await
}

impl Guest for crate::Component {
    fn create(name: String) -> Result<String, String> {
        block_on(create_volume(name)).map_err(|e| e.to_string())
    }

    fn rm(name: String) -> Result<(), String> {
        block_on(rm_volume(name)).map_err(|e| e.to_string())
    }

    fn exists(name: String) -> Result<bool, String> {
        block_on(exists_volume(name)).map_err(|e| e.to_string())
    }
}
