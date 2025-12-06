build:
	(cd activity/fly-http/impl && cargo build --release)
	(cd webhook/fly-secrets-updater/impl && cargo build --release)

verify:
	obelisk server verify --config obelisk-local.toml
	
serve:
	obelisk server run --config obelisk-local.toml

test *args:
	cargo nextest run --workspace {{args}}
