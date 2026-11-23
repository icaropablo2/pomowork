BINARY       := pomowork
RELEASE_BIN  := target/release/$(BINARY)
WIN_TARGET   := x86_64-pc-windows-gnu

.DEFAULT_GOAL := help

.PHONY: help all build release run run-release test fmt fmt-check clippy check \
        clean windows install-windows-target reset-state linux-deps

help: ## Show available commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}'

all: check release ## Run all checks, then build the release binary

build: ## Debug build
	cargo build

release: ## Optimized build
	cargo build --release

run: ## Run in debug mode
	cargo run

run-release: release ## Build and run the release binary
	$(RELEASE_BIN)

test: ## Run unit tests
	cargo test

fmt: ## Format the code with rustfmt
	cargo fmt

fmt-check: ## Verify formatting without changing files
	cargo fmt --check

clippy: ## Lint with warnings treated as errors
	cargo clippy --all-targets -- -D warnings

check: fmt-check clippy test ## Run formatting check, linter, and tests

clean: ## Remove build artifacts
	cargo clean

windows: ## Cross-compile a Windows .exe (requires mingw-w64 linker)
	cargo build --release --target $(WIN_TARGET)

install-windows-target: ## Install the Windows GNU Rust target
	rustup target add $(WIN_TARGET)

linux-deps: ## Install native build dependencies on Debian/Ubuntu (needs sudo)
	sudo apt-get update
	sudo apt-get install -y libxkbcommon-dev libwayland-dev libx11-dev libxcb1-dev \
		libxrandr-dev libxi-dev libgl1-mesa-dev libudev-dev

reset-state: ## Delete the saved session (Linux/macOS; on Windows delete %APPDATA%\pomowork\state.json)
	rm -f "$${XDG_DATA_HOME:-$$HOME/.local/share}/pomowork/state.json"
