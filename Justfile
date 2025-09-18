build:
	(cd activity/fly-http/impl && cargo build --release)
	(cd webhook-fly-secrets-updater && cargo build --release)

serve:
	obelisk server run --config obelisk-local.toml

test *args:
	cargo nextest run --workspace {{args}}
