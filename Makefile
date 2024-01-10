help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sed -n 's/^\(.*\): \(.*\)##\(.*\)/\1\t\3/p' | column -t -s $$'\t' | sort

build: ## Build library for release
	cargo build --release
.PHONY: build

cbindgen: ## Generate C header file with bindings
	cbindgen --config cbindgen.toml --crate hiallib --output hiallib.h
.PHONY: cbindgen
