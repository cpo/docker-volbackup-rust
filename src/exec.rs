use log::debug;
use serde::de::DeserializeOwned;
use std::{
    ffi::OsStr,
    fmt::Debug,
    io::{self},
    process::{Command, Output, Stdio},
};

use crate::{types::DockerError, CliArguments};

/*
 * Execute a docker command without output.
 */
pub fn docker_outputless_command(
    cli_args: &CliArguments,
    arguments: Vec<&str>,
) -> Result<(), DockerError> {
    let mut child = Command::new(cli_args.docker.as_str())
        .args(arguments)
        .stdout(Stdio::null())
        .spawn()?;
    let exit_status = child.wait()?;
    if !exit_status.success() {
        Err(DockerError {
            message: String::from(""),
        })
    } else {
        Ok(())
    }
}

/*
 * Execute a docker command and parse the output as jsonline.
 */
pub fn docker_jsonline_command<R, I, S>(
    arguments: I,
    cli_args: &CliArguments,
) -> Result<Vec<R>, DockerError>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    let f = execute_with_output(arguments, cli_args)?;
    let elements = serde_jsonlines::JsonLinesReader::new(f.stdout.as_slice()).read_all::<R>();
    Ok(elements.collect::<io::Result<Vec<R>>>()?)
}

/*
 * Execute a docker command and parse the output as json.
 */
pub fn docker_json_command<R, I, S>(
    arguments: I,
    cli_args: &CliArguments,
) -> Result<Vec<R>, DockerError>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
    R: DeserializeOwned,
{
    let f = execute_with_output(arguments, cli_args)?;
    Ok(serde_json::from_reader::<_, Vec<R>>(f.stdout.as_slice())?)
}

/*
 * Execute a single command and return the File containing the output to the caller.
 */
pub fn execute_with_output<I, S>(
    arguments: I,
    cli_args: &CliArguments,
) -> Result<Output, DockerError>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
{
    debug!("Execute {:?}", arguments);
    Ok(Command::new(cli_args.docker.as_str())
        .args(arguments)
        .stdout(Stdio::piped())
        .output()?)
}
