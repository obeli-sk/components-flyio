use crate::generated::obelisk::activity::process::{self as process_support};
use anyhow::{Context, anyhow, ensure};
use futures_concurrency::future::Join;
use wasip2::io::streams::InputStream;
use wstd::io::{AsyncInputStream, AsyncPollable, Cursor};

/// Executes a docker command, waits for it to finish, and returns stdout.
/// Returns error on non-zero exit code.
pub async fn exec(args: Vec<String>) -> Result<String, anyhow::Error> {
    // Inject "docker" as the binary.
    // Note: The runner environment must have the 'docker' binary available in PATH
    // or aliased appropriately via the runner configuration.

    let proc = process_support::spawn(
        "docker",
        &process_support::SpawnOptions {
            args,
            environment: vec![], // Inherit or set specific env vars if needed
            current_working_directory: None,
            stdin: process_support::Stdio::Discard,
            stdout: process_support::Stdio::Pipe,
            stderr: process_support::Stdio::Pipe,
        },
    )
    .map_err(|e| anyhow!("Failed to spawn docker process: {:?}", e))?;

    let stdout_stream = proc.take_stdout().context("Failed to take stdout")?;
    let stderr_stream = proc.take_stderr().context("Failed to take stderr")?;

    // We need to read streams concurrently while waiting for the process
    let stdout_fut = stream_to_string(stdout_stream);
    let stderr_fut = stream_to_string(stderr_stream);

    // Subscribe to wait
    let wait_fut = AsyncPollable::new(proc.subscribe_wait()).wait_for();

    // Run everything
    let (stdout_res, stderr_res, _wait) = (stdout_fut, stderr_fut, wait_fut).join().await;

    let exit_status = proc
        .wait()
        .map_err(|e| anyhow!("Failed to wait on process: {:?}", e))?;
    let stdout = stdout_res?;
    let stderr = stderr_res?;

    ensure!(
        exit_status == Some(0),
        "Docker command failed (Exit {:?}).\nStderr: {}\nStdout: {}",
        exit_status,
        stderr.trim(),
        stdout.trim()
    );

    Ok(stdout.trim().to_string())
}

async fn stream_to_string(stream: InputStream) -> Result<String, anyhow::Error> {
    let mut buffer = Cursor::new(Vec::new());
    let stream = AsyncInputStream::new(stream);
    wstd::io::copy(stream, &mut buffer).await?;
    let output = buffer.into_inner();
    Ok(String::from_utf8_lossy(&output).into_owned())
}

/// Checks if a resource exists by inspecting it.
/// Returns Ok(true) if exists, Ok(false) if 'No such object', Err on other failures.
pub async fn check_exists(resource_type: &str, name: &str) -> Result<bool, anyhow::Error> {
    // We use 'docker inspect'
    let args = vec![
        "inspect".to_string(),
        "--type".to_string(),
        resource_type.to_string(),
        name.to_string(),
    ];
    match exec(args).await {
        Ok(_) => Ok(true),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("No such") || err_str.contains("Error: No such object") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}
