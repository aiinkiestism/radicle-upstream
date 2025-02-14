#!/usr/bin/env bash

# Copyright © 2021 The Radicle Upstream Contributors
#
# This file is part of radicle-upstream, distributed under the GPLv3
# with Radicle Linking Exception. For full terms see the included
# LICENSE file.

set -euo pipefail

curl -sSO https://dl.google.com/cloudagents/add-google-cloud-ops-agent-repo.sh
sudo bash add-google-cloud-ops-agent-repo.sh --also-install

ln -sf "$(pwd)/infra/seed-node/google-cloud-ops-agent-config.yaml" /etc/google-cloud-ops-agent/config.yaml
systemctl restart "google-cloud-ops-agent*"

mkdir -p /var/local/upstream-seed
chown 1000:1000 /var/local/upstream-seed

if [[ ! -f /etc/upstream-seed.env ]]; then
  cp "$(pwd)/infra/seed-node/upstream-seed.env" /etc
fi

ln -sf "$(pwd)/infra/seed-node/upstream-seed.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable upstream-seed
systemctl stop upstream-seed

curl -fsSL \
  https://storage.googleapis.com/radicle-upstream-build-artifacts/v1/main/x86_64-linux/upstream-seed \
  -o /usr/local/bin/upstream-seed
chmod +x /usr/local/bin/upstream-seed

systemctl start upstream-seed
