# Par Fractal Makefile
# Cross-platform GPU-accelerated fractal renderer

.PHONY: help build build-release run run-release test check clean clippy clippy-fix fmt fmt-check \
        install-deps doc checkall pre-commit-install pre-commit-uninstall pre-commit-run \
        pre-commit-update lint deploy release cache-clean

# Default target
.DEFAULT_GOAL := help

help:
	@echo "==================================================================="
	@echo "  par-fractal Makefile"
	@echo "==================================================================="
	@echo ""
	@echo "  GPU-accelerated fractal renderer with 2D and 3D support"
	@echo ""
	@echo "==================================================================="
	@echo ""
	@echo "Building:"
	@echo "  build              - Build the application in debug mode"
	@echo "  build-release      - Build the application in release mode (optimized)"
	@echo "  release-build      - Build optimized release binary with info"
	@echo ""
	@echo "Running:"
	@echo "  run                - Run the application in debug mode"
	@echo "  run-release        - Run the application in release mode (recommended)"
	@echo "  r                  - Shortcut for run-release"
	@echo "  run-reset          - Run in release mode with settings cleared"
	@echo ""
	@echo "Testing:"
	@echo "  test               - Run all tests"
	@echo "  test-verbose       - Run tests with verbose output"
	@echo "  t                  - Shortcut for test"
	@echo ""
	@echo "Code Quality:"
	@echo "  fmt                - Format Rust code"
	@echo "  f                  - Shortcut for fmt"
	@echo "  lint               - Run Rust linters (clippy + fmt, auto-fix)"
	@echo "  clippy             - Run clippy (Rust linter)"
	@echo "  clippy-fix         - Run clippy and automatically fix issues"
	@echo "  check              - Check the code for errors without building"
	@echo "  c                  - Shortcut for check"
	@echo "  checkall           - Run ALL checks: format, lint, tests (auto-fix all)"
	@echo ""
	@echo "Pre-commit Hooks:"
	@echo "  pre-commit-install   - Install pre-commit hooks"
	@echo "  pre-commit-uninstall - Uninstall pre-commit hooks"
	@echo "  pre-commit-run       - Run pre-commit on all files"
	@echo "  pre-commit-update    - Update pre-commit hook versions"
	@echo ""
	@echo "Documentation:"
	@echo "  doc                - Generate and open documentation"
	@echo "  doc-all            - Generate documentation including dependencies"
	@echo ""
	@echo "Deployment:"
	@echo "  release            - Trigger GitHub release pipeline (with confirmation)"
	@echo "  deploy             - Trigger GitHub 'Release and Deploy' workflow"
	@echo ""
	@echo "System:"
	@echo "  install-deps       - Install required system dependencies (Linux only)"
	@echo "  install            - Install the binary to ~/.cargo/bin"
	@echo "  uninstall          - Uninstall the binary from ~/.cargo/bin"
	@echo "  clean              - Clean build artifacts"
	@echo "  cache-clean        - Clean Cargo cache and build artifacts"
	@echo ""
	@echo "Development Tools:"
	@echo "  watch              - Watch for changes and rebuild"
	@echo "  watch-run          - Watch for changes and re-run"
	@echo "  audit              - Audit dependencies for security vulnerabilities"
	@echo "  bloat              - Analyze binary size"
	@echo "  update             - Update dependencies"
	@echo ""
	@echo "==================================================================="

# ============================================================================
# Building
# ============================================================================

build:
	@echo "Building in debug mode..."
	cargo build

build-release:
	@echo "Building in release mode..."
	cargo build --release

release-build:
	@echo "Building optimized release binary..."
	cargo build --release
	@echo ""
	@echo "======================================================================"
	@echo "  Release binary built successfully!"
	@echo "======================================================================"
	@echo ""
	@ls -lh target/release/par-fractal
	@echo ""

# ============================================================================
# Running
# ============================================================================

run:
	@echo "Running in debug mode..."
	cargo run

run-release:
	@echo "Running in release mode..."
	cargo run --release

