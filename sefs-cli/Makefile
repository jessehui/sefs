######## SGX SDK Settings ########

SGX_SDK ?= /opt/intel/sgxsdk
SGX_MODE ?= HW
SGX_ARCH ?= x64
RUST_SGX_SDK = ../../../..
DEBUG ?= false

######## CUSTOM Settings ########

CUSTOM_LIBRARY_PATH := ./lib
CUSTOM_BIN_PATH := ./bin
CUSTOM_EDL_PATH := $(RUST_SGX_SDK)/sgx_edl/edl
CUSTOM_COMMON_PATH := $(RUST_SGX_SDK)/common

######## APP Settings ########

ifeq ($(DEBUG), false)
	App_Rust_Flags := --release
	target_dir := release
else
	App_Rust_Flags :=
	target_dir := debug
endif

App_SRC_Files := $(shell find app/ -type f -name '*.rs') $(shell find app/ -type f -name 'Cargo.toml')
App_Rust_Path := ./app/target/$(target_dir)
App_Name := bin/sefs-cli

.PHONY: all
all: $(App_Name)

$(App_Name):  $(App_SRC_Files)
	cd app && SGX_SDK=$(SGX_SDK) cargo build $(App_Rust_Flags)
	@echo "Cargo  =>  $@"
	@mkdir -p ./bin
	cp $(App_Rust_Path)/sefs-cli $@

.PHONY: clean
clean:
	@rm -rf $(App_Name) lib bin
	# @cd enclave && cargo clean && rm -f Cargo.lock
	@cd app && cargo clean && rm -f Cargo.lock

