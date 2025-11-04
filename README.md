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
- **SemanticSource System** — Modular, trait-based per file type:
  - Text extraction from various file types (txt, md, pdf, csv, zip)
  - Dictionary + LLM tagging
  - Metadata extraction (PDF metadata, CSV headers, etc.)
  - Custom logic for different file types
- **Embeddings** — Compute semantic vectors via Ollama (nomic-embed-text, mxbai-embed-large) for clustering + search
- **Indexing** — Store metadata + embeddings in Meilisearch (vector search enabled)
- **Incremental Sync** — Smart synchronization that updates existing documents without losing data
- **Search** — Query Meilisearch with semantic + tag filters
- **Organizer** — Auto-create folder names from dominant tags, preview changes, safely reorganize files
- **CLI** — Complete command-line interface for all operations

## 🏗️ Architecture

Cognifs is built with a **trait-based architecture** for maximum extensibility:

- **Trait-based components**: LLM providers, embedding providers, indexers, and file handlers all use traits
- **SemanticSource**: Unified interface for extracting text, metadata, and generating tags from files
- **Local-first**: Works with local LLMs (llama-cpp) and local Ollama instance
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
- Local LLM via llama-cpp (GGUF) for intelligent tagging (optional)

## 🚦 Usage

Cognifs provides multiple binary executables for different operations. Each command is available as a separate binary:

- `cognifs-watch` - Monitor directories for file changes
- `cognifs-tag` - Tag individual files
- `cognifs-index` - Index files to Meilisearch
- `cognifs-search` - Search indexed files
- `cognifs-organize` - Organize files into folders

You can also use the main `cognifs` binary which dispatches to the appropriate command.

### Watch a Directory

Monitor a directory for file changes:

```bash
cognifs-watch ~/Documents/
```

With auto-indexing enabled, files are automatically indexed when they change:

```bash
cognifs-watch ~/Documents/ --auto-index
```

This will:
- **Index** new files when they are created
- **Update** the index when files are modified
- **Remove** files from the index when they are deleted

### Tag a Single File

Tag a file and see extracted tags:

```bash
cognifs-tag ~/Documents/notes.txt
```

Save tags to Meilisearch index:

```bash
cognifs-tag ~/Documents/notes.txt --save
```

### Index Files to Meilisearch

Index all files in a directory with incremental sync (updates existing documents, removes deleted files):

```bash
cognifs-index ~/Documents/ \
  --meili-url http://127.0.0.1:7700 \
  --index-name my-files
```

The index command automatically:
- **Synchronizes** the index with the filesystem (detects changes, removes deleted files)
- **Updates** existing documents when file content changes
- **Preserves** all existing data when reindexing
- Shows sync statistics (unchanged/updated/deleted files)

### Search Files

Search indexed files:

```bash
cognifs-search "meeting notes" \
  --meili-url http://127.0.0.1:7700 \
  --index-name my-files
```

### Organize Files

Automatically organize files into folders based on tags (with preview):

```bash
cognifs-organize ~/Documents/
```

Use `--dry-run` to preview without making changes:

```bash
cognifs-organize ~/Documents/ --dry-run
```

Skip confirmation with `--yes` (use with caution):

```bash
cognifs-organize ~/Documents/ --yes
```

Enable intelligent LLM-powered tag generation (requires LLM configured in `settings.toml`):

```bash
cognifs-organize ~/Documents/ --use-llm
```

Index files after organizing:

```bash
cognifs-organize ~/Documents/ --index
```

The `--use-llm` flag enhances tag generation by using your local LLM (llama-cpp) to understand file content semantically. If the LLM is not available, it falls back to dictionary-based tagging.

The `--index` flag will index all organized files to Meilisearch after the reorganization is complete, using the tags and embeddings already computed during the organization process.

**Protected Structures**: Cognifs automatically detects and skips files inside protected directory structures to preserve their organization. These include:

- **Version Control Systems**: Git (`.git`), Mercurial (`.hg`), Subversion (`.svn`), Bazaar (`.bzr`), CVS, Fossil (`.fossil`)
- **Project Dependencies**: `node_modules`, `target` (Rust), `dist`, `build`, `.gradle`, `.mvn`
- **Virtual Environments**: `venv`, `.venv`, `env`, `.env`, `__pycache__`, `.pytest_cache`, `.tox`, `.mypy_cache`
- **Project Config Files**: When files like `package.json`, `Cargo.toml`, `go.mod`, `requirements.txt`, `pom.xml`, `build.gradle`, `composer.json`, `Gemfile`, `docker-compose.yml`, or `Dockerfile` are found, the entire project directory is protected

