use serde::Deserialize;
use std::collections::HashMap;

/*
 * Docker json types.
 */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PsInfo {
    pub names: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInfo {
    pub id: String,
    pub mounts: Vec<Mounts>,
    pub config: ContainerConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mounts {
    pub destination: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub labels: HashMap<String, String>,
}

#[derive(Debug)]
pub struct DockerError {
    pub message: String,
}

impl From<std::io::Error> for DockerError {
    fn from(value: std::io::Error) -> Self {
        DockerError {
            message: value.to_string(),
        }
    }
}

impl From<serde_json::Error> for DockerError {
    fn from(value: serde_json::Error) -> Self {
        DockerError {
            message: value.to_string(),
        }
    }
}
