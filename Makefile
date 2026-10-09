# Runalytics developer tasks.
#
# CI runs these same targets (see .github/workflows/ci.yml), so a branch that
# passes `make check` locally passes the quality gate too. The gate is the
# Cargo workspace: there is no JavaScript build — the desktop UI is static and
# embedded via Tauri's `withGlobalTauri`, so there is no pnpm/npm step.

CARGO ?= cargo

.DEFAULT_GOAL := help

.PHONY: help
help: ## List the available targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

# --- quality gate (mirrored by CI) -----------------------------------------

.PHONY: fmt
fmt: ## Check formatting (CI gate)
	$(CARGO) fmt --all --check

.PHONY: fmt-fix
fmt-fix: ## Format the workspace in place
	$(CARGO) fmt --all

.PHONY: clippy
clippy: ## Lint with warnings denied (CI gate)
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

.PHONY: test
test: ## Run the workspace test suite (CI gate)
	$(CARGO) test --workspace --all-targets --locked

.PHONY: check
check: fmt clippy test ## Run every CI gate locally

# --- build & run ------------------------------------------------------------

.PHONY: dev
dev: ## Run the desktop app (debug)
	$(CARGO) run -p runalytics-desktop

.PHONY: mcp
mcp: ## Build the MCP server (release) — the agent-facing sidecar
	$(CARGO) build -p runalytics-mcp --release

.PHONY: bundle
bundle: ## Build the macOS .app bundle (needs the Tauri CLI; see `make tools`)
	$(CARGO) tauri build

.PHONY: tools
tools: ## Install the Tauri CLI used by `make bundle`
	$(CARGO) install tauri-cli --version '^2' --locked

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
