#!/usr/bin/env bash

set -exuo pipefail
cd "$(dirname "$0")/.."

obelisk generate extensions activity_wasm activity-fly-http/wit/
