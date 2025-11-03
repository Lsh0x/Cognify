#!/bin/bash
# Script to initialize Ollama container with embedding models
# Run this after docker-compose up to ensure models are downloaded

set -e

CONTAINER_NAME="cognifs-ollama"
MODEL="${1:-nomic-embed-text}"

# Ensure container is running
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  echo "Error: Container ${CONTAINER_NAME} is not running. Start it with 'docker-compose up -d' first."
  exit 1
fi

echo "Waiting for Ollama container to be ready..."
max_attempts=30
attempt=0

# Check readiness via host port instead of requiring curl inside the container
while [ $attempt -lt $max_attempts ]; do
    if curl -sSf http://localhost:11434/api/tags >/dev/null 2>&1; then
        echo "Ollama container is ready!"
        break
    fi
    attempt=$((attempt + 1))
    echo "Waiting for Ollama container... (attempt $attempt/$max_attempts)"
    sleep 2
done

if [ $attempt -eq $max_attempts ]; then
    echo "Error: Ollama container did not become ready in time"
    exit 1
fi

echo "Checking if model ${MODEL} is available..."
if docker exec "$CONTAINER_NAME" ollama list | grep -q "^${MODEL}"; then
    echo "Model ${MODEL} is already available"
else
    echo "Pulling model ${MODEL} (this may take a few minutes)..."
    docker exec "$CONTAINER_NAME" ollama pull "${MODEL}"
    echo "Model ${MODEL} pulled successfully"
fi

echo "Ollama initialization complete!"

