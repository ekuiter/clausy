# This Makefile sets up clausy in the build/ directory, which includes all dependencies necessary for distribution.
# It can also be used to run tests or generate documentation.

.PHONY: clean test doc doc-live

SRC_FILES := $(filter-out $(wildcard src/external/ src/io/),$(wildcard src/* src/*/* .cargo/* Cargo.*))
CMD_NOT_FOUND = $(error Required command $(1) could not be found, please install it)
CHECK_CMD = $(if $(shell command -v $(1)),,$(call CMD_NOT_FOUND,$(1)))
CHECK_CARGO = if ! command -v cargo; then \
	curl https://sh.rustup.rs -sSf | sh; \
	source "$HOME/.cargo/env"; \
fi

clausy: build/clausy

external:
	$(MAKE) -C src/external

io:
	$(MAKE) -C src/io

build/clausy: $(SRC_FILES) io
	$(call CHECK_CMD,cc)
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo build --release
	mkdir -p build
	cp target/release/clausy build/clausy

clean:
	rm -rf build

test:
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo test

doc:
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo doc --no-deps --open

# this only works on Linux
doc-live:
	# sudo apt-get update
	# sudo apt-get install -y inotify-tools nodejs npm
	# npm install -g browser-sync
	$(call CHECK_CMD,inotifywait)
	$(call CHECK_CMD,browser-sync)
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	while inotifywait -re close_write,moved_to,create src; do \
		cargo doc --no-deps; \
	done &
	cd target/doc && browser-sync start --server --files "*.html"