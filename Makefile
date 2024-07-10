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

all: lib clausy

lib: bin/kissat bin/sbva_cadical bin/sharpsat-td bin/d4 bin/bc_minisat_all
clausy: bin/clausy

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

bin/kissat:
	rm -rf build
	mkdir -p build
	tar xzf lib/KissatMabPropPrNosym.tar.gz -C build
	$(MAKE) -C build/KissatMabPropPrNosym/bliss
	(cd build/KissatMabPropPrNosym; ./configure)
	$(MAKE) -C build/KissatMabPropPrNosym
	mkdir -p bin
	mv build/KissatMabPropPrNosym/build/kissat bin/kissat
	rm -rf build

bin/sbva_cadical:
	rm -rf build
	mkdir -p build
	tar xzf lib/sbva_cadical.tar.gz -C build
	(cd build/sbva_cadical/archives/cadical-rel-1.5.3; ./configure; make)
	$(MAKE) -C build/sbva_cadical/src
	mkdir -p bin
	mv build/sbva_cadical/archives/cadical-rel-1.5.3/build/cadical bin/cadical
	mv build/sbva_cadical/src/bva bin/bva
	mv build/sbva_cadical/bin/sbva_cadical.py bin/sbva_cadical.py
	rm -rf build

bin/sharpsat-td:
	$(call CHECK_CMD,cmake)
	rm -rf build
	cp -R lib/sharpsat-td build
	mkdir -p build/bin
	cp lib/sse2neon/sse2neon.h build/src/clhash/
	(cd build; ./setupdev.sh static) || ( \
		sed -i='' 's/ -msse4.1 -mpclmul//' build/CMakeLists.txt && \
		(cd build; ./setupdev.sh static) \
	)
	mkdir -p bin
	mv build/bin/* bin/
	mv bin/sharpSAT bin/sharpsat-td
	rm -rf build

bin/d4:
	$(call CHECK_CMD,cmake)
	(cd lib/d4v2; ./build.sh -s)
	mkdir -p bin
	mv lib/d4v2/build/d4_static bin/d4

bin/bc_minisat_all:
	$(call CHECK_CMD,curl)
	$(call CHECK_CMD,tar)
	$(call CHECK_CMD,sed)
	$(call CHECK_CMD,cc)
	rm -rf build
	mkdir -p build
	tar xzf lib/bc_minisat_all-1.1.2.tar.gz -C build
	sed -i='' 's/out = NULL;/s->out = stderr;/' build/bc_minisat_all-1.1.2/main.c
	$(MAKE) -C build/bc_minisat_all-1.1.2 rs || $(MAKE) -C build/bc_minisat_all-1.1.2 r
	mkdir -p bin
	mv build/bc_minisat_all-1.1.2/bc_minisat_all_* bin/bc_minisat_all
	rm -rf build

bin/io.jar:
	$(call CHECK_CMD,java)
	mkdir -p bin
	io/gradlew -p io shadowJar

bin/clausy: $(SRC_FILES) bin/io.jar
	$(call CHECK_CMD,cc)
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo build --release
	mkdir -p bin
	cp target/release/clausy bin/clausy