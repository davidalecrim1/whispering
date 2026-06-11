#!/usr/bin/env bash

set -euo pipefail

PLATFORM="${1:-}"
MODEL_NAME="${2:-ggml-medium.bin}"
MODEL_URL="${3:-https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL_NAME}}"

if [[ -z "$PLATFORM" ]]; then
  case "$(uname -s)" in
    Darwin) PLATFORM="macos" ;;
    Linux) PLATFORM="linux" ;;
    *)
      echo "Unsupported OS for install-model.sh" >&2
      exit 1
      ;;
  esac
fi

case "$PLATFORM" in
  macos|linux) ;;
  *)
    echo "install-model.sh only supports macos or linux" >&2
    exit 1
    ;;
esac

MODELS_DIR="${HOME}/.whispering/models"
MODEL_PATH="${MODELS_DIR}/${MODEL_NAME}"

echo "Detected OS: ${PLATFORM}"
mkdir -p "$MODELS_DIR"

if [[ -f "$MODEL_PATH" ]]; then
  echo "Model already installed at $MODEL_PATH"
  exit 0
fi

echo "Downloading ${MODEL_NAME} to ${MODEL_PATH}"
curl -L --progress-bar -o "$MODEL_PATH" "$MODEL_URL"
echo "Model installed to $MODEL_PATH"
