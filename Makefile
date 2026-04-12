.PHONY: build build-mac build-linux check test clean

# Linux build (run inside devcontainer)
build-linux:
	cargo build -p delphai-gdext

# macOS build — run this on your Mac terminal (NOT inside devcontainer)
# Usage: make build-mac
build-mac:
	cargo build -p delphai-gdext
	mkdir -p prebuilt/macos/debug prebuilt/macos/release
	cp target/debug/libdelphai_gdext.dylib prebuilt/macos/debug/
	@echo "dylib copied to prebuilt/macos/debug/ — restart Godot to reload"

build-mac-release:
	cargo build -p delphai-gdext --release
	mkdir -p prebuilt/macos/release
	cp target/release/libdelphai_gdext.dylib prebuilt/macos/release/
	@echo "release dylib copied to prebuilt/macos/release/"

build: build-linux

check:
	cargo check

test:
	cargo test

clean:
	cargo clean
