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

bin/sharpsat-td:
	$(call CHECK_CMD,cmake)
	mkdir -p build
	cp -R lib/sharpsat-td build/
	mkdir -p build/bin
	cp lib/sse2neon/sse2neon.h build/src/clhash/
	(cd build; ./setupdev.sh static) || ( \
		sed -i='' 's/ -msse4.1 -mpclmul//' build/CMakeLists.txt && \
		(cd build; ./setupdev.sh static) \
	)
	mkdir -p bin
	mv build/bin/* bin/
	rm -rf build

bin/d4:
	$(call CHECK_CMD,cmake)
	mkdir -p d4v2-cc730adb
	tar xzf lib/d4v2-cc730adb.tar.gz -C d4v2-cc730adb
	cp lib/sse2neon/sse2neon.h d4v2-cc730adb/3rdParty/kahypar/kahypar/utils/
	cp lib/sse2neon/sse2neon.h d4v2-cc730adb/3rdParty/kahypar/external_tools/WHFC/util/
	(cd d4v2-cc730adb; ./build.sh) || ( \
		sed -i='' 's/defined(__linux__)/defined(__linux__) \&\& defined(_FPU_EXTENDED) \&\& defined(_FPU_DOUBLE) \&\& defined(_FPU_GETCW)/' d4v2-cc730adb/3rdParty/glucose-3.0/core/Main.cc && \
		sed -i='' 's/#include <x86intrin.h>/#include "sse2neon.h"/' d4v2-cc730adb/3rdParty/kahypar/kahypar/utils/math.h && \
		sed -i='' 's/#include <emmintrin.h>/#include "sse2neon.h"/' d4v2-cc730adb/3rdParty/kahypar/external_tools/WHFC/util/meta.h && \
		(cd d4v2-cc730adb; ./build.sh) \
	)
	mkdir -p bin
	mv d4v2-cc730adb/build/* bin/
	rm -rf d4v2-cc730adb

bin/bc_minisat_all:
	$(call CHECK_CMD,curl)
	$(call CHECK_CMD,tar)
	$(call CHECK_CMD,sed)
	$(call CHECK_CMD,cc)
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

bin/clausy: $(SRC_FILES) bin/kissat_MAB-HyWalk bin/sharpsat-td bin/d4 bin/bc_minisat_all bin/io.jar
	$(call CHECK_CMD,cc)
	$(call CHECK_CMD,curl)
	$(call CHECK_CARGO)
	cargo build --release
	cp target/release/clausy bin/clausy