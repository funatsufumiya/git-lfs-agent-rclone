# `git-lfs-agent-rclone`

A custom transfer agent for [`git-lfs`](https://git-lfs.github.com/) that uses [`rclone`](https://www.openssh.com/) to transfer files.
This is useful if you do not want to install a `git-lfs` server.

## Usage

Configure your local git repository as follows

```sh
$ git config lfs.standalonetransferagent rclone <1>
$ git config lfs.customtransfer.rclone.path git-lfs-agent-rclone <2>
$ git config lfs.customtransfer.rclone.args $DESTINATION <3>
```
1. tell `git-lfs` to use the transfer agent named "rclone"
2. tell `git-lfs` what the name of the program is of the transfer agent
3. `$DESTINATION` is the destination to which `rclone` will copy files tracked by `git-lfs` when running `$ git pull` and the place it will store files when running `$ git push`

- NOTE: `$DESTINATION` can be set to anything `rclone` understands.
      As an example, `source:/home/git/my-lfs-files` ships files to a remote server over `rclone`.

## Install

Clone the source and run:

```sh
$ cargo install --path .
```
