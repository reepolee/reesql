#!/usr/bin/env bash
set -euo pipefail

APP="reesql"

# ── Dependency checks ──────────────────────────────────────────

check_target() {
	local target="$1"
	if ! rustup target list --installed 2>/dev/null | grep -q "^${target}$"; then
		echo "Error: Rust target '${target}' is not installed."
		echo ""
		echo "Install it with:"
		echo "  rustup target add ${target}"
		exit 1
	fi
}

check_linux_toolchain() {
	local cc="x86_64-unknown-linux-gnu-gcc"
	if ! command -v "$cc" &>/dev/null; then
		echo "Error: Linux cross-compiler '${cc}' not found."
		echo ""
		echo "Install it with:"
		echo "  brew tap messense/macos-cross-toolchains"
		echo "  brew install x86_64-unknown-linux-gnu"
		exit 1
	fi
}

# ── Build functions ────────────────────────────────────────────

build_native() {
	cargo build --release

	cp ./target/release/$APP ./${APP}-macos-arm64

	echo "Built macOS arm64:"
	file ./${APP}-macos-arm64 | sed 's/.*: //'
}

build_intel() {
	check_target "x86_64-apple-darwin"

	cargo build --release --target x86_64-apple-darwin

	cp ./target/x86_64-apple-darwin/release/$APP ./${APP}-macos-x64

	echo "Built macOS x64:"
	file ./${APP}-macos-x64 | sed 's/.*: //'
}

build_linux() {
	check_target "x86_64-unknown-linux-gnu"
	check_linux_toolchain

	cargo build --release --target x86_64-unknown-linux-gnu

	cp ./target/x86_64-unknown-linux-gnu/release/$APP \
		./${APP}-linux-x64

	echo "Built Linux x64:"
	file ./${APP}-linux-x64 | sed 's/.*: //'
}

build_universal() {
	build_native
	build_intel

	lipo -create \
		-output ./${APP}-macos-universal \
		./${APP}-macos-arm64 \
		./${APP}-macos-x64

	echo "Built macOS universal:"
	lipo -info ./${APP}-macos-universal | sed 's/.*: //'
}

show_outputs() {
	echo "---"
	echo "Build outputs:"
	ls -lh ./${APP}-*
}

# ── Entry point ────────────────────────────────────────────────

case "${1:-native}" in
	native)
		build_native
		;;

	intel)
		build_intel
		;;

	universal)
		build_universal
		;;

	linux)
		build_linux
		;;

	all)
		build_universal
		build_linux
		show_outputs
		;;

	*)
		echo "Usage: $0 {native|intel|universal|linux|all}"
		exit 1
		;;
esac
