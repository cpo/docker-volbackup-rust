use crate::{
    exec::{docker_json_command, docker_jsonline_command, docker_outputless_command},
    types::{ContainerInfo, PsInfo},
};
use clap::Parser;
use log::{debug, error, info};
use std::{env, process::ExitCode};
use types::DockerError;

mod exec;
mod types;

const TYPE_BACKUPCONTAINER: &str = "backupcontainer";

/// Backup all mounted volumes connected to a running container.
#[derive(Parser)]
pub struct CliArguments {
    /// Stop the container before backup and restart it afterwards
    #[arg(short, long, default_value = "false")]
    stop_start: bool,

    /// The image to use for running a volume backup
    #[arg(short, long, default_value = "ubuntu")]
    image: String,

    /// Logging level
    #[arg(short, long, default_value = "info")]
    loglevel: String,

    /// Where to find the docker executable
    #[arg(short, long, default_value = "/usr/bin/docker")]
    docker: String,
}

/*
 * Entrypoint.
 */
fn main() -> ExitCode {
    let cli_args = CliArguments::parse();
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", cli_args.loglevel.as_str())
    }
    env_logger::init();

    info!("Docker volume backup v1.0");

    match docker_jsonline_command::<PsInfo, _, _>(vec!["ps", "--format=json"], &cli_args) {
        Ok(ps_info) => match backup_container(ps_info, cli_args).expect("Backup failed") {
            true => ExitCode::FAILURE,
            false => ExitCode::SUCCESS,
        },
        Err(e) => {
            error!("Error {:?}", e);
            ExitCode::SUCCESS
        }
    }
}

/*
 * Inspect a container to find out the mounts.
 */
fn backup_container(ps_info: Vec<PsInfo>, cli_args: CliArguments) -> Result<bool, DockerError> {
    info!(
        "Found containers: {:?}",
        ps_info
            .iter()
            .map(|f| { f.names.as_str() })
            .collect::<Vec<&str>>()
    );

    let mut has_errors = false;
    for ps_info in ps_info {
        let container_name = &ps_info.names;
        info!(
            "[{container_name}] Getting container information for {}",
            container_name
        );

        let inspected = docker_json_command::<ContainerInfo, _, _>(
            vec!["inspect", container_name.as_str(), "--format=json"],
            &cli_args,
        )?;
        if let Some(container_info) = inspected.get(0) {
            if !backup_all_mounts(container_info, &ps_info, &cli_args)? {
                has_errors = true;
                error!(
                    "[{container_name}] Error backing up container {}",
                    container_name
                )
            } else {
                info!(
                    "[{container_name}] Backup of container {} done.",
                    container_name
                )
            }
        } else {
            error!("[{container_name}] Response from inspect is wrong (no data returned)")
        }
    }
    Ok(!has_errors)
}

/*
 * Backup the mounts listed in the container as tar files.
 */
fn backup_all_mounts(
    container_info: &ContainerInfo,
    container: &PsInfo,
    cli_args: &CliArguments,
) -> Result<bool, DockerError> {
    debug!("Inspect: {:?}", container_info);
    info!("[{}] Start backup of volumes", container.names);

    if *container_info
        .config
        .labels
        .get("type")
        .unwrap_or(&"-".to_string())
        == TYPE_BACKUPCONTAINER
    {
        info!(
            "[{}] Skipping this container as it it a backup container",
            container.names
        );
        return Ok(true);
    }

    if cli_args.stop_start {
        info!("[{}] Stopping container", container.names);
        docker_outputless_command(cli_args, vec!["stop", container_info.id.as_str()])?;
    }

    let mut errors = 0;
    for mount in container_info.mounts.iter() {
        info!("[{}] - backing up {}", container.names, mount.destination);
        if docker_outputless_command(
            cli_args,
            vec![
                "run",
                "--rm",
                "--label",
                format!("type={}", TYPE_BACKUPCONTAINER).as_str(),
                "-v",
                ".:/backupdest",
                "--volumes-from",
                container_info.id.as_str(),
                cli_args.image.as_str(),
                "tar",
                "cvf",
                format!(
                    "/backupdest/{}{}.tar",
                    container.names,
                    sanitize(&mount.destination).as_str()
                )
                .as_str(),
                mount.destination.as_str(),
            ],
        ).is_err() {
            error!(
                "[{}] Error in backup of volume {}",
                container.names, mount.destination
            );
            errors += 1;
        };
    }
    if cli_args.stop_start {
        info!("[{}] Restarting container", container.names);
        docker_outputless_command(cli_args, vec!["start", container_info.id.as_str()])?;
    }

    Ok(errors == 0)
}

/*
 * Sanitize a path into part of the backup filename.
 */
fn sanitize(s: &str) -> String {
    s.replace('/', "_")
}
