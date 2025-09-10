#!/usr/bin/env bash

cat  <<EOF
{
    "image": "$FLY_IMAGE",
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
