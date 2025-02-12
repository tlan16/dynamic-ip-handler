#!/usr/bin/env bash
cd "$(dirname "$0")/../" || exit 1
set -euro pipefail

docker compose run --rm app
docker container prune --force
docker image prune --force
