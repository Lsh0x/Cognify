use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider, TeiEmbeddingProvider},
    file::FileFactory,
    indexer::MeilisearchIndexer,
    llm::{LlmProvider, LocalLlmProvider},
    models::FileMeta,
    utils,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "cognifs-index")]
#[command(about = "Index files in a directory to Meilisearch")]
#[command(version)]
struct Cli {
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
    
    /// Enable LLM-based tag generation (very slow: ~30s per file)
    #[arg(long)]
    use_llm: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    
    println!("Indexing directory: {}", cli.dir.display());
    
    let meili_url = cli.meili_url.unwrap_or_else(|| config.meilisearch.url.clone());
    let meili_key = cli.meili_key.or_else(|| config.meilisearch_api_key());
    let index_name = cli.index_name.unwrap_or_else(|| config.meilisearch.index_name.clone());
    
    let indexer = Arc::new(MeilisearchIndexer::new(
        &meili_url,
        meili_key.as_deref(),
        &index_name,
    )
    .await
    .context("Failed to create Meilisearch indexer")?);

    // Initialize LLM provider for tag generation (disabled by default due to performance)
    let llm_provider = LocalLlmProvider::from_config(&config);
    let use_llm = cli.use_llm && llm_provider.model_exists();
    if cli.use_llm {
        if llm_provider.model_exists() {
            println!("✓ Using LLM for intelligent tag generation (slow: ~30s per file)");
        } else {
            println!("⚠️  --use-llm specified but LLM model not found, using dictionary-based tagging");
        }
    } else {
        if llm_provider.model_exists() {
            println!("ℹ️  LLM model found but disabled (use --use-llm to enable, adds ~30s per file)");
        }
        println!("✓ Using dictionary-based tagging (fast)");
    }

    // Initialize embedding provider for semantic search
    let embedding_provider: Box<dyn EmbeddingProvider> = if config.embedding_provider == "tei" {
        println!("📊 Using TEI embeddings ({} dimensions)", config.tei.dims);
        Box::new(TeiEmbeddingProvider::new(
            Some(&config.tei.url),
            Some(config.tei.dims),
        ))
    } else {
        let ollama_url = config.ollama.url.as_str();
        let embedding_model = config.ollama.model.as_str();
        let embedding_dims = config.ollama.dims;
        println!("📊 Generating embeddings with Ollama model: {} ({} dimensions)", 
                 embedding_model, embedding_dims);
        Box::new(LocalEmbeddingProvider::new(
            Some(ollama_url),
            Some(embedding_model),
            Some(embedding_dims),
        ))
    };
    
    println!("  ✓ Embedding dimension: {}", embedding_provider.dimension());

    // Process files as we discover them (streaming, no sync, no need to store everything)
    println!("📂 Scanning and indexing files...");
    
    // Use futures for concurrent processing
    use futures::stream::{self, StreamExt};
    
    // Determine concurrency limits
    let max_concurrent = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
        .min(16);
    
    println!("🚀 Starting to process files (max {} concurrent operations)...", max_concurrent);
    