Files inside these protected structures will not be moved or reorganized.

## ⚙️ Configuration

Cognifs uses a TOML configuration file (`config/settings.toml`) for default settings. All settings can be overridden via command-line flags.

### Configuration File

Edit `config/settings.toml` to customize defaults:

```toml
[meilisearch]
url = "http://127.0.0.1:7700"
# api_key = ""  # Optional, can also use MEILI_MASTER_KEY env var
index_name = "cognifs"

[ollama]
url = "http://127.0.0.1:11434"
model = "nomic-embed-text"  # or "mxbai-embed-large"
dims = 768  # Embedding dimension (768 for nomic-embed-text, 1024 for mxbai-embed-large)

[llm]
provider = "local"
model_path = "~/.local/share/models/guff/model.bin"
executable = "guff"
```

**Note**: The default settings match the docker-compose.yml services.

**Important**: After starting Ollama with Docker Compose, you must initialize it with the embedding model:
```bash
./scripts/init-ollama-container.sh nomic-embed-text
```

You can override any setting via CLI flags.

### LLM Configuration

LLM settings are configured in `config/settings.toml`:

```toml
[llm]
provider = "local"
model_path = "~/.local/share/models/guff/model.bin"
executable = "guff"
```

#### Getting GGUF Model Files

Cognifs uses local LLM models in GGUF format. The easiest way to get a model is using the provided script:

**Quick Download (Recommended)**

Run the download script to get a 7B model (~4GB):

```bash
./scripts/download-model.sh
```

This downloads Mistral-7B-Instruct-v0.2 (Q4_K_M quantization) to `~/.local/share/models/guff/`.

**Manual Download from Hugging Face**