run-reset:
	@echo "Running with cleared settings..."
	cargo run --release -- --clear-settings

# ============================================================================
# Testing
# ============================================================================

test:
	@echo "Running tests..."
	cargo test --verbose

test-verbose:
	@echo "Running tests (verbose)..."
	cargo test -- --nocapture

# ============================================================================
# Code Quality
# ============================================================================

fmt:
	@echo "Formatting code..."
	cargo fmt

fmt-check:
	@echo "Checking code formatting..."
	cargo fmt -- --check

lint:
	@echo "Running Rust linters and auto-fixing issues..."
	cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
	cargo fmt
	@echo ""
	@echo "======================================================================"
	@echo "  Lint complete! (auto-fixed)"
	@echo "======================================================================"
	@echo ""

clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

clippy-fix:
	@echo "Running clippy with automatic fixes..."
	cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings

check:
	@echo "Checking code..."
	cargo check

checkall: test lint
	@echo ""
	@echo "======================================================================"
	@echo "  All code quality checks passed!"
	@echo "======================================================================"
	@echo ""
	@echo "Summary:"
	@echo "  ✓ Rust tests"
	@echo "  ✓ Rust format (auto-fixed)"
	@echo "  ✓ Rust lint (clippy auto-fixed)"
	@echo ""

# ============================================================================
# Pre-commit Hooks
# ============================================================================

pre-commit-install:
	@echo "Installing pre-commit hooks..."
	@if ! command -v pre-commit > /dev/null; then \
		echo "Error: pre-commit not found. Install it with:"; \
		echo "  pip install pre-commit"; \
		echo "  # or"; \
		echo "  brew install pre-commit  (on macOS)"; \
		exit 1; \
	fi
	pre-commit install
	@echo ""
	@echo "======================================================================"
	@echo "  Pre-commit hooks installed successfully!"
	@echo "======================================================================"
	@echo ""
	@echo "Hooks will now run automatically on 'git commit'."
	@echo "To run hooks manually: make pre-commit-run"
	@echo "To skip hooks on commit: git commit --no-verify"
	@echo ""

pre-commit-uninstall:
	@echo "Uninstalling pre-commit hooks..."
	pre-commit uninstall
	@echo "Pre-commit hooks uninstalled."

pre-commit-run:
	@echo "Running pre-commit on all files..."
	pre-commit run --all-files

pre-commit-update:
	@echo "Updating pre-commit hook versions..."
	pre-commit autoupdate
	@echo ""
	@echo "Hook versions updated. Review changes in .pre-commit-config.yaml"

# ============================================================================
# Documentation
# ============================================================================

doc:
	@echo "Generating documentation..."
	cargo doc --open --no-deps

doc-all:
	@echo "Generating documentation with dependencies..."
	cargo doc --open

# ============================================================================
# Deployment
# ============================================================================

deploy:
	@echo "======================================================================"
	@echo "  Triggering GitHub 'Release and Deploy' workflow"
	@echo "======================================================================"
	@echo ""
	@if ! command -v gh > /dev/null; then \
		echo "Error: GitHub CLI (gh) not found. Install it from:"; \
		echo "  https://cli.github.com/"; \
		exit 1; \
	fi
	gh workflow run release.yml
	@echo ""
	@echo "Workflow triggered successfully!"
	@echo "Monitor progress at:"
	@echo "  https://github.com/paulrobello/par-fractal/actions"
	@echo ""
	@echo "Or use: gh run list --workflow=release.yml"
	@echo ""

release:
	@echo "======================================================================"
	@echo "  Triggering GitHub Release Pipeline"
	@echo "======================================================================"
	@echo ""
	@echo "This will:"
	@echo "  1. Build binaries for Linux, macOS, and Windows"
	@echo "  2. Publish to crates.io"
	@echo "  3. Create a GitHub release with all artifacts"
	@echo ""
	@if ! command -v gh > /dev/null; then \
		echo "Error: GitHub CLI (gh) not found. Install it from:"; \
		echo "  https://cli.github.com/"; \
		exit 1; \
	fi
	@read -p "Are you sure you want to trigger a release? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		gh workflow run release.yml; \
		echo ""; \
		echo "Release workflow triggered successfully!"; \
		echo "Monitor progress at:"; \
		echo "  https://github.com/paulrobello/par-fractal/actions"; \
		echo ""; \
		echo "Or use: gh run list --workflow=release.yml"; \
	else \
		echo "Release cancelled."; \
	fi
	@echo ""

