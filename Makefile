.PHONY: cleanup
cleanup: fix clippy fmt

.PHONY: test
test:
	cargo test

.PHONY: clippy
clippy:
	cargo clippy --all-targets -- -D warnings

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: fix
fix:
	cargo fix --allow-staged --allow-dirty --all-targets
	cargo clippy --fix --allow-staged --allow-dirty --all-targets
	cargo fmt

.PHONY: local
local:
	cargo run --bin lemon -- run local_config.toml
