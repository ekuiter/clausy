# This Makefile compiles all supported external solvers from source.
# This is useful to include future updates, and to support arbitrary CPU architectures (Apple Silicon in particular).

BUILD_DIR := ../../build
TMP_DIR := build
CMD_NOT_FOUND = $(error Required command $(1) could not be found, please install it.)
CHECK_CMD = $(if $(shell command -v $(1)),,$(call CMD_NOT_FOUND,$(1)))

external: $(BUILD_DIR)/kissat \
	$(BUILD_DIR)/sbva_cadical.py \
	$(BUILD_DIR)/sharpsat-td \
	$(BUILD_DIR)/d4 \
	$(BUILD_DIR)/bc_minisat_all
kissat: $(BUILD_DIR)/kissat
sbva_cadical: $(BUILD_DIR)/sbva_cadical
sharpsat-td: $(BUILD_DIR)/sharpsat-td
d4: $(BUILD_DIR)/d4
bc_minisat_all: $(BUILD_DIR)/bc_minisat_all

$(BUILD_DIR)/kissat:
	rm -rf $(TMP_DIR)/kissat
	mkdir -p $(TMP_DIR)/kissat
	tar xzf KissatMabPropPrNosym.tar.gz -C $(TMP_DIR)/kissat
	$(MAKE) -C $(TMP_DIR)/kissat/KissatMabPropPrNosym/bliss
	(cd $(TMP_DIR)/kissat/KissatMabPropPrNosym; ./configure)
	$(MAKE) -C $(TMP_DIR)/kissat/KissatMabPropPrNosym
	mkdir -p $(BUILD_DIR)
	cp $(TMP_DIR)/kissat/KissatMabPropPrNosym/build/kissat $(BUILD_DIR)/kissat

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

$(BUILD_DIR)/d4:
	$(call CHECK_CMD,cmake)
	(cd d4v2; ./build.sh -s)
	mkdir -p $(BUILD_DIR)
	cp d4v2/build/d4_static $(BUILD_DIR)/d4

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