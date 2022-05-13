#!/bin/sh

#
# Mostly used this as a crutch during development / for debugging purposes.
#

mydir=$(pwd) # Assumes script is ran from the project directory.
workdir=$(mktemp -d)

echo >&2 working in \"$workdir\"

SRC_GIT=$workdir/src.git
DEST_GIT=$workdir/dest.git
DEST_GIT_LFS_FILES=$workdir/dest.git.lfs
mkdir $SRC_GIT $DEST_GIT $DEST_GIT_LFS_FILES

#
# Setup of git repositories and files for testing.
#

# Create destination git repository.
git -C $DEST_GIT init --bare

# Create source repository and enable git-lfs
git="git -C $SRC_GIT"
$git init
$git lfs install
# Add a binary file to the repository, tracked by git-lfs
dd if=/dev/random of=$SRC_GIT/binaryfile.bin bs=1m count=1
$git lfs track '*.bin'
$git add --all
# Create a single commit
$git commit --message 'commit message'
# Add $DEST_GIT as a remote to the source repository
$git remote add origin $DEST_GIT
# Configure the source repository to use git-lfs-agent-scp to transfer files
$git config lfs.standalonetransferagent scp
$git config lfs.concurrenttransfers 1
$git config lfs.customtransfer.scp.path $mydir/git-lfs-agent-scp
$git config lfs.customtransfer.scp.args $DEST_GIT_LFS_FILES

#
# Test uploading.
#
GIT_TRACE=1 $git push

if [ $? -ne 0 ]; then
  echo >&2 git push failed
  exit 1
fi

# Hack: remove the object tracked by git-lfs in the source repository and have git-lfs detect that with fsck so we can then download it
find $SRC_GIT/.git/lfs/objects -type f -delete
$git lfs fsck

#
# Test downloading
#
GIT_TRACE=1 $git lfs pull
