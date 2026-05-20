SHELL := /usr/bin/env bash

.DEFAULT_GOAL := help

BLADE_DIR ?= ../blade
ANDROID_TARGET ?= aarch64-linux-android
ANDROID_API ?= 24

# Prefer explicit ANDROID_HOME from environment, otherwise try common locations.
ANDROID_HOME ?= $(firstword $(wildcard /usr/lib/android-sdk $(HOME)/Android/Sdk $(HOME)/Android/sdk))
ANDROID_SDK_ROOT ?= $(ANDROID_HOME)
ANDROID_NDK_ROOT ?= $(firstword $(wildcard $(ANDROID_HOME)/ndk/*))

export ANDROID_HOME
export ANDROID_SDK_ROOT
export ANDROID_NDK_ROOT
export PATH := $(if $(ANDROID_HOME),$(ANDROID_HOME)/platform-tools:,)$(PATH)

.PHONY: help doctor setup-rust-android setup-android check-blade check-rust check-rust-android check-android-sdk check-cargo-apk check-adb
.PHONY: run build build-logic build-android apk-build apk-run

help:
	@echo "interact build commands"
	@echo ""
	@echo "Desktop:"
	@echo "  make run           Run desktop app"
	@echo "  make build         Build whole workspace"
	@echo "  make build-logic   Build hot-reload logic crate"
	@echo ""
	@echo "Android:"
	@echo "  make build-android Build Android target (Rust compile check)"
	@echo "  make apk-build     Build APK via .android-runner"
	@echo "  make apk-run       Build + install + launch on connected device"
	@echo ""
	@echo "Diagnostics:"
	@echo "  make doctor        Print environment/tooling status"
	@echo ""
	@echo "Setup:"
	@echo "  make setup-rust-android  Install Rust Android target"
	@echo "  make setup-android       Install Rust target + cargo-apk"

setup-rust-android: check-rust
	rustup target add $(ANDROID_TARGET)

setup-android: setup-rust-android
	cargo install cargo-apk

doctor:
	@echo "BLADE_DIR=$(BLADE_DIR)"
	@echo "ANDROID_HOME=$(ANDROID_HOME)"
	@echo "ANDROID_SDK_ROOT=$(ANDROID_SDK_ROOT)"
	@echo "ANDROID_NDK_ROOT=$(ANDROID_NDK_ROOT)"
	@echo "ANDROID_TARGET=$(ANDROID_TARGET)"
	@echo ""
	@command -v cargo >/dev/null && echo "cargo: OK" || echo "cargo: MISSING"
	@command -v rustup >/dev/null && echo "rustup: OK" || echo "rustup: MISSING"
	@command -v cargo-apk >/dev/null && echo "cargo-apk: OK" || echo "cargo-apk: MISSING (run: cargo install cargo-apk)"
	@command -v adb >/dev/null && echo "adb: OK" || echo "adb: MISSING (install Android platform-tools)"
	@if [[ -n "$(ANDROID_HOME)" && -d "$(ANDROID_HOME)" ]]; then echo "sdk dir: OK"; else echo "sdk dir: MISSING"; fi
	@if [[ -n "$(ANDROID_HOME)" && -d "$(ANDROID_HOME)/build-tools" ]]; then echo "build-tools dir: OK"; else echo "build-tools dir: MISSING"; fi
	@if [[ -n "$(ANDROID_HOME)" && -d "$(ANDROID_HOME)/platforms" ]]; then echo "platforms dir: OK"; else echo "platforms dir: MISSING"; fi
	@if [[ -n "$(ANDROID_NDK_ROOT)" && -d "$(ANDROID_NDK_ROOT)" ]]; then echo "ndk dir: OK"; else echo "ndk dir: MISSING"; fi

check-blade:
	@test -d "$(BLADE_DIR)/src/blade-engine" || (echo "Missing blade repo at $(BLADE_DIR). Clone https://github.com/nergnezor/blade there." && exit 1)

check-rust:
	@command -v cargo >/dev/null || (echo "cargo not found. Install Rust via rustup first." && exit 1)
	@command -v rustup >/dev/null || (echo "rustup not found. Install Rust via rustup first." && exit 1)

check-rust-android: check-rust
	@rustup target list --installed | grep -q '^$(ANDROID_TARGET)$$' || (echo "Rust target $(ANDROID_TARGET) not installed. Run: rustup target add $(ANDROID_TARGET)" && exit 1)

check-android-sdk:
	@test -n "$(ANDROID_HOME)" || (echo "ANDROID_HOME is empty. Set it or install SDK in /usr/lib/android-sdk or ~/Android/Sdk." && exit 1)
	@test -d "$(ANDROID_HOME)" || (echo "Android SDK not found at $(ANDROID_HOME)" && exit 1)
	@test -d "$(ANDROID_HOME)/platform-tools" || (echo "Missing platform-tools in $(ANDROID_HOME)." && exit 1)
	@test -d "$(ANDROID_HOME)/build-tools" || (echo "Missing build-tools in $(ANDROID_HOME). Install package: android-sdk-build-tools." && exit 1)
	@test -d "$(ANDROID_HOME)/platforms" || (echo "Missing platforms in $(ANDROID_HOME). Install package: android-sdk-platform-23 (or newer)." && exit 1)
	@test -n "$(ANDROID_NDK_ROOT)" || (echo "No NDK found under $(ANDROID_HOME)/ndk" && exit 1)
	@test -d "$(ANDROID_NDK_ROOT)" || (echo "NDK path not found: $(ANDROID_NDK_ROOT)" && exit 1)

check-cargo-apk:
	@command -v cargo-apk >/dev/null || (echo "cargo-apk not found. Run: cargo install cargo-apk" && exit 1)

check-adb:
	@command -v adb >/dev/null || (echo "adb not found. Install Android platform-tools." && exit 1)

run: check-blade
	cargo run

build: check-blade
	cargo build --workspace

build-logic: check-blade
	cargo build -p interact-logic

build-android: check-blade check-rust-android check-android-sdk
	cargo build -p interact --target $(ANDROID_TARGET)

apk-build: check-blade check-rust-android check-android-sdk check-cargo-apk
	cd .android-runner && cargo apk build

apk-run: check-blade check-rust-android check-android-sdk check-cargo-apk check-adb
	adb get-state >/dev/null
	cd .android-runner && cargo apk run
