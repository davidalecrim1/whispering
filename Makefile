MODEL_NAME ?= ggml-medium.bin
MODEL_URL ?= https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(MODEL_NAME)
APP_BUNDLE_ID := com.davidalecrim.whispering
CARGO_MANIFEST := src-tauri/Cargo.toml
CARGO_FLAGS := --locked --manifest-path $(CARGO_MANIFEST)
APP_BUNDLE := src-tauri/target/release/bundle/macos/Whispering.app
APPLICATIONS_APP := /Applications/Whispering.app
LINUX_APPIMAGE := $(HOME)/.local/opt/Whispering/Whispering.AppImage

.PHONY: install install-model install-model-macos install-model-linux install-model-windows install-macos install-linux install-windows run run-macos-app run-linux-app run-windows-app reinstall build dev release release-macos release-linux release-windows frontend frontend-deps fmt fmt-check cargo-check check clippy clippy-review test lint clean clean-accessibility-macos clean-permissions-macos

## Detect the host OS, log it, and run the full platform install flow
install:
	@OS_NAME="$$(uname -s 2>/dev/null || printf '%s' "$${OS:-Unknown}")"; \
	case "$$OS_NAME" in \
		Darwin) echo "Detected OS: macos"; $(MAKE) install-macos MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		Linux) echo "Detected OS: linux"; $(MAKE) install-linux MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		MINGW*|MSYS*|CYGWIN*|Windows_NT) echo "Detected OS: windows"; $(MAKE) install-windows MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		*) echo "Unsupported OS: $$OS_NAME"; exit 1 ;; \
	esac

## Detect the host OS, log it, and install only the default whisper.cpp model
install-model:
	@OS_NAME="$$(uname -s 2>/dev/null || printf '%s' "$${OS:-Unknown}")"; \
	case "$$OS_NAME" in \
		Darwin) echo "Detected OS: macos"; $(MAKE) install-model-macos MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		Linux) echo "Detected OS: linux"; $(MAKE) install-model-linux MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		MINGW*|MSYS*|CYGWIN*|Windows_NT) echo "Detected OS: windows"; $(MAKE) install-model-windows MODEL_NAME="$(MODEL_NAME)" MODEL_URL="$(MODEL_URL)" ;; \
		*) echo "Unsupported OS: $$OS_NAME"; exit 1 ;; \
	esac

## Download the default whisper.cpp model on macOS
install-model-macos:
	@./scripts/install-model.sh macos "$(MODEL_NAME)" "$(MODEL_URL)"

## Download the default whisper.cpp model on Linux
install-model-linux:
	@./scripts/install-model.sh linux "$(MODEL_NAME)" "$(MODEL_URL)"

## Download the default whisper.cpp model on Windows without OS detection
install-model-windows:
	@POWERSHELL_CMD="$$(command -v powershell.exe 2>/dev/null || command -v pwsh 2>/dev/null)"; \
	if [ -z "$$POWERSHELL_CMD" ]; then \
		echo "PowerShell is required for install-model-windows"; \
		exit 1; \
	fi; \
	"$$POWERSHELL_CMD" -ExecutionPolicy Bypass -File scripts/install-model.ps1 -Platform windows -ModelName "$(MODEL_NAME)" -ModelUrl "$(MODEL_URL)"

## Build the app in debug mode
build: frontend
	cargo build $(CARGO_FLAGS)

## Build and run in dev mode via Tauri CLI
dev:
	cargo tauri dev

## Run the app from the repo in development mode
run: dev

## Detect the host OS and run the matching release target
release:
	@OS_NAME="$$(uname -s 2>/dev/null || printf '%s' "$${OS:-Unknown}")"; \
	case "$$OS_NAME" in \
		Darwin) echo "Detected OS: macos"; $(MAKE) release-macos ;; \
		Linux) echo "Detected OS: linux"; $(MAKE) release-linux ;; \
		MINGW*|MSYS*|CYGWIN*|Windows_NT) echo "Detected OS: windows"; $(MAKE) release-windows ;; \
		*) echo "Unsupported OS: $$OS_NAME"; exit 1 ;; \
	esac

## Build the macOS app bundle and DMG
release-macos: clean-accessibility-macos
	cargo tauri build --bundles app
	./scripts/build_dmg.sh

## Build the Linux AppImage release artifact on a Linux host
release-linux:
	cargo tauri build --bundles appimage

## Build the Windows MSI release artifact on a Windows host
release-windows:
	cargo tauri build --bundles msi