# ============================================================================
# System
# ============================================================================

install-deps:
	@echo "Installing system dependencies..."
	@if [ "$$(uname)" = "Linux" ]; then \
		if command -v apt-get > /dev/null; then \
			sudo apt-get update && sudo apt-get install -y \
				libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
				libxkbcommon-dev libwayland-dev libssl-dev pkg-config; \
		elif command -v dnf > /dev/null; then \
			sudo dnf install -y \
				libxcb-devel libxkbcommon-devel wayland-devel openssl-devel pkg-config; \
		elif command -v pacman > /dev/null; then \
			sudo pacman -S --noconfirm \
				libxcb libxkbcommon wayland openssl pkg-config; \
		else \
			echo "Unknown package manager. Please install dependencies manually."; \
		fi; \
	else \
		echo "Not on Linux. Skipping system dependency installation."; \
	fi

install: build-release
	@echo "Installing to ~/.cargo/bin..."
	cargo install --path .
	@echo ""
	@echo "======================================================================"
	@echo "  Installed successfully!"
	@echo "======================================================================"
	@echo ""
	@echo "Run with: par-fractal"
	@echo ""

uninstall:
	@echo "Uninstalling..."
	cargo uninstall par-fractal

clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/
	@echo "Clean complete!"

cache-clean:
	@echo "Cleaning Cargo cache and build artifacts..."
	cargo clean
	rm -rf target/
	rm -rf ~/.cargo/registry/cache
	rm -rf ~/.cargo/git/db
	@echo "Cache clean complete!"

# ============================================================================
# Development Tools
# ============================================================================

watch:
	@echo "Watching for changes..."
	@if ! command -v cargo-watch > /dev/null; then \
		echo "Installing cargo-watch..."; \
		cargo install cargo-watch; \
	fi
	cargo watch -x check -x test

watch-run:
	@echo "Watching and running..."
	@if ! command -v cargo-watch > /dev/null; then \
		echo "Installing cargo-watch..."; \
		cargo install cargo-watch; \
	fi
	cargo watch -x 'run --release'

update:
	@echo "Updating dependencies..."
	cargo update

audit:
	@echo "Auditing dependencies..."
	@if ! command -v cargo-audit > /dev/null; then \
		echo "Installing cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	cargo audit

bloat:
	@echo "Analyzing binary size..."
	@if ! command -v cargo-bloat > /dev/null; then \
		echo "Installing cargo-bloat..."; \
		cargo install cargo-bloat; \
	fi
	cargo bloat --release

bench:
	@echo "Running benchmarks..."
	cargo bench

profile: build-release
	@echo "Running with profiling..."
	@if command -v perf > /dev/null; then \
		perf record --call-graph=dwarf ./target/release/par-fractal; \
		perf report; \
	else \
		echo "perf not found. Install linux-tools package."; \
	fi

# ============================================================================
# Platform-specific targets
# ============================================================================

run-linux:
	@echo "Running with Vulkan (Linux)..."
	WGPU_BACKEND=vulkan cargo run --release

run-macos:
	@echo "Running with Metal (macOS)..."
	WGPU_BACKEND=metal cargo run --release

run-windows-dx12:
	@echo "Running with DirectX 12 (Windows)..."
	set WGPU_BACKEND=dx12 && cargo run --release

run-windows-vulkan:
	@echo "Running with Vulkan (Windows)..."
	set WGPU_BACKEND=vulkan && cargo run --release

# ============================================================================
# Quick shortcuts
# ============================================================================

r: run-release
b: build
t: test
c: check
f: fmt
