MODELS_DIR := $(HOME)/.whispering/models
MODEL_NAME ?= ggml-medium.bin
MODEL_URL ?= https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(MODEL_NAME)
APP_BUNDLE_ID := com.davidalecrim.whispering
CARGO_MANIFEST := src-tauri/Cargo.toml
CARGO_FLAGS := --locked --manifest-path $(CARGO_MANIFEST)
APP_BUNDLE := src-tauri/target/release/bundle/macos/Whispering.app
APPLICATIONS_APP := /Applications/Whispering.app

.PHONY: install install-model install-app reinstall build dev release frontend fmt fmt-check check clippy clippy-review test lint clean clean-accessibility clean-permissions

## Download the default whisper.cpp model (medium multilingual) to ~/.whispering/models/
install: install-model

install-model:
	@mkdir -p $(MODELS_DIR)
	@if [ -f "$(MODELS_DIR)/$(MODEL_NAME)" ]; then \
		echo "Model already installed at $(MODELS_DIR)/$(MODEL_NAME)"; \
	else \
		echo "Downloading $(MODEL_NAME) (~1.5GB)..."; \
		curl -L --progress-bar -o "$(MODELS_DIR)/$(MODEL_NAME)" "$(MODEL_URL)"; \
		echo "Model installed to $(MODELS_DIR)/$(MODEL_NAME)"; \
	fi

## Build the app in debug mode
build: frontend
	cargo build $(CARGO_FLAGS)

## Build and run in dev mode via Tauri CLI
dev:
	cargo tauri dev

## Build release bundle
release: clean-accessibility
	cargo tauri build --bundles app
	./scripts/build_dmg.sh

## Install the release app into /Applications for local testing
install-app: clean-accessibility
	pkill -x Whispering || true
	rm -rf "$(APPLICATIONS_APP)"
	cp -R "$(APP_BUNDLE)" "$(APPLICATIONS_APP)"
	open "$(APPLICATIONS_APP)"

## Build release bundle and reinstall it locally
reinstall: release install-app

frontend:
	npm run build

fmt:
	cargo fmt --manifest-path $(CARGO_MANIFEST)

fmt-check:
	cargo fmt --manifest-path $(CARGO_MANIFEST) -- --check

check:
	cargo check $(CARGO_FLAGS)

clippy:
	cargo clippy $(CARGO_FLAGS) --all-targets -- -D warnings

clippy-review:
	cargo clippy $(CARGO_FLAGS) --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo

test:
	cargo test $(CARGO_FLAGS)

## Run frontend build, rustfmt check, clippy, and tests
lint: frontend fmt-check clippy test

## Remove build artifacts
clean:
	cargo clean --manifest-path $(CARGO_MANIFEST)

## Remove this app from macOS Accessibility permissions before rebuilding release
clean-accessibility:
	@tccutil reset Accessibility $(APP_BUNDLE_ID) 2>/dev/null || true

## Reset macOS TCC permissions that can conflict between dev and release builds
clean-permissions: clean-accessibility
	tccutil reset Microphone $(APP_BUNDLE_ID)
