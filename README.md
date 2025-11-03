# Cognifs

The Cognitive File System — it understands and organizes your files automatically.

[![GitHub last commit](https://img.shields.io/github/last-commit/lsh0x/Cognifs)](https://github.com/lsh0x/Cognifs/commits/main)
[![CI](https://github.com/lsh0x/Cognifs/workflows/CI/badge.svg)](https://github.com/lsh0x/Cognifs/actions)
[![Codecov](https://codecov.io/gh/lsh0x/Cognifs/branch/main/graph/badge.svg)](https://codecov.io/gh/lsh0x/Cognifs)
[![Docs](https://docs.rs/cognifs/badge.svg)](https://docs.rs/cognifs)
[![Crates.io](https://img.shields.io/crates/v/cognifs.svg)](https://crates.io/crates/cognifs)
[![crates.io](https://img.shields.io/crates/d/cognifs)](https://crates.io/crates/cognifs)

## 🚀 Vision

Cognifs is an AI-powered, auto-organizing file system. It scans directories, extracts metadata, understands content using LLMs and semantic embeddings, generates intelligent tags, clusters related documents, and physically reorganizes files into meaningful folders — without any manual rules. A Meilisearch index provides instant semantic search and smart virtual folders.

## 🧩 Core Features

- **Scanner** — Recursively read files, collect metadata (size, extension, created_at, hash)
- **Watcher** — Monitor directories for file changes (safe, only watches specified paths)
- **Tagger System** — Modular, trait-based per file type:
  - Dictionary + LLM tagging
  - Handlers per extension (txt, md, etc.)
  - Custom logic for different file types
- **Embeddings** — Compute semantic vectors via Ollama (nomic-embed-text, mxbai-embed-large) for clustering + search
- **Indexing** — Store metadata + embeddings in Meilisearch (vector search enabled)
- **Search** — Query Meilisearch with semantic + tag filters
- **Organizer** — Auto-create folder names from dominant tags, preview changes, safely reorganize files
- **CLI** — Complete command-line interface for all operations

## 🏗️ Architecture

Cognifs is built with a **trait-based architecture** for maximum extensibility:

- **Trait-based components**: LLM providers, embedding providers, indexers, and taggers all use traits
- **Local-first**: Works with local LLMs (guff) and local Ollama instance
- **Extensible**: Easy to add new file type handlers, LLM providers, or search backends
- **Safe**: Only operates on explicitly provided directory paths (never entire filesystem)
- **Async**: Built on Tokio for high-performance async I/O

## 📦 Installation

### From Source

```bash
git clone https://github.com/lsh0x/Cognifs.git
cd Cognifs
cargo build --release
```

### Requirements

- Rust 1.70+
- Meilisearch (for indexing and search)
- Ollama (for embeddings, optional)
- Local LLM (guff/llama.cpp) for intelligent tagging (optional)

## 🚦 Usage

### Watch a Directory

Monitor a directory for file changes:

```bash
cognifs watch ~/Documents/cognifs/
```

### Tag a Single File

Tag a file and see extracted tags:

```bash
cognifs tag ~/Documents/notes.txt
```

### Index Files to Meilisearch

Index all files in a directory:

```bash
cognifs index ~/Documents/ \
  --meili-url http://127.0.0.1:7700 \
  --index-name my-files
```

### Search Files

Search indexed files:

```bash
cognifs search "meeting notes" \
  --meili-url http://127.0.0.1:7700 \
  --index-name my-files
```

### Organize Files

Automatically organize files into folders based on tags (with preview):

```bash
cognifs organize ~/Documents/
```

Use `--dry-run` to preview without making changes:

```bash
cognifs organize ~/Documents/ --dry-run
```

Skip confirmation with `--yes` (use with caution):

```bash
cognifs organize ~/Documents/ --yes
```

## ⚙️ Configuration

### LLM Configuration

Edit `config/llm.yaml`:

```yaml
provider: local
model_path: ~/.local/share/models/guff/model.bin
executable: guff
```

### Embeddings

Embeddings use Ollama by default (http://127.0.0.1:11434). Supported models:
- `nomic-embed-text` (768 dimensions) - default
- `mxbai-embed-large` (1024 dimensions)

### Using Docker Compose

The easiest way to run all dependencies is using Docker Compose:

```bash
docker-compose up -d
```

This will start:
- **Meilisearch** on `http://127.0.0.1:7700`
- **Ollama** on `http://127.0.0.1:11434`

To stop all services:

```bash
docker-compose down
```

### Manual Setup

#### Meilisearch

Start a local Meilisearch instance:

```bash
docker run -it -p 7700:7700 -v $(pwd)/meili_data:/meili_data getmeili/meilisearch:latest
```

Or use [Meilisearch Cloud](https://www.meilisearch.com/cloud).

#### Ollama

Install Ollama following the [official instructions](https://ollama.ai), then pull the embedding models:

```bash
ollama pull nomic-embed-text
# or for larger model
ollama pull mxbai-embed-large
```

## 🧱 Project Structure

```
cognifs/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── models.rs        # FileMeta struct
│   ├── watcher.rs       # Filesystem watcher
│   ├── tagger/          # File type handlers
│   │   ├── trait.rs     # Taggable trait
│   │   ├── text/        # Text handlers (txt, md)
│   │   └── registry.rs  # Handler registry
│   ├── llm/             # LLM providers
│   │   ├── trait.rs     # LlmProvider trait
│   │   └── local.rs     # Local guff implementation
│   ├── embeddings/      # Embedding providers
│   │   ├── trait.rs     # EmbeddingProvider trait
│   │   └── local.rs     # Ollama embeddings
│   ├── indexer/         # Search backends
│   │   ├── trait.rs     # Indexer trait
│   │   └── meili.rs     # Meilisearch implementation
│   └── organizer/       # File organization
│       ├── generator.rs # Folder name generation
│       ├── mover.rs     # File reorganization
│       └── preview.rs   # Tree preview
├── config/
│   └── llm.yaml         # LLM configuration
└── Cargo.toml
```

## 🔧 Development

### Running Tests

```bash
cargo test
```

### Building

```bash
cargo build --release
```

### Code Structure

- **Traits** define interfaces for extensibility (LlmProvider, EmbeddingProvider, Indexer, Taggable)
- **Implementations** live in separate modules (e.g., `llm/local.rs`, `embeddings/local.rs`)
- **Registry pattern** used for taggers to select handlers by file extension
- **All I/O is async** using Tokio

## 🧠 Design Principles

1. **Trait-based**: All major components use traits for easy extension
2. **Local-first**: Works offline, optional cloud services
3. **Safe**: Never operates on entire filesystem, only specified directories
4. **Testable**: Comprehensive unit tests for all components
5. **Modular**: Each component is isolated and can be extended independently

## 🚧 Roadmap

- [ ] Additional file type handlers (PDF, images, video, audio)
- [ ] Clustering algorithm for grouping similar files
- [ ] HTTP LLM providers (OpenAI, Mistral)
- [ ] Alternative indexers (Qdrant, local JSON)
- [ ] Feedback learning system
- [ ] Tauri desktop app

## 📝 License

Licensed under the GPL-3.0 License.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
