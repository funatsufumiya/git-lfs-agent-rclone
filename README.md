# `git-lfs-agent-scp`

A custom transfer agent for https://git-lfs.github.com/[`git-lfs`] that uses https://www.openssh.com/[`scp`] to transfer files.
This transfer agent makes it possible to use `git-lfs` in situations where the remote only speaks `ssh`.
This is useful if you do not want to install a `git-lfs` server.

## Usage

Configure your local git repository as follows

```sh
$ git config lfs.standalonetransferagent scp <1>
$ git config lfs.customtransfer.scp.path git-lfs-agent-scp <2>
$ git config lfs.customtransfer.scp.args $DESTINATION <3>
```
<1> tell `git-lfs` to use the transfer agent named "scp"
<2> tell `git-lfs` what the name of the program is of the transfer agent
<3> `$DESTINATION` is the destination to which `scp` will copy files tracked by `git-lfs` when running `$ git pull` and the place it will store files when running `$ git push`

NOTE: `$DESTINATION` can be set to anything `scp` understands.
      As an example, `server.example.com:/home/git/my-lfs-files` ships files to a remote server over `ssh`.

## Install

Clone the source and run:

```sh
$ cargo install --path .
```
