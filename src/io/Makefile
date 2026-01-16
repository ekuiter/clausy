# This Makefile compiles the input/output interface between clausy and FeatureIDE.

# same path as in build.gradle
BUILD_DIR := ../../build
CMD_NOT_FOUND = $(error Required command $(1) could not be found, please install it.)
CHECK_CMD = $(if $(shell command -v $(1)),,$(call CMD_NOT_FOUND,$(1)))

io: $(BUILD_DIR)/io.jar

$(BUILD_DIR)/io.jar:
	$(call CHECK_CMD,java)
	mkdir -p $(BUILD_DIR)
	./gradlew shadowJar