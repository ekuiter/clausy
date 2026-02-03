# This Makefile compiles several supported external solvers from source.
# This is useful to include future updates, and to support arbitrary CPU architectures (Apple Silicon in particular).

BUILD_DIR := ../../build
TMP_DIR := build
CMD_NOT_FOUND = $(error Required command $(1) could not be found, please install it.)
CHECK_CMD = $(if $(shell command -v $(1)),,$(call CMD_NOT_FOUND,$(1)))

external_all: $(BUILD_DIR)/tinisat \
	$(BUILD_DIR)/kissat \
	$(BUILD_DIR)/sbva_cadical.py \
	$(BUILD_DIR)/sharpsat-td \
	$(BUILD_DIR)/d4 \
	$(BUILD_DIR)/bc_minisat_all
external_linux: $(BUILD_DIR)/kissat \
	$(BUILD_DIR)/d4 \
	$(BUILD_DIR)/bc_minisat_all
external_macos: $(BUILD_DIR)/tinisat \
	$(BUILD_DIR)/d4-aarch64-darwin \
	$(BUILD_DIR)/bc_minisat_all
kissat: $(BUILD_DIR)/kissat
tinisat: $(BUILD_DIR)/tinisat
sbva_cadical: $(BUILD_DIR)/sbva_cadical.py
sharpsat-td: $(BUILD_DIR)/sharpsat-td
d4: $(BUILD_DIR)/d4
d4-aarch64-darwin: $(BUILD_DIR)/d4-aarch64-darwin
bc_minisat_all: $(BUILD_DIR)/bc_minisat_all

# used by default for satisfiability solving
# has a complex build process that only works on Linux
$(BUILD_DIR)/kissat:
	rm -rf $(TMP_DIR)/kissat
	mkdir -p $(TMP_DIR)/kissat
	tar xzf KissatMabPropPrNosym.tar.gz -C $(TMP_DIR)/kissat
	sed -i '' 's|-l:../bliss/libbliss.a|-L../bliss -lbliss|' $(TMP_DIR)/kissat/KissatMabPropPrNosym/makefile.in
	sed -i '' 's|(compiled "__DATE__")\\n", bliss::version|(compiled %s)\\n", bliss::version, __DATE__|' $(TMP_DIR)/kissat/KissatMabPropPrNosym/bliss/bliss.cc
	$(MAKE) -C $(TMP_DIR)/kissat/KissatMabPropPrNosym/bliss
	(cd $(TMP_DIR)/kissat/KissatMabPropPrNosym; ./configure)
	$(MAKE) -C $(TMP_DIR)/kissat/KissatMabPropPrNosym
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/kissat/KissatMabPropPrNosym/build/kissat $(BUILD_DIR)/kissat

# alternative for satisfiability solving
# easy to build, but not as performant as kissat and does not return a satisfying solution
$(BUILD_DIR)/tinisat:
	rm -rf $(TMP_DIR)/tinisat
	mkdir -p $(TMP_DIR)/tinisat
	tar xzf tinisat0.22.tar.gz -C $(TMP_DIR)/tinisat
	$(MAKE) -C $(TMP_DIR)/tinisat/tinisat0.22
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/tinisat/tinisat0.22/tinisat $(BUILD_DIR)/tinisat

# not currently used for anything, so we don't compile it by default
$(BUILD_DIR)/sbva_cadical.py:
	rm -rf $(TMP_DIR)/sbva_cadical
	mkdir -p $(TMP_DIR)/sbva_cadical
	tar xzf sbva_cadical.tar.gz -C $(TMP_DIR)/sbva_cadical
	(cd $(TMP_DIR)/sbva_cadical/sbva_cadical/archives/cadical-rel-1.5.3; ./configure; make)
	$(MAKE) -C $(TMP_DIR)/sbva_cadical/sbva_cadical/src
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/sbva_cadical/sbva_cadical/archives/cadical-rel-1.5.3/build/cadical $(BUILD_DIR)/cadical
	cp $(TMP_DIR)/sbva_cadical/sbva_cadical/src/bva $(BUILD_DIR)/bva
	cp $(TMP_DIR)/sbva_cadical/sbva_cadical/bin/sbva_cadical.py $(BUILD_DIR)/sbva_cadical.py

# not currently used for anything, so we don't compile it by default
$(BUILD_DIR)/sharpsat-td:
	$(call CHECK_CMD,cmake)
	rm -rf $(TMP_DIR)/sharpsat-td
	cp -R sharpsat-td $(TMP_DIR)/sharpsat-td
	mkdir -p $(TMP_DIR)/sharpsat-td/bin
	cp sse2neon/sse2neon.h $(TMP_DIR)/sharpsat-td/src/clhash/
	(cd $(TMP_DIR)/sharpsat-td; ./setupdev.sh static) || ( \
		sed -i='' 's/ -msse4.1 -mpclmul//' $(TMP_DIR)/sharpsat-td/CMakeLists.txt && \
		(cd $(TMP_DIR)/sharpsat-td; ./setupdev.sh static) \
	)
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/sharpsat-td/bin/sharpSAT $(BUILD_DIR)/sharpsat-td

# used by default for model counting
# has a complex build process that only works on Linux
$(BUILD_DIR)/d4:
	$(call CHECK_CMD,cmake)
	(cd d4v2; ./build.sh -s)
	mkdir -p $(BUILD_DIR)
	cp d4v2/build/d4_static $(BUILD_DIR)/d4

# used for model counting on macOS
# this just unzips a precompiled binary which works on Apple Silicon
$(BUILD_DIR)/d4-aarch64-darwin:
	rm -rf $(TMP_DIR)/d4-aarch64-darwin
	mkdir -p $(TMP_DIR)/d4-aarch64-darwin
	tar xzf d4-aarch64-darwin.tar.gz -C $(TMP_DIR)/d4-aarch64-darwin
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/d4-aarch64-darwin/bin/d4 $(BUILD_DIR)/d4-aarch64-darwin
	cp -R $(TMP_DIR)/d4-aarch64-darwin/lib $(BUILD_DIR)/lib_d4
	printf '#!/bin/sh\nDYLD_LIBRARY_PATH="$$(dirname "$$0")/lib_d4" "$$(dirname "$$0")/d4-aarch64-darwin" "$$@"\n' > $(BUILD_DIR)/d4
	chmod +x $(BUILD_DIR)/d4

# used by default for model enumeration
$(BUILD_DIR)/bc_minisat_all:
	$(call CHECK_CMD,curl)
	$(call CHECK_CMD,tar)
	$(call CHECK_CMD,sed)
	$(call CHECK_CMD,cc)
	rm -rf $(TMP_DIR)/bc_minisat_all
	mkdir -p $(TMP_DIR)/bc_minisat_all
	tar xzf bc_minisat_all-1.1.2.tar.gz -C $(TMP_DIR)/bc_minisat_all
	sed -i='' 's/out = NULL;/s->out = stderr;/' $(TMP_DIR)/bc_minisat_all/bc_minisat_all-1.1.2/main.c
	$(MAKE) -C $(TMP_DIR)/bc_minisat_all/bc_minisat_all-1.1.2 rs || $(MAKE) -C $(TMP_DIR)/bc_minisat_all/bc_minisat_all-1.1.2 r
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/bc_minisat_all/bc_minisat_all-1.1.2/bc_minisat_all_* $(BUILD_DIR)/bc_minisat_all