.PHONY: build build-mac build-linux check test smoke-citizen clean

GODOT ?= godot

# Linux build (run inside devcontainer)
build-linux:
	cargo build -p delphai-gdext

# macOS build — run this on your Mac terminal (NOT inside devcontainer)
# Usage: make build-mac
build-mac:
	cargo build -p delphai-gdext
	mkdir -p prebuilt/macos/debug prebuilt/macos/release
	cp target/debug/libdelphai_gdext.dylib prebuilt/macos/debug/
	codesign --force --sign - prebuilt/macos/debug/libdelphai_gdext.dylib
	@echo "dylib copied and ad-hoc signed — restart Godot to reload"

build-mac-release:
	cargo build -p delphai-gdext --release
	mkdir -p prebuilt/macos/release
	cp target/release/libdelphai_gdext.dylib prebuilt/macos/release/
	codesign --force --sign - prebuilt/macos/release/libdelphai_gdext.dylib
	@echo "release dylib copied and ad-hoc signed"

build: build-linux

check:
	cargo check

test:
	cargo test

# Sprint N4 smoke: headless で住民が連続して動くことを確認（マス目移動regression guard）
# 事前に make build（or build-mac）を済ませておくこと。
smoke-citizen:
	@command -v $(GODOT) >/dev/null 2>&1 || { echo "godot binary not found; set GODOT=/path/to/godot"; exit 1; }
	cd game && $(GODOT) --headless --script res://scripts/tests/smoke_citizen_motion.gd

clean:
	cargo clean
