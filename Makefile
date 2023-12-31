# This Makefile sets up clausy in the bin/ directory, which includes all dependencies necessary for distribution.
# It can also be used to run tests or generate documentation.

.PHONY: clean test doc doc-live

SRC_FILES := $(wildcard src/* src/*/* .cargo/* Cargo.*)
CMD_NOT_FOUND = $(error Required command $(1) could not be found, please install it)
CHECK_CMD = $(if $(shell command -v $(1)),,$(call CMD_NOT_FOUND,$(1)))
CHECK_CARGO = if ! command -v cargo; then \
	curl https://sh.rustup.rs -sSf | sh; \
	source "$HOME/.cargo/env"; \
fi

all: bin/clausy

clean:
	rm -rf bin

test:
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo test

doc:
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo doc --no-deps --open

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

bin/kissat_MAB-HyWalk:
	$(call CHECK_CMD,curl)
	mkdir -p bin
	curl https://github.com/ekuiter/torte/raw/main/docker/solver/other/kissat_MAB-HyWalk -Lo bin/kissat_MAB-HyWalk
	chmod +x bin/kissat_MAB-HyWalk

bin/d4:
	$(call CHECK_CMD,curl)
	mkdir -p bin
	curl https://github.com/ekuiter/torte/raw/main/docker/solver/model-counting-competition-2022/d4 -Lo bin/d4
	chmod +x bin/d4

bin/bc_minisat_all_static:
	$(call CHECK_CMD,curl)
	$(call CHECK_CMD,tar)
	$(call CHECK_CMD,sed)
	$(call CHECK_CMD,cc)
	curl http://www.sd.is.uec.ac.jp/toda/code/bc_minisat_all-1.1.2.tar.gz -Lo bc_minisat_all-1.1.2.tar.gz
	tar xzvf bc_minisat_all-1.1.2.tar.gz
	rm -f bc_minisat_all-1.1.2.tar.gz
	sed -i 's/out = NULL;/s->out = stderr;/' bc_minisat_all-1.1.2/main.c
	$(MAKE) -C bc_minisat_all-1.1.2 rs
	mkdir -p bin
	mv bc_minisat_all-1.1.2/bc_minisat_all_static bin/
	rm -rf bc_minisat_all-1.1.2

bin/io.jar:
	$(call CHECK_CMD,java)
	mkdir -p bin
	io/gradlew -p io shadowJar

bin/clausy: $(SRC_FILES) bin/kissat_MAB-HyWalk bin/d4 bin/bc_minisat_all_static bin/io.jar
	$(call CHECK_CMD,cc)
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo build --release
	cp target/release/clausy bin/clausy