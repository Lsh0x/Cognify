use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cognifs::{
    config::Config,
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider},
    indexer::{Indexer, MeilisearchIndexer},
    models::FileMeta,
    organizer::{FileMover, FolderGenerator, PreviewTree},
    tagger::TaggerRegistry,
    utils,
    watcher::FileWatcher,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cognifs")]
#[command(about = "The Cognitive File System — it understands and organizes your files automatically")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch a directory for file changes
    Watch {
        /// Directory to watch
        #[arg(value_name = "DIR")]
        dir: PathBuf,
    },
    /// Tag a single file
    Tag {
        /// File to tag
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Index files in a directory to Meilisearch
    Index {
        /// Directory to index
        #[arg(value_name = "DIR")]
        dir: PathBuf,
        /// Meilisearch URL (overrides config)
        #[arg(long)]
        meili_url: Option<String>,
        /// Meilisearch API key (overrides config and env)
        #[arg(long)]
        meili_key: Option<String>,
        /// Meilisearch index name (overrides config)
        #[arg(long)]
        index_name: Option<String>,
    },
    /// Search for files using Meilisearch
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,
        /// Meilisearch URL (overrides config)
        #[arg(long)]
        meili_url: Option<String>,
        /// Meilisearch API key (overrides config and env)
        #[arg(long)]
        meili_key: Option<String>,
        /// Meilisearch index name (overrides config)
        #[arg(long)]
        index_name: Option<String>,
    },
    /// Organize files into folders based on tags
    Organize {
        /// Directory to organize
        #[arg(value_name = "DIR")]
        dir: PathBuf,
        /// Dry run (preview only, don't move files)
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration (falls back to defaults if not found)
    let config = Config::load().unwrap_or_default();

    match cli.command {
        Commands::Watch { dir } => {
            println!("Watching directory: {}", dir.display());
            let watcher = FileWatcher::new(&dir)?;
            let mut rx = watcher.subscribe();
            
            watcher.watch().await?;
            println!("Watcher started. Press Ctrl+C to stop.");

            loop {
                tokio::select! {
                    event = rx.recv() => {
                        match event {
                            Ok(cognifs::watcher::WatchEvent::Created(meta)) => {
                                println!("Created: {}", meta.path.display());
                            }
                            Ok(cognifs::watcher::WatchEvent::Modified(meta)) => {
                                println!("Modified: {}", meta.path.display());
                            }
                            Ok(cognifs::watcher::WatchEvent::Deleted(path)) => {
                                println!("Deleted: {}", path.display());
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        Commands::Tag { file } => {
            println!("Tagging file: {}", file.display());
            
            // Get file metadata
            let metadata = std::fs::metadata(&file)
                .with_context(|| format!("Failed to read file: {}", file.display()))?;
            
            let extension = utils::get_extension(&file);
            let extension_str = extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
            let hash = utils::compute_file_hash(&file)?;
            let created_at = metadata
                .created()
                .or_else(|_| metadata.modified())
                .unwrap_or_else(|_| std::time::SystemTime::now());

            // Get handler before moving extension
            let registry = TaggerRegistry::new();
            let handler = registry.get_handler(extension_str);

            let file_meta = FileMeta::new(
                file.clone(),
                metadata.len(),
                extension,
                created_at,
                hash,
            );

            let content = handler.extract_text(&file_meta).await?;
            println!("Extracted {} bytes of text", content.len());

            // Generate tags
            let tags = handler.generate_tags(&content).await?;
            println!("Tags: {:?}", tags);

            // Optionally compute embedding
            let ollama_url = config.ollama.url.as_str();
            let embedding_model = config.ollama.model.as_str();
            let embedding_provider = LocalEmbeddingProvider::new(Some(ollama_url), Some(embedding_model));
            match embedding_provider.compute_embedding(&content).await {
                Ok(embedding) => {
                    println!("Embedding computed: {} dimensions", embedding.len());
                }
                Err(e) => {
                    eprintln!("Warning: Failed to compute embedding: {}", e);
                }
            }
        }
        Commands::Index { dir, meili_url, meili_key, index_name } => {
            println!("Indexing directory: {}", dir.display());
            
            let meili_url = meili_url.unwrap_or_else(|| config.meilisearch.url.clone());
            let meili_key = meili_key.or_else(|| config.meilisearch_api_key());
            let index_name = index_name.unwrap_or_else(|| config.meilisearch.index_name.clone());
            
            let indexer = MeilisearchIndexer::new(
                &meili_url,
                meili_key.as_deref(),
                &index_name,
            )
            .await
            .context("Failed to create Meilisearch indexer")?;

            // Walk directory and index files
            let mut count = 0;
            for entry in walkdir::WalkDir::new(&dir) {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                // Get metadata
                let metadata = std::fs::metadata(path)?;
                let extension = utils::get_extension(path);
                let extension_str = extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
                let hash = utils::compute_file_hash(path)?;
                let created_at = metadata
                    .created()
                    .or_else(|_| metadata.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::now());

                // Get handler before moving extension
                let registry = TaggerRegistry::new();
                let handler = registry.get_handler(extension_str);

                let file_meta = FileMeta::new(
                    path.to_path_buf(),
                    metadata.len(),
                    extension,
                    created_at,
                    hash,
                );

                let content = match handler.extract_text(&file_meta).await {
                    Ok(c) => c,
                    Err(_) => continue, // Skip files that can't be extracted
                };

                let tags = handler.generate_tags(&content).await?;

                // Index without embedding for now
                indexer.index_file(&file_meta, &tags).await?;
                count += 1;

                if count % 10 == 0 {
                    println!("Indexed {} files...", count);
                }
            }

            println!("Indexed {} files total", count);
        }
        Commands::Search { query, meili_url, meili_key, index_name } => {
            println!("Searching for: {}", query);
            
            let meili_url = meili_url.unwrap_or_else(|| config.meilisearch.url.clone());
            let meili_key = meili_key.or_else(|| config.meilisearch_api_key());
            let index_name = index_name.unwrap_or_else(|| config.meilisearch.index_name.clone());
            
            let indexer = MeilisearchIndexer::new(
                &meili_url,
                meili_key.as_deref(),
                &index_name,
            )
            .await
            .context("Failed to create Meilisearch indexer")?;

            let results = indexer.search(&query).await?;

            println!("\nFound {} results:", results.len());
            for (i, file) in results.iter().enumerate() {
                println!("{}. {}", i + 1, file.path.display());
            }
        }
        Commands::Organize { dir, dry_run, yes } => {
            println!("Organizing directory: {}", dir.display());
            
            let mover = FileMover::new(&dir)?;
            let generator = FolderGenerator::new();
            let mut preview = PreviewTree::new();

            // Walk directory and plan organization
            for entry in walkdir::WalkDir::new(&dir) {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                // Get file metadata and tags
                let metadata = std::fs::metadata(path)?;
                let extension = utils::get_extension(path);
                let extension_str = extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
                let hash = utils::compute_file_hash(path)?;
                let created_at = metadata
                    .created()
                    .or_else(|_| metadata.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::now());

                // Get handler before moving extension
                let registry = TaggerRegistry::new();
                let handler = registry.get_handler(extension_str);

                let file_meta = FileMeta::new(
                    path.to_path_buf(),
                    metadata.len(),
                    extension,
                    created_at,
                    hash,
                );

                let content = match handler.extract_text(&file_meta).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let tags = handler.generate_tags(&content).await?;

                // Generate folder name from tags
                let folder_name = generator.from_tags(&tags);
                let dest_dir = dir.join(&folder_name);
                let dest_file = dest_dir.join(
                    path.file_name()
                        .context("Failed to get file name")?
                );

                if path != dest_file {
                    preview.add_directory(dest_dir.clone());
                    preview.add_move(path.to_path_buf(), dest_file);
                }
            }

            if preview.is_empty() {
                println!("No files need to be organized.");
                return Ok(());
            }

            // Show preview
            println!("{}", preview.to_string());

            // Confirm if not auto-yes
            if !yes && !dry_run {
                use dialoguer::Confirm;
                let proceed = Confirm::new()
                    .with_prompt("Proceed with file reorganization?")
                    .default(false)
                    .interact()
                    .context("Failed to read user input")?;

                if !proceed {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            // Execute moves
            mover.execute(&preview, dry_run).await?;

            if dry_run {
                println!("\nDry run completed. No files were moved.");
            } else {
                println!("\nFiles organized successfully!");
            }
        }
    }

    Ok(())
}