1. Visit [Hugging Face Models](https://huggingface.co/models?library=gguf)
2. Search for a compatible model (e.g., "llama", "mistral", "phi")
3. Download a GGUF file (look for `*.gguf` files, Q4_K_M or Q5_K_M are good quality/size balance)
4. Save it to your model directory:

```bash
mkdir -p ~/.local/share/models/guff/
# Move your downloaded .gguf file to this location
mv ~/Downloads/model.gguf ~/.local/share/models/guff/model.bin
```

**Option 2: Using Ollama (Alternative)**

You can also use Ollama's models for LLM tagging (if you add HTTP provider support):

```bash
# Pull a model via Ollama
docker exec cognifs-ollama ollama pull llama3.2:1b
```

**Option 3: Pre-converted Models**

Popular sources for pre-converted GGUF models:
- [TheBloke's Hugging Face](https://huggingface.co/TheBloke) - Many models in GGUF format
- [llama.cpp releases](https://github.com/ggerganov/llama.cpp/releases) - Official conversions

**Recommended Models for Tagging:**
- `llama-3.2-1b` - Fast, efficient for tagging
- `mistral-7b-instruct` - Good balance of quality and speed
- `phi-2` - Small, fast, Microsoft's model

### Embeddings

Embeddings use Ollama by default (http://localhost:11434). Supported models:
- `nomic-embed-text` (768 dimensions) - default, smaller and faster
- `mxbai-embed-large` (1024 dimensions) - larger, potentially more accurate

**Important**: The embedding model must be downloaded before use. See [Using Docker Compose](#using-docker-compose) section for initialization instructions.

### Overriding Configuration

All settings can be overridden via command-line flags:

```bash
# Use different Meilisearch URL
cognifs index ~/Documents/ --meili-url http://localhost:7701

# Use different embedding model
cognifs tag file.txt --embedding-model mxbai-embed-large
```

### Using Docker Compose

The easiest way to run all dependencies is using Docker Compose:

```bash
# Start all services
docker-compose up -d

# Initialize Ollama with embedding models (required on first run)
./scripts/init-ollama-container.sh nomic-embed-text
```

This will start:
- **Meilisearch** on `http://localhost:7700` (accessible from host, bound to 0.0.0.0)
- **Ollama** on `http://localhost:11434` (accessible from host, bound to all interfaces)

**Note**: The services are configured to be accessible from the host machine. For production, consider restricting access.

**Important**: After starting Ollama for the first time, you must pull the embedding model. Run:

```bash
./scripts/init-ollama-container.sh nomic-embed-text
```

Or for the larger model:

```bash
./scripts/init-ollama-container.sh mxbai-embed-large
```

The initialization script will:
- Verify that the Ollama container is running (exits with error if not)
- Wait for Ollama API to be ready (checks via host port using `curl`)
- Check if the model is already downloaded using `ollama list`
- Pull the model if needed (this may take a few minutes on first run)

**Note**: The script uses `curl` from the host to check Ollama readiness, avoiding dependencies inside the container. The healthcheck in docker-compose uses `ollama list` directly.

**Note**: Models are persisted in `./data/ollama_data`, so you only need to pull them once.

To stop all services:

```bash
docker-compose down
```

To restart after pulling models:

```bash
docker-compose restart
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

**Using Docker Compose (recommended)**:

```bash
# Start Ollama
docker-compose up -d ollama

# Pull the embedding model using the initialization script
./scripts/init-ollama-container.sh nomic-embed-text

# Or manually:
docker exec cognifs-ollama ollama pull nomic-embed-text
```

**Using local Ollama installation**:

```bash
# Pull embedding models
ollama pull nomic-embed-text
# or for larger model
ollama pull mxbai-embed-large
```

**Available embedding models**:
- `nomic-embed-text` (768 dimensions) - default, smaller and faster
- `mxbai-embed-large` (1024 dimensions) - larger, potentially more accurate

Update your `config/settings.toml` to use a different model:

```toml
[ollama]
url = "http://127.0.0.1:11434"
model = "mxbai-embed-large"  # Change this to use a different model
```

## 🧱 Project Structure

```
cognifs/
├── src/
│   ├── bin/                    # CLI binary executables
│   │   ├── cognifs.rs         # Main dispatcher binary
│   │   ├── cognifs-watch.rs   # Watch command binary
│   │   ├── cognifs-tag.rs     # Tag command binary
│   │   ├── cognifs-index.rs   # Index command binary
│   │   ├── cognifs-search.rs  # Search command binary
│   │   └── cognifs-organize.rs # Organize command binary
│   ├── lib.rs           # Library exports
│   ├── models.rs        # FileMeta struct
│   ├── watcher.rs       # Filesystem watcher
│   ├── file/            # File type handlers (SemanticSource)
│   │   ├── trait.rs     # SemanticSource trait
│   │   ├── factory.rs   # File factory
│   │   └── types/       # Type-specific handlers
│   │       ├── txt.rs   # Text files
│   │       ├── md.rs    # Markdown files
│   │       ├── pdf.rs   # PDF files
│   │       ├── csv.rs   # CSV files
│   │       ├── zip.rs   # ZIP archives
│   │       └── generic.rs # Generic fallback
│   ├── llm/             # LLM providers
│   │   ├── trait.rs     # LlmProvider trait
│   │   └── local.rs     # Local llama-cpp implementation
│   ├── embeddings/      # Embedding providers
│   │   ├── trait.rs     # EmbeddingProvider trait
│   │   └── local.rs     # Ollama embeddings
│   ├── indexer/         # Search backends
│   │   ├── trait.rs     # Indexer trait
│   │   └── meili.rs     # Meilisearch implementation
│   ├── organizer/       # File organization
│   │   ├── generator.rs # Folder name generation
│   │   ├── mover.rs     # File reorganization
│   │   ├── preview.rs   # Tree preview
│   │   ├── cluster.rs   # Semantic clustering
│   │   └── context.rs   # Path-based tag extraction
│   ├── utils.rs         # Utility functions
│   ├── constants.rs     # Constants (protected patterns, extensions)
│   └── config.rs        # Configuration management
├── config/
│   └── settings.toml    # Configuration file
├── scripts/
│   ├── download-model.sh # Download GGUF model script
│   └── init-ollama-container.sh # Initialize Ollama
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

- **Traits** define interfaces for extensibility (LlmProvider, EmbeddingProvider, Indexer, SemanticSource)
- **Implementations** live in separate modules (e.g., `llm/local.rs`, `embeddings/local.rs`, `file/types/`)
- **Factory pattern** used for file handlers to select handlers by file extension
- **Modular CLI** with each command in its own module (`src/bin/`)
- **All I/O is async** using Tokio

## 🧠 Design Principles

1. **Trait-based**: All major components use traits for easy extension
2. **Local-first**: Works offline, optional cloud services
3. **Safe**: Never operates on entire filesystem, only specified directories
4. **Testable**: Comprehensive unit tests for all components
5. **Modular**: Each component is isolated and can be extended independently

## 🚧 Roadmap

- [x] PDF file handler with native Rust libraries
- [x] CSV file handler
- [x] ZIP archive handler
- [x] Semantic clustering for file organization
- [x] Incremental sync for Meilisearch index
- [ ] Additional file type handlers (images, video, audio)
- [ ] HTTP LLM providers (OpenAI, Mistral)
- [ ] Alternative indexers (Qdrant, local JSON)
- [ ] Feedback learning system
- [ ] Tauri desktop app

## 📝 License

Licensed under the GPL-3.0 License.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
