use log::debug;
use serde::de::DeserializeOwned;
use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::File,
    io::BufReader,
    os::fd::{AsRawFd, FromRawFd},
    process::{Command, Stdio},
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
    let f = &mut BufReader::new(execute(arguments, cli_args)?);
    let elements = serde_jsonlines::JsonLinesReader::new(f).read_all::<R>();
    Ok(elements.collect::<std::io::Result<Vec<R>>>()?)
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
    let f = execute(arguments, cli_args)?;
    Ok(serde_json::from_reader::<_, Vec<R>>(f)?)
}

/*
 * Execute a single command and return the File containing the output to the caller.
 */
pub fn execute<I, S>(arguments: I, cli_args: &CliArguments) -> Result<File, DockerError>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Debug,
{
    debug!("Execute {:?}", arguments);
    let child = Command::new(cli_args.docker.as_str())
        .args(arguments)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.as_ref().unwrap();
    let fd = stdout.as_raw_fd();
    let f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    Ok(f)
}