## Install the macOS app: model download, release build, and /Applications copy
install-macos: install-model-macos release-macos
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		echo "install-macos is only supported on macOS"; \
		exit 1; \
	fi
	pkill -x Whispering || true
	rm -rf "$(APPLICATIONS_APP)"
	cp -R "$(APP_BUNDLE)" "$(APPLICATIONS_APP)"
	open "$(APPLICATIONS_APP)"
	@echo "Installed macOS app: $(APPLICATIONS_APP)"

## Install the Linux app: model download, AppImage build, and user-local desktop install
install-linux: install-model-linux release-linux
	@./scripts/install-linux-app.sh
	@echo "Installed Linux app: $(LINUX_APPIMAGE)"
	@echo "Launch it with 'make run-linux-app' or from your desktop menu."

## Install the Windows app: model download, MSI build, and native installer run
install-windows: install-model-windows release-windows
	@POWERSHELL_CMD="$$(command -v powershell.exe 2>/dev/null || command -v pwsh 2>/dev/null)"; \
	if [ -z "$$POWERSHELL_CMD" ]; then \
		echo "PowerShell is required for install-windows"; \
		exit 1; \
	fi; \
	"$$POWERSHELL_CMD" -ExecutionPolicy Bypass -File scripts/install-windows-app.ps1

## Launch the installed macOS app
run-macos-app:
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		echo "run-macos-app is only supported on macOS"; \
		exit 1; \
	fi
	open "$(APPLICATIONS_APP)"

## Launch the installed Linux app from the user-local AppImage path
run-linux-app:
	@if [ "$$(uname -s)" != "Linux" ]; then \
		echo "run-linux-app is only supported on Linux"; \
		exit 1; \
	fi
	@if [ ! -x "$(LINUX_APPIMAGE)" ]; then \
		echo "Installed Linux app not found at $(LINUX_APPIMAGE)"; \
		echo "Run 'make install-linux' first."; \
		exit 1; \
	fi
	nohup "$(LINUX_APPIMAGE)" >/dev/null 2>&1 &

## Launch the installed Windows app from the native install location
run-windows-app:
	@POWERSHELL_CMD="$$(command -v powershell.exe 2>/dev/null || command -v pwsh 2>/dev/null)"; \
	if [ -z "$$POWERSHELL_CMD" ]; then \
		echo "PowerShell is required for run-windows-app"; \
		exit 1; \
	fi; \
	"$$POWERSHELL_CMD" -ExecutionPolicy Bypass -File scripts/install-windows-app.ps1 -LaunchOnly

## Build the macOS release bundle and reinstall it locally
reinstall: release-macos install-macos

frontend: frontend-deps
	npm run build

frontend-deps:
	@if [ ! -d node_modules ] || [ package-lock.json -nt node_modules ] || [ package.json -nt node_modules ]; then \
		npm ci; \
	fi

fmt:
	cargo fmt --manifest-path $(CARGO_MANIFEST)

fmt-check:
	cargo fmt --manifest-path $(CARGO_MANIFEST) -- --check

cargo-check:
	cargo check $(CARGO_FLAGS)

## CI-safe validation without rewriting files
check: frontend fmt-check clippy test

clippy:
	cargo clippy $(CARGO_FLAGS) --all-targets -- -D warnings

clippy-review:
	cargo clippy $(CARGO_FLAGS) --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo

test:
	cargo test $(CARGO_FLAGS)

## Full validation gate for local work
lint: frontend-deps check

## Remove build artifacts
clean:
	cargo clean --manifest-path $(CARGO_MANIFEST)

## Reset macOS Accessibility permission state before rebuilding a macOS release
clean-accessibility-macos:
	@if [ "$$(uname -s 2>/dev/null || printf '%s' "$${OS:-Unknown}")" = "Darwin" ]; then \
		tccutil reset Accessibility $(APP_BUNDLE_ID) 2>/dev/null || true; \
	else \
		echo "Skipping macOS accessibility reset on non-macOS host"; \
	fi

## Reset macOS TCC permissions that can conflict between macOS dev and release builds
clean-permissions-macos: clean-accessibility-macos
	@if [ "$$(uname -s 2>/dev/null || printf '%s' "$${OS:-Unknown}")" = "Darwin" ]; then \
		tccutil reset Microphone $(APP_BUNDLE_ID); \
	else \
		echo "Skipping macOS permission reset on non-macOS host"; \
	fi
