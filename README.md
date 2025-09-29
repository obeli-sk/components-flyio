# WASM Components for interacting with fly.io

## activity-fly-http
Activity that uses the official [Machines API](https://docs.machines.dev/) to interact with:
* Apps
* VMs
* Volumes
* Secrets

Check out the [WIT definition](activity/fly-http/wit/obelisk-flyio_activity-fly-http@1.0.0-beta/fly.wit).

## webhook-fly-secrets-updater
Webhook endpoint for creating and updating secret values in a fly.io App.

### Prerequisites
Obelisk, Rust and other dependencies can be installed using Nix and Direnv:
```sh
cp .envrc-example .envrc
# Modify .envrc - enter your fly.io token, org slug and app name.
direnv allow
```
Otherwise see [dev-deps.txt](./dev-deps.txt) for exact version of each build dependecy. Environment variables
like `FLY_API_TOKEN` must be present, check out [.envrc-example](./.envrc-example) .

### Start the Obelisk server
```sh
just build serve
```

### Submit activity executions
Executions can be submitted and observed either using CLI or the WebUI at http://localhost:8080 .

List apps:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/apps@1.0.0-beta.list -- \
\"$FLY_ORG_SLUG\"
```

Create an app:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/apps@1.0.0-beta.put -- \
\"$FLY_ORG_SLUG\" \"$FLY_APP_NAME\"
```

List secret keys of the app:
```sh
obelisk client execution submit -f  obelisk-flyio:activity-fly-http/secrets@1.0.0-beta.list -- \
\"$FLY_APP_NAME\"
```

Insert or update a secret (note this is a webhook endpoint to avoid persisting the secret):
```sh
curl -v localhost:9090/ -X POST -d '{"app_name":"'$FLY_APP_NAME'","name":"foo","value":"bar"}'
```

List VMs:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/machines@1.0.0-beta.list -- \
\"$FLY_APP_NAME\"
```

List volumes:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/volumes@1.0.0-beta.list -- \
\"$FLY_APP_NAME\"
```

Create a volume:
```sh
export VOLUME_ID=$(obelisk client execution submit -f --json obelisk-flyio:activity-fly-http/volumes@1.0.0-beta.create -- \
\"$FLY_APP_NAME\" '{
      "name": "my_app_vol",
      "region": "ams",
      "size-gb": 1
    }' | jq -r '.[-1].ok.id')
```

Delete the volume:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/volumes@1.0.0-beta.delete -- \
\"$FLY_APP_NAME\" \"$VOLUME_ID\"
```

Launch a VM:
```sh
MACHINE_ID=$(obelisk client execution submit -f --json obelisk-flyio:activity-fly-http/machines@1.0.0-beta.create -- \
\"$FLY_APP_NAME\" \"$FLY_MACHINE_NAME\" "$(./scripts/fly-http-machine-config.json.sh)" \"$FLY_REGION\" \
| jq -r '.[-1].ok')
```

Get the VM:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/machines@1.0.0-beta.get -- \
\"$FLY_APP_NAME\" \"$MACHINE_ID\"
```

Delete the VM:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/machines@1.0.0-beta.delete -- \
\"$FLY_APP_NAME\" \"$MACHINE_ID\" true
```

Delete the App:
```sh
obelisk client execution submit -f obelisk-flyio:activity-fly-http/apps@1.0.0-beta.delete -- \
\"$FLY_APP_NAME\" true
```
