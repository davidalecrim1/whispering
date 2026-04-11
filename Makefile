MODELS_DIR := $(HOME)/.whispering/models
MODEL_NAME := ggml-medium.en.bin
MODEL_URL := https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(MODEL_NAME)

.PHONY: install install-model build dev release lint clean

## Download the default whisper.cpp model (medium.en) to ~/.whispering/models/
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
	cargo tauri build

## Run cargo fmt (format) and clippy (lint)
lint:
	cargo fmt --manifest-path src-tauri/Cargo.toml
	cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

## Remove build artifacts
clean:
	cargo clean --manifest-path src-tauri/Cargo.toml
