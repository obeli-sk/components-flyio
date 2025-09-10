# Obelisk Activities for interacting with fly.io

## fly-http
Activity that uses the official [Fly Machines API](https://fly.io/docs/machines/api/) to interact with Apps and VMs.

Check out the [WIT definition](fly-http/wit/activity-flyio_fly-http@1.0.0-beta/fly.wit).

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
obelisk client execution submit -f activity-flyio:fly-http/app@1.0.0-beta.list -- \
\"$FLY_ORG_SLUG\"
```

Create an app:
```sh
obelisk client execution submit -f activity-flyio:fly-http/app@1.0.0-beta.create -- \
\"$FLY_ORG_SLUG\" \"$FLY_APP_NAME\"
```

List secrets of the app:
```sh
obelisk client execution submit -f  activity-flyio:fly-http/secret@1.0.0-beta.list -- \
\"$FLY_APP_NAME\"
```

Launch a VM:
```sh
MACHINE_ID=$(obelisk client execution submit -f --json activity-flyio:fly-http/machine@1.0.0-beta.create -- \
\"$FLY_APP_NAME\" \"$FLY_MACHINE_NAME\" "$(./scripts/fly-http-machine-config.json.sh)" | jq -r '.[-1].ok.ok')
```

Delete the VM:
```sh
obelisk client execution submit -f activity-flyio:fly-http/machine@1.0.0-beta.delete -- \
\"$FLY_APP_NAME\" \"$MACHINE_ID\" true
```
