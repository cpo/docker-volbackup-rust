use crate::types::{ContainerInfo, PsInfo};
use clap::Parser;
use log::{debug, error, info};
use serde::de::DeserializeOwned;
use std::{
    env,
    ffi::OsStr,
    fmt::Debug,
    fs::File,
    io::BufReader,
    os::fd::{AsFd, AsRawFd, FromRawFd},
    process::{Command, Stdio},
};

mod types;

const TYPE_BACKUPCONTAINER: &str = "backupcontainer";

/// Backup all mounted volumes connected to a running container.
#[derive(Parser)]
struct CliArguments {
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

fn main() {
    let cli_args = CliArguments::parse();
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", cli_args.loglevel.as_str())
    }
    env_logger::init();

    info!("Docker volume backup v1.0");

    match docker_jsonline_command::<PsInfo, _, _>(vec!["ps", "--format=json"], &cli_args) {
        Ok(ps_info) => backup_container(ps_info, cli_args).expect("Backup failed"),
        Err(e) => error!("Error {e}"),
    };
}

fn backup_container(ps_info: Vec<PsInfo>, cli_args: CliArguments) -> Result<(), std::io::Error> {
    info!(
        "Found containers: {:?}",
        ps_info
            .iter()
            .map(|f| { f.names.as_str() })
            .collect::<Vec<&str>>()
    );

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
    Ok(())
}

fn backup_all_mounts(
    container_info: &ContainerInfo,
    container: &PsInfo,
    cli_args: &CliArguments,
) -> Result<bool, std::io::Error> {
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
        let mut child = Command::new(cli_args.docker.as_str())
            .arg("stop")
            .arg(container_info.id.as_str())
            .spawn()?;
        let exit_status = child.wait()?;
        if !exit_status.success() {
            return Ok(false);
        }
    }

    for mount in container_info.mounts.iter() {
        info!("[{}] - backing up {}", container.names, mount.destination);
        let mut child = Command::new(cli_args.docker.as_str())
            .arg("run")
            .arg("--rm")
            .arg("--label")
            .arg(format!("type={}", TYPE_BACKUPCONTAINER))
            .arg("-v")
            .arg(".:/backupdest")
            .arg("--volumes-from")
            .arg(container_info.id.as_str())
            .arg(cli_args.image.as_str())
            .arg("tar")
            .arg("cvf")
            .arg(format!(
                "/backupdest/{}{}.tar",
                container.names,
                sanitize(&mount.destination).as_str()
            ))
            .arg(mount.destination.as_str())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()?;
        let exit_status = child.wait()?;
        if !exit_status.success() {
            return Ok(false);
        }
    }
    if cli_args.stop_start {
        info!("[{}] Restarting container", container.names);
        let mut child = Command::new(cli_args.docker.as_str())
            .arg("start")
            .arg(container_info.id.as_str())
            .spawn()?;
        let exit_status = child.wait()?;
        if !exit_status.success() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn sanitize(s: &str) -> String {
    s.replace('/', "_")
}

fn docker_jsonline_command<R, I, S>(
    arguments: I,
    cli_args: &CliArguments,
) -> Result<Vec<R>, std::io::Error>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    debug!("Execute jsonline {:?}", arguments);
    let child = Command::new(cli_args.docker.as_str())
        .args(arguments)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.as_ref().unwrap();
    let fd = stdout.as_fd();
    let f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    serde_jsonlines::JsonLinesReader::new(&mut BufReader::new(f))
        .read_all::<R>()
        .collect::<std::io::Result<Vec<R>>>()
}

fn docker_json_command<R, I, S>(
    arguments: I,
    cli_args: &CliArguments,
) -> Result<Vec<R>, std::io::Error>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    debug!("Execute json {:?}", arguments);
    let child = Command::new(cli_args.docker.as_str())
        .args(arguments)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.as_ref().unwrap();
    let fd = stdout.as_fd();
    let f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    Ok(serde_json::from_reader::<_, Vec<R>>(f)?)
}
