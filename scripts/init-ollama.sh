#!/bin/bash
# Script to initialize Ollama with required embedding models
# This script waits for Ollama to be ready, then pulls the required models

set -e

OLLAMA_HOST="${OLLAMA_HOST:-http://localhost:11434}"
MODEL="${OLLAMA_MODEL:-nomic-embed-text}"

echo "Waiting for Ollama to be ready..."
max_attempts=30
attempt=0

while [ $attempt -lt $max_attempts ]; do
    if curl -s "${OLLAMA_HOST}/api/tags" > /dev/null 2>&1; then
        echo "Ollama is ready!"
        break
    fi
    attempt=$((attempt + 1))
    echo "Waiting for Ollama... (attempt $attempt/$max_attempts)"
    sleep 2
done

if [ $attempt -eq $max_attempts ]; then
    echo "Error: Ollama did not become ready in time"
    exit 1
fi

echo "Checking if model ${MODEL} is already available..."
if curl -s "${OLLAMA_HOST}/api/tags" | grep -q "\"name\":\"${MODEL}\""; then
    echo "Model ${MODEL} is already available"
else
    echo "Pulling model ${MODEL}..."
    curl -X POST "${OLLAMA_HOST}/api/pull" -d "{\"name\":\"${MODEL}\"}"
    echo ""
    echo "Model ${MODEL} pulled successfully"
fi

echo "Ollama initialization complete!"

