#!/bin/bash

# Copyright © 2021 The Radicle Upstream Contributors
#
# This file is part of radicle-upstream, distributed under the GPLv3
# with Radicle Linking Exception. For full terms see the included
# LICENSE file.

source ci/env.sh

log-group-start "install toolcahin"
time rustup show active-toolchain
log-group-start "install toolcahin"

log-group-start "yarn install"
# Unsetting GITHUB_ACTIONS because yarn tries to add log groups in a buggy way.
env -u GITHUB_ACTIONS yarn install --immutable
log-group-end

log-group-start "Building and packaging binaries"
time yarn dist
log-group-end

clean-cargo-build-artifacts

echo "Collect artifacts"
mkdir artifacts
shopt -s nullglob
shopt -u failglob
cp --archive \
  --target-directory artifacts \
  dist/*.dmg \
  dist/*.AppImage

target="$(uname -m)-$(uname -s | tr "[:upper:]" "[:lower:]")"
mkdir "artifacts/${target}"
cp \
  --target-directory "artifacts/${target}" \
  target/release/upstream-seed \
  target/release/rad
