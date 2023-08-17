use crate::types::{ContainerInfo, PsInfo};
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

const DOCKER_COMMAND: &str = "/Users/cpolderman/.rd/bin/docker";
const TYPE_BACKUPCONTAINER: &str = "backupcontainer";

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    env_logger::init();
    info!("Docker volume backup v1.0");
    match docker_jsonline_command::<PsInfo, _, _>(vec!["ps", "--format=json"]) {
        Ok(ps_info) => backup_container(ps_info).expect("Backup failed"),
        Err(e) => error!("Error {e}"),
    };
}

fn backup_container(ps_info: Vec<PsInfo>) -> Result<(), std::io::Error> {
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

        let inspected = docker_json_command::<ContainerInfo, _, _>(vec![
            "inspect",
            container_name.as_str(),
            "--format=json",
        ])?;
        if let Some(container_info) = inspected.get(0) {
            if !backup_all_mounts(container_info, &ps_info)? {
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
            error!("[{container_name}] Response from inspect is wrong")
        }
    }
    Ok(())
}

fn backup_all_mounts(
    container_info: &ContainerInfo,
    container: &PsInfo,
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

    for mount in container_info.mounts.iter() {
        info!("[{}] - backing up {}", container.names, mount.destination);

        let mut child = Command::new("/Users/cpolderman/.rd/bin/docker")
            .arg("run")
            .arg("--rm")
            .arg("--label")
            .arg(format!("type={}", TYPE_BACKUPCONTAINER))
            .arg("-v")
            .arg(".:/backupdest")
            .arg("--volumes-from")
            .arg(container_info.id.as_str())
            .arg(env::var("BACKUP_IMAGE").unwrap_or(String::from("ubuntu")))
            .arg("tar")
            .arg("cvf")
            .arg(format!(
                "/backupdest/{}{}.tar",
                container.names,
                sanitize(&mount.destination).as_str()
            ))
            .arg(mount.destination.as_str())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;
        let exit_status = child.wait().unwrap();
        if !exit_status.success() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn sanitize(s: &str) -> String {
    s.replace('/', "_")
}

fn docker_jsonline_command<R, I, S>(args: I) -> Result<Vec<R>, std::io::Error>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    debug!("Execute jsonline {:?}", args);
    let child = Command::new(DOCKER_COMMAND)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.as_ref().unwrap();
    let fd = stdout.as_fd();
    let f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    serde_jsonlines::JsonLinesReader::new(&mut BufReader::new(f))
        .read_all::<R>()
        .collect::<std::io::Result<Vec<R>>>()
}

fn docker_json_command<R, I, S>(args: I) -> Result<Vec<R>, std::io::Error>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    debug!("Execute json {:?}", args);
    let child = Command::new(DOCKER_COMMAND)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.as_ref().unwrap();
    let fd = stdout.as_fd();
    let f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    Ok(serde_json::from_reader::<_, Vec<R>>(f)?)
}
