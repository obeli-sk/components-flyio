#!/usr/bin/env bash

cat  <<EOF
{
    "image": "registry.fly.io/$FLY_APP_NAME:$FLY_IMAGE_TAG",
    "init": {
        "swap-size-mb": 256
    },
    "guest": {
        "cpu-kind": "shared",
        "cpus": 1,
        "memory-mb": 256
    },
    "restart": {
        "policy": "no"
    },
    "region": "ams"
}
EOF