    // Create progress bar (will update as we discover files)
    let pb = ProgressBar::new(0); // Start with unknown count
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos} files indexed ({msg})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Thread-safe counters
    let count = Arc::new(std::sync::Mutex::new(0));
    let embedding_count = Arc::new(std::sync::Mutex::new(0));
    let embedding_failures = Arc::new(std::sync::Mutex::new(0));
    let protected_count = Arc::new(std::sync::Mutex::new(0));
    let llm_tag_count = Arc::new(std::sync::Mutex::new(0));
    let llm_tag_failures = Arc::new(std::sync::Mutex::new(0));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    
    // Stream file paths as we discover them using a channel
    // This allows processing to start immediately without collecting all paths first
    println!("  Scanning directory structure...");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Spawn task to walk directory and send paths through channel
    let dir_clone = cli.dir.clone();
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let mut count = 0;
            for entry in WalkDir::new(&dir_clone) {
                match entry {
                    Ok(e) => {
                        if e.path().is_file() {
                            let path = e.path().to_path_buf();
                            if tx.send(path).is_err() {
                                break; // Receiver dropped, stop walking
                            }
                            count += 1;
                            // Print progress every 10k files
                            if count % 10000 == 0 {
                                eprintln!("  Found {} files so far...", count);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Error accessing entry: {}", e);
                    }
                }
            }
            eprintln!("  Found {} files total", count);
        })
        .await
        .ok();
    });
    
    // Convert channel receiver to stream using tokio_stream
    use tokio_stream::wrappers::UnboundedReceiverStream;
    let rx_stream = UnboundedReceiverStream::new(rx);
    
    // Process files concurrently as we receive them from the channel
    let mut stream = rx_stream
        .map(|path| {
            let semaphore = semaphore.clone();
            let indexer = indexer.clone();
            let embedding_provider = embedding_provider.as_ref();
            let llm_provider = &llm_provider;
            let cli_dir = cli.dir.clone();
            let use_llm = use_llm;
            let pb = pb.clone();
            let count = count.clone();
            let embedding_count = embedding_count.clone();
            let embedding_failures = embedding_failures.clone();
            let protected_count = protected_count.clone();
            let llm_tag_count = llm_tag_count.clone();
            let llm_tag_failures = llm_tag_failures.clone();
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                // Compute file metadata (hash, etc.) in blocking task
                let file_meta = match tokio::task::spawn_blocking({
                    let path = path.clone();
                    move || -> Result<FileMeta> {
                        let metadata = std::fs::metadata(&path)?;
                        let extension = utils::get_extension(&path);
                        let hash = utils::compute_file_hash(&path)?;
                        let created_at = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .unwrap_or_else(|_| std::time::SystemTime::now());
                        let updated_at = metadata
                            .modified()
                            .or_else(|_| metadata.created())
                            .unwrap_or_else(|_| std::time::SystemTime::now());
                        
                        Ok(FileMeta::new(
                            path,
                            metadata.len(),
                            extension,
                            created_at,
                            updated_at,
                            hash,
                        ))
                    }
                }).await {
                    Ok(Ok(meta)) => meta,
                    Ok(Err(e)) => {
                        eprintln!("Warning: Failed to process file: {}", e);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Warning: Task error: {}", e);
                        return Ok(());
                    }
                };
                
                let path = &file_meta.path;
                let is_protected = utils::is_inside_protected_structure_with_base(path, Some(&cli_dir));
                
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                
                if !is_protected {
                    pb.set_message(format!("Indexing: {}", file_name));
                    pb.inc_length(1); // Update total count as we discover files
                }

                let extension_str = file_meta.extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");

                // Create SemanticFile via factory
                let semantic_source = FileFactory::create_from_meta(&file_meta);
                
                // Extract text and metadata using SemanticSource
                let text = match semantic_source.to_text().await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        eprintln!("Warning: Failed to extract text from {}: {}", file_name, e);
                        None
                    }
                };
                
                let file_metadata = match semantic_source.to_metadata().await {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Warning: Failed to extract metadata from {}: {}", file_name, e);
                        None
                    }
                };

                // Generate tags using LLM if available, otherwise use SemanticSource
                let content_for_tags = text.as_deref().unwrap_or("");
                let tags = if use_llm {
                    match llm_provider.generate_tags(content_for_tags, path).await {
                        Ok(llm_tags) => {
                            *llm_tag_count.lock().unwrap() += 1;
                            llm_tags
                        }
                        Err(e) => {
                            *llm_tag_failures.lock().unwrap() += 1;
                            eprintln!("Warning: LLM tag generation failed for {}: {}, falling back to dictionary", file_name, e);
                            // Fallback to dictionary-based tags
                            match semantic_source.generate_tags(content_for_tags).await {
                                Ok(dict_tags) => dict_tags,
                                Err(e2) => {
                                    eprintln!("Warning: Dictionary tag generation also failed for {}: {}, using empty tags", file_name, e2);
                                    Vec::new()
                                }
                            }
                        }
                    }
                } else {
                    // Use dictionary-based tagging from SemanticSource
                    match semantic_source.generate_tags(content_for_tags).await {
                        Ok(dict_tags) => dict_tags,
                        Err(e) => {
                            eprintln!("Warning: Dictionary tag generation failed for {}: {}, using empty tags", file_name, e);
                            Vec::new()
                        }
                    }
                };

                // Generate embedding for semantic search
                // Use extracted text if available, otherwise fallback to filename + tags
                let embedding_content = if let Some(ref txt) = text {
                    if txt.trim().is_empty() || txt.len() < 10 {
                        // Build fallback content from filename and tags
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
                    // No text extracted, use fallback
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

                let embedding = match embedding_provider.compute_embedding(&embedding_content).await {
                    Ok(emb) => {
                        // Validate embedding is not empty
                        if emb.is_empty() {
                            *embedding_failures.lock().unwrap() += 1;
                            eprintln!("Warning: Empty embedding returned for {}, skipping", file_name);
                            None
                        } else {
                            let mut ec = embedding_count.lock().unwrap();
                            *ec += 1;
                            if *ec == 1 {
                                println!("  ✓ First embedding generated: {} dimensions", emb.len());
                            }
                            Some(emb)
                        }
                    }
                    Err(e) => {
                        *embedding_failures.lock().unwrap() += 1;
                        eprintln!("Warning: Failed to generate embedding for {}: {}", file_name, e);
                        None // Continue without embedding
                    }
                };

                // Index with metadata and embedding
                let result = match indexer.index_semantic_file(
                    &file_meta,
                    &tags,
                    text.as_deref(),
                    file_metadata.as_ref(),
                    embedding.as_deref(),
                ).await {
                    Ok(()) => {
                        let mut c = count.lock().unwrap();
                        *c += 1;
                        if *c == 1 {
                            println!("  ✓ First file indexed successfully");
                        }
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to index {}: {}", file_name, e);
                        Err(e)
                    }
                };
                
                if is_protected {
                    *protected_count.lock().unwrap() += 1;
                } else {
                    pb.inc(1);
                }
                
                result
            }
        })
        .buffer_unordered(max_concurrent);
    
    // Process all files concurrently
    while let Some(result) = stream.next().await {
        // Results are already handled in the closure above
        let _ = result;
    }
    
    // Extract final counts
    let count = *count.lock().unwrap();
    let embedding_count = *embedding_count.lock().unwrap();
    let embedding_failures = *embedding_failures.lock().unwrap();
    let protected_count = *protected_count.lock().unwrap();
    let llm_tag_count = *llm_tag_count.lock().unwrap();
    let llm_tag_failures = *llm_tag_failures.lock().unwrap();

    pb.finish_with_message("Indexing complete!");
    println!("\n✓ Indexed {} files total", count);
    if embedding_count > 0 {
        println!("  ✓ Generated embeddings for {} files", embedding_count);
    }
    if embedding_failures > 0 {
        println!("  ⚠️  Failed to generate embeddings for {} files (will continue without embeddings)", embedding_failures);
    }
    if use_llm {
        if llm_tag_count > 0 {
            println!("  ✓ Generated LLM tags for {} files", llm_tag_count);
        }
        if llm_tag_failures > 0 {
            println!("  ⚠️  LLM tag generation failed for {} files (fell back to dictionary)", llm_tag_failures);
        }
    }
    if protected_count > 0 {
        println!("  ℹ️  {} file(s) in protected structures were indexed but not moved", protected_count);
    }

    Ok(())
}

