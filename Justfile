build:
	(cd fly-http && cargo build --release)

serve:
	obelisk server run --config obelisk-local.toml

test:
	cargo nextest run
