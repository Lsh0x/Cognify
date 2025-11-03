#!/bin/bash

# Script to download a 7B GGUF model for Cognifs
# This script downloads Mistral-7B-Instruct-v0.2 in Q4_K_M quantization
# (Good balance between quality and size ~4GB)

set -e

MODEL_DIR="$HOME/.local/share/models/guff"
MODEL_NAME="mistral-7b-instruct-v0.2.Q4_K_M.gguf"
HF_REPO="TheBloke/Mistral-7B-Instruct-v0.2-GGUF"
MODEL_URL="https://huggingface.co/${HF_REPO}/resolve/main/${MODEL_NAME}"

echo "🔽 Downloading GGUF model for Cognifs..."
echo "   Model: Mistral-7B-Instruct-v0.2 (Q4_K_M)"
echo "   Size: ~4.1GB"
echo ""

# Create model directory
mkdir -p "$MODEL_DIR"
cd "$MODEL_DIR"

# Check if model already exists
if [ -f "$MODEL_NAME" ]; then
    echo "⚠️  Model already exists: $MODEL_DIR/$MODEL_NAME"
    read -p "   Do you want to re-download? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "✅ Skipping download."
        exit 0
    fi
    rm -f "$MODEL_NAME"
fi

echo "📥 Downloading from Hugging Face..."
echo "   URL: $MODEL_URL"
echo ""

# Download using wget or curl
if command -v wget &> /dev/null; then
    wget --progress=bar:force "$MODEL_URL" -O "$MODEL_NAME"
elif command -v curl &> /dev/null; then
    curl -L --progress-bar "$MODEL_URL" -o "$MODEL_NAME"
else
    echo "❌ Error: Neither wget nor curl found. Please install one of them."
    exit 1
fi

# Verify download
if [ -f "$MODEL_NAME" ]; then
    SIZE=$(du -h "$MODEL_NAME" | cut -f1)
    echo ""
    echo "✅ Download complete!"
    echo "   Location: $MODEL_DIR/$MODEL_NAME"
    echo "   Size: $SIZE"
    echo ""
    echo "📝 Update your config/settings.toml with:"
    echo "   model_path = \"$MODEL_DIR/$MODEL_NAME\""
    echo ""
    echo "   Or keep the default path if you name it 'model.bin'"
else
    echo "❌ Error: Download failed"
    exit 1
fi

