#!/usr/bin/env bash

set -exuo pipefail
cd "$(dirname "$0")/.."

(
cd fly-http/wit
obelisk generate extensions activity_wasm .
)
