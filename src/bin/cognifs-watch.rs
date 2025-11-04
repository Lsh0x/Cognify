use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider, TeiEmbeddingProvider},
    file::FileFactory,
    indexer::MeilisearchIndexer,
    models::FileMeta,
    utils,
    watcher::FileWatcher,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cognifs-watch")]
#[command(about = "Watch a directory for file changes")]
#[command(version)]
struct Cli {
    /// Directory to watch
    #[arg(value_name = "DIR")]
    dir: PathBuf,
    
    /// Automatically index files when they change (requires Meilisearch)
    #[arg(long)]
    auto_index: bool,
    
    /// Meilisearch URL (overrides config)
    #[arg(long)]
    meili_url: Option<String>,
    
    /// Meilisearch API key (overrides config and env)
    #[arg(long)]
    meili_key: Option<String>,
    
    /// Meilisearch index name (overrides config)
    #[arg(long)]
    index_name: Option<String>,
}

async fn index_file(
    path: &std::path::Path,
    indexer: &MeilisearchIndexer,
    embedding_provider: &dyn EmbeddingProvider,
    base_dir: &std::path::Path,
) -> Result<()> {
    // Skip if in protected structure
    if utils::is_inside_protected_structure_with_base(path, Some(base_dir)) {
        return Ok(());
    }

    // Get file metadata
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let extension = utils::get_extension(path);
    let hash = utils::compute_file_hash(path)?;
    let created_at = metadata
        .created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| std::time::SystemTime::now());
    let updated_at = metadata
        .modified()
        .or_else(|_| metadata.created())
        .unwrap_or_else(|_| std::time::SystemTime::now());

    let file_meta = FileMeta::new(
        path.to_path_buf(),
        metadata.len(),
        extension,
        created_at,
        updated_at,
        hash,
    );

    // Create SemanticSource for text extraction and tagging
    let semantic_source = FileFactory::create_from_meta(&file_meta);
    let text = semantic_source.to_text().await.ok();
    let file_metadata = semantic_source.to_metadata().await.ok().flatten();

    // Generate tags
    let content_for_tags = text.as_deref().unwrap_or("");
    let tags = semantic_source.generate_tags(content_for_tags).await?;

    // Generate embedding
    let extension_str = file_meta.extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let embedding_content = if let Some(ref txt) = text {
        if txt.trim().is_empty() || txt.len() < 10 {
            let mut fallback = format!("File: {}", file_name);
            if extension_str != "unknown" {
                fallback.push_str(&format!(" ({} file)", extension_str));
            }
            if !tags.is_empty() {
                fallback.push_str(". Tags: ");
                fallback.push_str(&tags.join(", "));
            }
            if fallback.len() < 20 {
                fallback.push_str(". Document file.");
            }
            fallback
        } else {
            txt.clone()
        }
    } else {
        let mut fallback = format!("File: {}", file_name);
        if extension_str != "unknown" {
            fallback.push_str(&format!(" ({} file)", extension_str));
        }
        if !tags.is_empty() {
            fallback.push_str(". Tags: ");
            fallback.push_str(&tags.join(", "));
        }
        if fallback.len() < 20 {
            fallback.push_str(". Document file.");
        }
        fallback
    };

    let embedding = embedding_provider.compute_embedding(&embedding_content).await.ok();

    // Index file
    indexer.index_semantic_file(
        &file_meta,
        &tags,
        text.as_deref(),
        file_metadata.as_ref(),
        embedding.as_deref(),
    )
    .await
    .context("Failed to index file")?;

    Ok(())
}

async fn delete_file_from_index(
    path: &std::path::Path,
    indexer: &MeilisearchIndexer,
) -> Result<()> {
    // Use the new delete_by_path method which handles multiple versions
    let deleted_count = indexer.delete_by_path(path).await
        .context("Failed to delete file from index")?;
    
    if deleted_count > 0 {
        Ok(())
    } else {
        // No documents found with this path, that's okay
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    
    println!("Watching directory: {}", cli.dir.display());
    
    // Initialize indexer and embedding provider if auto-indexing is enabled
    let (indexer, embedding_provider) = if cli.auto_index {
        let meili_url = cli.meili_url.unwrap_or_else(|| config.meilisearch.url.clone());
        let meili_key = cli.meili_key.or_else(|| config.meilisearch_api_key());
        let index_name = cli.index_name.unwrap_or_else(|| config.meilisearch.index_name.clone());
        
        let idx = MeilisearchIndexer::new(
            &meili_url,
            meili_key.as_deref(),
            &index_name,
        )
        .await
        .context("Failed to create Meilisearch indexer")?;
        
        let emb_provider: Box<dyn EmbeddingProvider> = if config.embedding_provider == "tei" {
            Box::new(TeiEmbeddingProvider::new(
                Some(&config.tei.url),
                Some(config.tei.dims),
            ))
        } else {
            let ollama_url = config.ollama.url.as_str();
            let embedding_model = config.ollama.model.as_str();
            let embedding_dims = config.ollama.dims;
            Box::new(LocalEmbeddingProvider::new(
                Some(ollama_url),
                Some(embedding_model),
                Some(embedding_dims),
            ))
        };
        
        println!("✓ Auto-indexing enabled - files will be indexed when they change");
        println!("  Using Meilisearch index: {}", index_name);
        println!("  Using embedding provider: {} ({} dimensions)", 
                 if config.embedding_provider == "tei" { "TEI" } else { &config.ollama.model },
                 emb_provider.dimension());
        
        (Some(idx), Some(emb_provider))
    } else {
        println!("Watcher started. Press Ctrl+C to stop.");
        println!("ℹ️  Use --auto-index to automatically index files when they change");
        (None, None)
    };
    
    let watcher = FileWatcher::new(&cli.dir)?;
    let mut rx = watcher.subscribe();
    
    watcher.watch().await?;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(cognifs::watcher::WatchEvent::Created(meta)) => {
                        println!("Created: {}", meta.path.display());
                        if let (Some(ref idx), Some(ref emb)) = (&indexer, &embedding_provider) {
                            match index_file(&meta.path, idx, emb.as_ref(), &cli.dir).await {
                                Ok(_) => println!("  ✓ Indexed"),
                                Err(e) => eprintln!("  ⚠️  Failed to index: {}", e),
                            }
                        }
                    }
                    Ok(cognifs::watcher::WatchEvent::Modified(meta)) => {
                        println!("Modified: {}", meta.path.display());
                        if let (Some(ref idx), Some(ref emb)) = (&indexer, &embedding_provider) {
                            match index_file(&meta.path, idx, emb.as_ref(), &cli.dir).await {
                                Ok(_) => println!("  ✓ Updated in index"),
                                Err(e) => eprintln!("  ⚠️  Failed to update index: {}", e),
                            }
                        }
                    }
                    Ok(cognifs::watcher::WatchEvent::Deleted(path)) => {
                        println!("Deleted: {}", path.display());
                        if let Some(ref idx) = indexer {
                            match delete_file_from_index(&path, idx).await {
                                Ok(_) => println!("  ✓ Removed from index"),
                                Err(e) => eprintln!("  ⚠️  Failed to remove from index: {}", e),
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok(())
}

