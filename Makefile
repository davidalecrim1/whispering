MODELS_DIR := $(HOME)/.whispering/models
MODEL_NAME ?= ggml-medium.bin
MODEL_URL ?= https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(MODEL_NAME)
APP_BUNDLE_ID := com.davidalecrim.whispering

.PHONY: install install-model build dev release lint clean clean-permissions

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
build:
	cargo build --manifest-path src-tauri/Cargo.toml

## Build and run in dev mode via Tauri CLI
dev:
	cargo tauri dev

## Build release bundle
release:
	cargo tauri build --bundles app
	./scripts/build_dmg.sh

## Run cargo fmt (format) and clippy (lint)
lint:
	cargo fmt --manifest-path src-tauri/Cargo.toml
	cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

## Remove build artifacts
clean:
	cargo clean --manifest-path src-tauri/Cargo.toml

## Reset macOS TCC permissions that can conflict between dev and release builds
clean-permissions:
	tccutil reset Accessibility
	tccutil reset Microphone $(APP_BUNDLE_ID)
