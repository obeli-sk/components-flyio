#!/usr/bin/env bash

cat <<EOF
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
    "services": [
        {
            "internal-port": 8080,
            "ports": [
                {
                    "handlers": [
                        "http",
                        "tls"
                    ],
                    "port": 443
                }
            ],
            "protocol": "tcp"
        }
    ],
    "restart": {
        "policy": "no"
    }$(if [ -n "$VOLUME_ID" ]; then
        echo ',
    "mounts": [
        {
            "volume": "'"$VOLUME_ID"'",
            "path": "/opt"
        }
    ]'
    fi)
}
EOF
