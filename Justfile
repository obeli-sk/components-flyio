# docker
build-docker:
	(cd docker/activity-docker && cargo build --profile=release_activity)
verify-docker-local:
	obelisk server verify --ignore-missing-env-vars -c docker/obelisk-local.toml

# fly
build-fly:
	(cd fly/activity-fly-http && cargo build  --profile=release_activity)
	(cd fly/webhook-fly-secrets-updater && cargo build  --profile=release_webhook)
verify-fly-local:
	obelisk server verify --ignore-missing-env-vars -c fly/obelisk-local.toml
verify-fly-oci:
	obelisk server verify --ignore-missing-env-vars -c fly/obelisk-oci.toml


build: build-docker build-fly

verify-local: verify-docker-local verify-fly-local

verify-oci: verify-fly-oci

verify: verify-local verify-oci


test *args:
	cargo nextest run --workspace {{args}}
