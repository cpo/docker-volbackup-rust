# Docker volume backup

[![Rust](https://github.com/cpo/volbackup-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/cpo/volbackup-rust/actions/workflows/rust.yml)

## What is this wizardry
This is a small utility to automatically backup all volumes which are connected to a container.

> WARNING: some containers keep files open or rewrite files during the backup, this might cause data loss. If you want to be sure the backup is 1:1, please use the `start-stop` commandline option.

## Commandline usage

```
Backup all mounted volumes connected to a running container

Usage: volbackup [OPTIONS]

Options:
  -s, --stop-start           Stop the container before backup and restart it afterwards
  -i, --image <IMAGE>        The image to use for running a volume backup [default: ubuntu]
  -l, --loglevel <LOGLEVEL>  Logging level [default: info]
  -d, --docker <DOCKER>      Where to find the docker executable [default: /usr/bin/docker]
  -h, --help                 Print help
```

## Overview of the backup process

1. Query all running containers using `docker ps`.
1. For every container, run inspect to retrieve the mounted volumes.
1. If the commandline option `start-stop` has been given, stop the container.
1. Per mounted volume, start a docker container to `tar` the contents of the mounted location.
1. If the commandline option `start-stop` has been given, start the container again.
