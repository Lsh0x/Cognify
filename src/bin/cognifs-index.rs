use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider, TeiEmbeddingProvider},
    file::FileFactory,
    indexer::{MeilisearchIndexer, SyncStats},
    llm::{LlmProvider, LocalLlmProvider},
    models::FileMeta,
    utils,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
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
    
    let indexer = MeilisearchIndexer::new(
        &meili_url,
        meili_key.as_deref(),
        &index_name,
    )
    .await
    .context("Failed to create Meilisearch indexer")?;

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

    // First, collect all files and their metadata for sync
    println!("📂 Scanning directory for changes...");
    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
    
    // First pass: collect all file paths (fast)
    for entry in WalkDir::new(&cli.dir) {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        file_paths.push(path.to_path_buf());
    }
    
    if file_paths.is_empty() {
        println!("No files found to index.");
        return Ok(());
    }
    
    println!("  Found {} files, computing hashes...", file_paths.len());
    
    // Second pass: compute hashes in parallel (faster)
    let mut all_files: Vec<FileMeta> = Vec::new();
    let mut handles = Vec::new();
    
    for path in file_paths {
        let handle = tokio::task::spawn_blocking({
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
        });
        handles.push(handle);
        
        // Process in batches of 50 to avoid too many concurrent tasks
        if handles.len() >= 50 {
            for handle in handles.drain(..) {
                match handle.await {
                    Ok(Ok(file_meta)) => all_files.push(file_meta),
                    Ok(Err(e)) => eprintln!("Warning: Failed to process file: {}", e),
                    Err(e) => eprintln!("Warning: Task error: {}", e),
                }
            }
            print!("\r  Processed {} files...", all_files.len());
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }
    
    // Process remaining handles
    for handle in handles {
        match handle.await {
            Ok(Ok(file_meta)) => all_files.push(file_meta),
            Ok(Err(e)) => eprintln!("Warning: Failed to process file: {}", e),
            Err(e) => eprintln!("Warning: Task error: {}", e),
        }
    }
    
    print!("\r  Processed {} files.                    \n", all_files.len());
    
    if all_files.is_empty() {
        println!("No valid files found to index.");
        return Ok(());
    }
    
    println!("  Ready to index {} files", all_files.len());
    
    // Sync index: remove deleted files, detect changes
    println!("🔄 Synchronizing index with Meilisearch...");
    let file_refs: Vec<&FileMeta> = all_files.iter().collect();
    let sync_stats = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        indexer.sync_index(&file_refs)
    ).await {
        Ok(Ok(stats)) => stats,
        Ok(Err(e)) => {
            eprintln!("Error: Sync failed: {}", e);
            eprintln!("Continuing with indexing anyway...");
            // Return empty stats to continue
            SyncStats {
                updated: 0,
                deleted: 0,
                unchanged: 0,
            }
        }
        Err(_) => {
            eprintln!("Warning: Sync timed out after 60 seconds");
            eprintln!("Continuing with indexing anyway...");
            // Return empty stats to continue
            SyncStats {
                updated: 0,
                deleted: 0,
                unchanged: 0,
            }
        }
    };
    
    // Calculate how many files will be newly indexed (not in sync stats)
    let total_files_to_index = all_files.iter()
        .filter(|f| !utils::is_inside_protected_structure_with_base(&f.path, Some(&cli.dir)))
        .count();
    let new_files = total_files_to_index.saturating_sub(sync_stats.unchanged + sync_stats.updated);
    
    println!("Sync results:");
    if sync_stats.unchanged > 0 {
        println!("  ✓ {} files unchanged", sync_stats.unchanged);
    }
    if sync_stats.updated > 0 {
        println!("  ↻ {} files will be updated (content changed)", sync_stats.updated);
    }
    if new_files > 0 {
        println!("  ➕ {} new files will be indexed", new_files);
    }
    if sync_stats.deleted > 0 {
        println!("  ✗ {} files removed from index (no longer exist)", sync_stats.deleted);
    }
    if sync_stats.unchanged == 0 && sync_stats.updated == 0 && new_files == total_files_to_index && total_files_to_index > 0 {
        println!("  ℹ️  Index is empty, all {} files will be indexed", total_files_to_index);
    }
    
    // Count files for progress bar (excluding protected for indexing count)
    let total_files: usize = all_files.iter()
        .filter(|f| !utils::is_inside_protected_structure_with_base(&f.path, Some(&cli.dir)))
        .count();
    
    if total_files == 0 {
        println!("No files to index (all files are in protected structures).");
        return Ok(());
    }

    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} files indexed ({msg})")
            .unwrap()
            .progress_chars("#>-")
    );

    // Walk directory and index files
    let mut count = 0;
    let mut embedding_count = 0;
    let mut embedding_failures = 0;
    let mut protected_count = 0;
    let mut llm_tag_count = 0;
    let mut llm_tag_failures = 0;
    
    for file_meta in all_files.iter() {
        let path = &file_meta.path;
        let is_protected = utils::is_inside_protected_structure_with_base(path, Some(&cli.dir));
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        if is_protected {
            // Protected files are still indexed but not counted in progress
            // Skip progress bar for protected files
        } else {
            pb.set_message(format!("Indexing: {}", file_name));
        }

        let extension_str = file_meta.extension.as_ref().map(|s| s.as_str()).unwrap_or("unknown");

        // Create SemanticFile via factory
        let semantic_source = FileFactory::create_from_meta(file_meta);
        
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
                    llm_tag_count += 1;
                    llm_tags
                }
                Err(e) => {
                    llm_tag_failures += 1;
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
                    embedding_failures += 1;
                    eprintln!("Warning: Empty embedding returned for {}, skipping", file_name);
                    None
                } else {
                    embedding_count += 1;
                    if embedding_count == 1 {
                        println!("  ✓ First embedding generated: {} dimensions", emb.len());
                    }
                    Some(emb)
                }
            }
            Err(e) => {
                embedding_failures += 1;
                eprintln!("Warning: Failed to generate embedding for {}: {}", file_name, e);
                None // Continue without embedding
            }
        };

        // Index with metadata and embedding (tags and text are NOT stored, only used for embedding)
        // ID is based on file hash + updated_at, so same content at different times = different documents
        match indexer.index_semantic_file(
            file_meta,
            &tags, // Passed but not stored - used only for embedding generation
            text.as_deref(), // Passed but not stored - used only for embedding generation
            file_metadata.as_ref(),
            embedding.as_deref(),
        ).await {
            Ok(()) => {
                count += 1;
                if count == 1 {
                    println!("  ✓ First file indexed successfully");
                }
            }
            Err(e) => {
                eprintln!("Error: Failed to index {}: {}", file_name, e);
                // Continue with next file instead of aborting
                continue;
            }
        }
        
        if is_protected {
            protected_count += 1;
        } else {
            pb.inc(1);
        }
    }

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

