MODELS_DIR := $(HOME)/.whispering/models
MODEL_NAME ?= ggml-medium.bin
MODEL_URL ?= https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(MODEL_NAME)
APP_BUNDLE_ID := com.davidalecrim.whispering
CARGO_MANIFEST := src-tauri/Cargo.toml
CARGO_FLAGS := --locked --manifest-path $(CARGO_MANIFEST)

.PHONY: install install-model build dev release frontend fmt fmt-check check clippy clippy-review test lint clean clean-permissions

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
release:
	cargo tauri build --bundles app
	./scripts/build_dmg.sh

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

## Reset macOS TCC permissions that can conflict between dev and release builds
clean-permissions:
	tccutil reset Accessibility
	tccutil reset Microphone $(APP_BUNDLE_ID)
