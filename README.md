# WASM Components for interacting with fly.io

## activity-fly-http
Activity that uses the official [Machines API](https://docs.machines.dev/) to interact with:
* Apps
* VMs
* Volumes
* Secrets

Check out the [WIT definition](activity/fly-http/wit/obelisk-flyio_activity-fly-http%401.0.0-beta/fly.wit).

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

#### Apps

List apps:
```sh
obelisk client execution submit -f .../apps.list -- \
\"$FLY_ORG_SLUG\"
```

Create an app:
```sh
obelisk client execution submit -f .../apps.put -- \
\"$FLY_ORG_SLUG\" \"$FLY_APP_NAME\"
```

Delete the app:
```sh
obelisk client execution submit -f .../apps.delete -- \
\"$FLY_APP_NAME\" true
```

#### IPs

List IPs:
```sh
obelisk client execution submit -f .../ips.list -- \
\"$FLY_APP_NAME\"
```
Allocate an IP:
```sh
IP=$(obelisk client execution submit -f --json .../ips.allocate-unsafe -- \
\"$FLY_APP_NAME\" '{ "config":{ "ipv6": {"region": null} } }' | jq -r '.[-1].ok' )
```
Release an IP:
```sh
obelisk client execution submit -f .../ips.release -- \
\"$FLY_APP_NAME\" \"$IP\"
```

#### Secrets

List secret keys of the app:
```sh
obelisk client execution submit -f  .../secrets.list -- \
\"$FLY_APP_NAME\"
```

Insert or update a secret (note this is a webhook endpoint to avoid persisting the secret):
```sh
curl -v localhost:9090/ -X POST -d '{"app_name":"'$FLY_APP_NAME'","name":"foo","value":"bar"}'
```
#### Volumes

List volumes:
```sh
obelisk client execution submit -f .../volumes.list -- \
\"$FLY_APP_NAME\"
```

Create a volume:
```sh
export VOLUME_ID=$(obelisk client execution submit -f --json .../volumes.create -- \
\"$FLY_APP_NAME\" '{
      "name": "my_app_vol",
      "region": "ams",
      "size-gb": 1
    }' | jq -r '.[-1].ok.id')
```

Delete the volume:
```sh
obelisk client execution submit -f .../volumes.delete -- \
\"$FLY_APP_NAME\" \"$VOLUME_ID\"
```

#### VMs

List VMs:
```sh
obelisk client execution submit -f .../machines.list -- \
\"$FLY_APP_NAME\"
```

Launch a VM:
```sh
MACHINE_ID=$(obelisk client execution submit -f --json .../machines.create -- \
\"$FLY_APP_NAME\" \"$FLY_MACHINE_NAME\" "$(./scripts/fly-http-machine-config.json.sh)" \"$FLY_REGION\" \
| jq -r '.[-1].ok')
```

Get the VM:
```sh
obelisk client execution submit -f .../machines.get -- \
\"$FLY_APP_NAME\" \"$MACHINE_ID\"
```

Delete the VM:
```sh
obelisk client execution submit -f .../machines.delete -- \
\"$FLY_APP_NAME\" \"$MACHINE_ID\" true
```
