use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    constants::{
        ARCHIVE_EXTENSIONS, AUDIO_EXTENSIONS, DOCUMENT_EXTENSIONS,
        IMAGE_EXTENSIONS, SPREADSHEET_EXTENSIONS, VIDEO_EXTENSIONS,
    },
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider, TeiEmbeddingProvider},
    file::FileFactory,
    indexer::{Indexer, MeilisearchIndexer},
    llm::{LlmProvider, LocalLlmProvider},
    models::FileMeta,
    organizer::{EmbeddingClusterer, FileMover, FolderGenerator, PreviewTree},
    utils,
};
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Clone)]
struct FilePlan {
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    folder_path: std::path::PathBuf,
    embedding: Option<Vec<f32>>,
    file_meta: FileMeta,
    tags: Vec<String>,
    text: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Parser)]
#[command(name = "cognifs-organize")]
#[command(about = "Organize files into folders based on tags")]
#[command(version)]
struct Cli {
    /// Directory to organize
    #[arg(value_name = "DIR")]
    dir: PathBuf,
    
    /// Dry run (preview only, don't move files)
    #[arg(long)]
    dry_run: bool,
    
    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
    
    /// Use LLM for intelligent tag generation (requires LLM configured)
    #[arg(long)]
    use_llm: bool,
    
    /// Index files to Meilisearch after organizing
    #[arg(long)]
    index: bool,
    
    /// Meilisearch URL (overrides config, required if --index)
    #[arg(long)]
    meili_url: Option<String>,
    
    /// Meilisearch API key (overrides config and env)
    #[arg(long)]
    meili_key: Option<String>,
    
    /// Meilisearch index name (overrides config)
    #[arg(long)]
    index_name: Option<String>,
    
    /// Skip sync step when indexing (faster for initial indexing, assumes empty index)
    #[arg(long)]
    init: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    
    println!("Organizing directory: {}", cli.dir.display());
    
    let mover = FileMover::new(&cli.dir)?;
    let generator = FolderGenerator::new();
    let mut preview = PreviewTree::new();
    let mut existing_directories = HashSet::new();
    let mut skipped_git_files = 0;

    // Initialize LLM provider if requested
    let llm_provider: Option<LocalLlmProvider> = if cli.use_llm {
        let llm = LocalLlmProvider::from_config(&config);
        // Check if LLM is available
        if llm.model_exists() {
            println!("✓ Using LLM for intelligent tag generation");
            Some(llm)
        } else {
            println!("⚠️  LLM model not found, falling back to dictionary-based tagging");
            None
        }
    } else {
        None
    };

    // Initialize indexer if --index is enabled
    let indexer: Option<MeilisearchIndexer> = if cli.index {
        let meili_url = cli.meili_url.unwrap_or_else(|| config.meilisearch.url.clone());
        let meili_key = cli.meili_key.or_else(|| config.meilisearch_api_key());
        let index_name = cli.index_name.unwrap_or_else(|| config.meilisearch.index_name.clone());
        
        Some(MeilisearchIndexer::new(
            &meili_url,
            meili_key.as_deref(),
            &index_name,
        )
        .await
        .context("Failed to create Meilisearch indexer")?)
    } else {
        None
    };

    // Count files first for progress bar (including protected structures for analysis)
    let mut total_files = 0;
    let mut protected_files = 0;
    let mut total_entries = 0;
    
    for entry in WalkDir::new(&cli.dir) {
        match entry {
            Ok(e) => {
                total_entries += 1;
                if e.path().is_file() {
                    total_files += 1;
                    if utils::is_inside_protected_structure_with_base(e.path(), Some(&cli.dir)) {
                        protected_files += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Error accessing entry: {}", e);
            }
        }
    }

    if total_files == 0 {
        println!("No files found to organize.");
        if total_entries == 0 {
            println!("  ℹ️  The directory appears to be empty or inaccessible.");
        }
        return Ok(());
    }
    
    if protected_files > 0 {
        println!("ℹ️  {} file(s) in protected structures will be analyzed (tags + embeddings) but not moved", protected_files);
    }

    // Create progress bar (includes protected files for analysis)
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} files processed ({msg})")
            .unwrap()
            .progress_chars("#>-")
    );

    // PHASE 1: Collect all files, embeddings, tags, and planned destinations
    // This allows us to use embeddings for clustering and create folders based on semantic similarity
    let mut file_plans: Vec<FilePlan> = Vec::new();
    let mut file_embeddings: Vec<(usize, Vec<f32>)> = Vec::new(); // (file_index, embedding)
    let mut file_tags_map: HashMap<usize, Vec<String>> = HashMap::new(); // (file_index, tags)
    
    // Initialize embedding provider for semantic clustering
    let embedding_provider: Box<dyn EmbeddingProvider> = if config.embedding_provider == "tei" {
        println!("📊 Using TEI embeddings for semantic clustering ({} dimensions)", config.tei.dims);
        Box::new(TeiEmbeddingProvider::new(
            Some(&config.tei.url),
            Some(config.tei.dims),
        ))
    } else {
        let ollama_url = config.ollama.url.as_str();
        let embedding_model = config.ollama.model.as_str();
        let embedding_dims = config.ollama.dims;
        println!("📊 Using Ollama embeddings for semantic clustering (model: {}, {} dimensions)", 
                 embedding_model, embedding_dims);
        Box::new(LocalEmbeddingProvider::new(
            Some(ollama_url),
            Some(embedding_model),
            Some(embedding_dims),
        ))
    };
    
    println!("  ✓ Embedding dimension: {}", embedding_provider.dimension());

    // Walk directory and plan organization (collect phase)
    for entry in WalkDir::new(&cli.dir) {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Check if file is in protected structure
        let is_protected = utils::is_inside_protected_structure_with_base(path, Some(&cli.dir));
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        if is_protected {
            pb.set_message(format!("Analyzing (protected): {}", file_name));
        } else {
            pb.set_message(format!("Analyzing: {}", file_name));
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
        let updated_at = metadata
            .modified()
            .or_else(|_| metadata.created())
            .unwrap_or_else(|_| std::time::SystemTime::now());

        let file_meta = FileMeta::new(
            path.to_path_buf(),
            metadata.len(),
            extension.clone(),
            created_at,
            updated_at,
            hash,
        );

        // Create SemanticFile via factory
        let semantic_source = FileFactory::create_from_meta(&file_meta);
        
        // Extract text and metadata using SemanticSource
        let text = match semantic_source.to_text().await {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Warning: Could not extract text from {}: {}", file_name, e);
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

        // Generate tags using SemanticSource (already includes path-based tags)
        let content_for_tags = text.as_deref().unwrap_or("");
        let mut tags = semantic_source.generate_tags(content_for_tags).await?;
        
        // Extract tags from file extension if meaningful
        if extension_str != "unknown" {
            let ext_lower = extension_str.to_lowercase();
            if DOCUMENT_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"document".to_string()) {
                    tags.push("document".to_string());
                }
            } else if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"image".to_string()) {
                    tags.push("image".to_string());
                }
            } else if VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"video".to_string()) {
                    tags.push("video".to_string());
                }
            } else if AUDIO_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"audio".to_string()) {
                    tags.push("audio".to_string());
                }
            } else if ARCHIVE_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"archive".to_string()) {
                    tags.push("archive".to_string());
                }
            } else if SPREADSHEET_EXTENSIONS.contains(&ext_lower.as_str()) {
                if !tags.contains(&"spreadsheet".to_string()) {
                    tags.push("spreadsheet".to_string());
                }
            }
        }
        
        // Enhance tags with LLM if available and enabled
        // Pass both text content and file path for better context-aware tagging
        if let Some(ref llm) = llm_provider {
            let content_for_llm = text.as_deref().unwrap_or("");
            match llm.generate_tags(content_for_llm, path).await {
                Ok(llm_tags) => {
                    // Merge LLM tags with dictionary tags (avoid duplicates)
                    for llm_tag in llm_tags {
                        if !tags.contains(&llm_tag) {
                            tags.push(llm_tag);
                        }
                    }
                }
                Err(e) => {
                    // LLM failed, but continue with dictionary tags
                    eprintln!("Warning: LLM tag generation failed for {}: {}", file_name, e);
                }
            }
        }
        
        // Remove "unknown" tag if we have other meaningful tags
        if tags.len() > 1 && tags.contains(&"unknown".to_string()) {
            tags.retain(|t| t != "unknown");
        }

        // Generate embedding for semantic clustering
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
                    eprintln!("Warning: Empty embedding returned for {}, skipping", file_name);
                    None
                } else {
                    Some(emb)
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to generate embedding for {}: {}", file_name, e);
                None
            }
        };

        // Check for existing matching directory first (try hierarchical matching)
        let folder_path = match generator.find_matching_directory_hierarchical(&tags, &cli.dir) {
            Some(existing_path) => {
                // Track all directories in the hierarchical path as existing
                let mut current_track = cli.dir.to_path_buf();
                for component in existing_path.components() {
                    if let std::path::Component::Normal(comp) = component {
                        current_track.push(comp);
                        let path_str = current_track.strip_prefix(&cli.dir)
                            .unwrap_or(&current_track)
                            .to_string_lossy()
                            .to_string();
                        existing_directories.insert(path_str);
                    }
                }
                existing_path
            }
            None => {
                // Generate hierarchical folder path from tags (max 3-4 levels for better organization)
                generator.from_tags_hierarchical(&tags, 4)
            }
        };

        // Only add to file_plans if NOT protected (protected files are analyzed but not moved)
        if !is_protected {
            let dest_dir = cli.dir.join(&folder_path);
            let dest_file = dest_dir.join(
                path.file_name()
                    .context("Failed to get file name")?
            );

            if path != dest_file {
                // Calculate file_index AFTER checking protection, so it matches file_plans index
                let file_index = file_plans.len();
                
                // Store embedding and tags for clustering (ONLY for non-protected files)
                // This ensures indices in file_embeddings/file_tags_map match file_plans indices
                if let Some(ref emb) = embedding {
                    file_embeddings.push((file_index, emb.clone()));
                }
                file_tags_map.insert(file_index, tags.clone());
                
                file_plans.push(FilePlan {
                    source: path.to_path_buf(),
                    destination: dest_file,
                    folder_path: folder_path.clone(),
                    embedding: embedding.clone(),
                    file_meta: file_meta.clone(),
                    tags: tags.clone(),
                    text: text.clone(),
                    metadata: file_metadata.clone(),
                });
            }
        } else {
            // Protected file: analyzed (tags + embeddings) but not moved
            // Do NOT add to file_embeddings/file_tags_map because it's not in file_plans
            skipped_git_files += 1;
        }

        pb.inc(1);
    }

    pb.finish_with_message("Analysis complete!");
    
    if file_plans.is_empty() {
        println!("\n✓ All files are already organized!");
        println!("  ℹ️  {} file(s) analyzed (tags + embeddings generated).", total_files);
        if skipped_git_files > 0 {
            println!("  ℹ️  {} file(s) in protected structures were analyzed but not moved.", skipped_git_files);
        }
        if total_files - skipped_git_files > 0 {
            println!("  ℹ️  {} file(s) are already in their correct location.", total_files - skipped_git_files);
        }
        return Ok(());
    }
    
    println!("\n📊 Organizing {} files...", file_plans.len());
    
    // Use embeddings for clustering if available
    let clusterer = EmbeddingClusterer::new(0.7); // 70% similarity threshold
    
    if !file_embeddings.is_empty() {
        println!("🔍 Clustering {} files by semantic similarity...", file_embeddings.len());
        let clusters = clusterer.cluster_files(&file_embeddings, &file_tags_map);
        println!("  ✓ Found {} semantic clusters", clusters.len());
        
        // Update folder paths based on clusters
        // Files in the same cluster should be grouped together
        for (_cluster_id, cluster) in &clusters {
            // Use dominant tags from cluster to generate folder name
            let cluster_folder_path = generator.from_tags_hierarchical(&cluster.dominant_tags, 4);
            
            // Update folder_path for all files in this cluster
            for (file_idx, _) in &cluster.files {
                if *file_idx < file_plans.len() {
                    // If cluster has meaningful tags, use them; otherwise keep original tags
                    if !cluster.dominant_tags.is_empty() {
                        // Check if we should use cluster-based folder or keep original
                        // For now, prefer cluster-based if it's more specific
                        let original_path = &file_plans[*file_idx].folder_path;
                        let original_depth = original_path.components().count();
                        let cluster_depth = cluster_folder_path.components().count();
                        
                        // Use cluster folder if it's at least as specific (same or more depth)
                        if cluster_depth >= original_depth || cluster.dominant_tags.len() > 2 {
                            file_plans[*file_idx].folder_path = cluster_folder_path.clone();
                        }
                    }
                }
            }
        }
        
        // Update destinations after clustering
        for plan in &mut file_plans {
            let dest_dir = cli.dir.join(&plan.folder_path);
            plan.destination = dest_dir.join(
                plan.source.file_name()
                    .context("Failed to get file name")?
            );
        }
    } else {
        println!("⚠️  No embeddings available, using tag-based organization only");
    }
    
    println!("📁 Organizing by tag frequency...");

    // PHASE 2: Group files by hierarchical level and count tag frequency
    // We'll create folders level by level, starting with the most frequent tags
    
    // Group by complete folder path for file moves
    let mut folder_groups: HashMap<String, Vec<FilePlan>> = HashMap::new();
    
    // Count files per level to prioritize creation
    // Level 1: top-level categories (e.g., "document", "programming")
    let mut level1_counts: HashMap<String, usize> = HashMap::new();
    // Level 2: second-level categories (e.g., "document/financial")
    let mut level2_counts: HashMap<(String, String), usize> = HashMap::new();
    // Level 3: third-level categories (e.g., "document/financial/invoice")
    let mut level3_counts: HashMap<(String, String, String), usize> = HashMap::new();
    
    for plan in file_plans {
        let folder_key = plan.folder_path.to_string_lossy().to_string();
        folder_groups.entry(folder_key)
            .or_insert_with(Vec::new)
            .push(plan.clone());
        
        // Count at each hierarchical level
        let components: Vec<String> = plan.folder_path.components()
            .filter_map(|c| {
                if let std::path::Component::Normal(comp) = c {
                    comp.to_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        
        // Count level 1 (first component)
        if let Some(level1) = components.get(0) {
            *level1_counts.entry(level1.clone()).or_insert(0) += 1;
            
            // Count level 2 (first two components)
            if let Some(level2) = components.get(1) {
                let key = (level1.clone(), level2.clone());
                *level2_counts.entry(key).or_insert(0) += 1;
                
                // Count level 3 (first three components)
                if let Some(level3) = components.get(2) {
                    let key = (level1.clone(), level2.clone(), level3.clone());
                    *level3_counts.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    // PHASE 3: Sort levels by file count (descending)
    // Create directories level by level, starting with the most frequent tags
    let mut sorted_level1: Vec<(String, usize)> = level1_counts.into_iter().collect();
    sorted_level1.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    
    let mut sorted_level2: Vec<((String, String), usize)> = level2_counts.into_iter().collect();
    sorted_level2.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    
    let mut sorted_level3: Vec<((String, String, String), usize)> = level3_counts.into_iter().collect();
    sorted_level3.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    println!("📁 Organization summary:");
    println!("   Level 1 (top-level): {} unique folders", sorted_level1.len());
    if !sorted_level1.is_empty() {
        println!("   Most frequent: {} ({} files)", sorted_level1[0].0, sorted_level1[0].1);
    }

    // PHASE 4: Build preview tree by creating folders level by level
    // Start with level 1 (most frequent tags first)
    for (level1_name, _count) in &sorted_level1 {
        let level1_path = cli.dir.join(level1_name);
        let dir_path_str = level1_path.strip_prefix(&cli.dir)
            .unwrap_or(&level1_path)
            .to_string_lossy()
            .to_string();
        
        let exists = level1_path.exists() && level1_path.is_dir();
        if !exists && !existing_directories.contains(&dir_path_str) && level1_path != cli.dir {
            preview.add_directory(level1_path.clone());
            existing_directories.insert(dir_path_str);
        } else if exists {
            existing_directories.insert(dir_path_str);
        }
    }

    // Then create level 2 folders (most frequent combinations first)
    for ((level1_name, level2_name), _count) in &sorted_level2 {
        let level2_path = cli.dir.join(level1_name).join(level2_name);
        let dir_path_str = level2_path.strip_prefix(&cli.dir)
            .unwrap_or(&level2_path)
            .to_string_lossy()
            .to_string();
        
        let exists = level2_path.exists() && level2_path.is_dir();
        if !exists && !existing_directories.contains(&dir_path_str) && level2_path != cli.dir {
            preview.add_directory(level2_path.clone());
            existing_directories.insert(dir_path_str);
        } else if exists {
            existing_directories.insert(dir_path_str);
        }
    }

    // Then create level 3 folders (most frequent combinations first)
    for ((level1_name, level2_name, level3_name), _count) in &sorted_level3 {
        let level3_path = cli.dir.join(level1_name).join(level2_name).join(level3_name);
        let dir_path_str = level3_path.strip_prefix(&cli.dir)
            .unwrap_or(&level3_path)
            .to_string_lossy()
            .to_string();
        
        let exists = level3_path.exists() && level3_path.is_dir();
        if !exists && !existing_directories.contains(&dir_path_str) && level3_path != cli.dir {
            preview.add_directory(level3_path.clone());
            existing_directories.insert(dir_path_str);
        } else if exists {
            existing_directories.insert(dir_path_str);
        }
    }

    // Finally, add all file moves (sorted by folder path for consistency)
    let mut sorted_folders: Vec<(String, Vec<FilePlan>)> = folder_groups.into_iter().collect();
    sorted_folders.sort_by_key(|(path, _)| path.clone());
    
    let mut all_file_plans: Vec<FilePlan> = Vec::new();
    
    for (_, files) in &sorted_folders {
        for file_plan in files {
            preview.add_move(file_plan.source.clone(), file_plan.destination.clone());
            all_file_plans.push(file_plan.clone());
        }
    }

    if skipped_git_files > 0 {
        println!("\n⚠️  Skipped {} file(s) inside protected structures (Git repos, project directories, etc.)", skipped_git_files);
    }

    if preview.is_empty() {
        println!("\n✓ All files are already organized!");
        let analyzed_count = total_files;
        println!("  ℹ️  {} file(s) were analyzed.", analyzed_count);
        if skipped_git_files > 0 {
            println!("  ℹ️  {} file(s) were skipped (protected structures).", skipped_git_files);
        }
        return Ok(());
    }

    // Show preview with current and new structure
    println!("\n{}", preview.to_string(&cli.dir));

    // Confirm if not auto-yes
    if !cli.yes && !cli.dry_run {
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
    mover.execute(&preview, cli.dry_run).await?;

    if cli.dry_run {
        println!("\nDry run completed. No files were moved.");
    } else {
        println!("\nFiles organized successfully!");
    }

    // Index files after organizing if requested
    if cli.index && !cli.dry_run {
        if let Some(ref idx) = indexer {
            if cli.init {
                println!("\n📝 Indexing organized files (--init: skipping sync checks)...");
            } else {
                println!("\n📝 Indexing organized files...");
            }
            let mut indexed = 0;
            let mut failed = 0;
            
            for plan in &all_file_plans {
                // Use destination path after move
                let path = &plan.destination;
                
                // Recompute file_meta with new path (file may have moved)
                let metadata = match std::fs::metadata(path) {
                    Ok(m) => m,
                    Err(_) => {
                        failed += 1;
                        continue;
                    }
                };
                
                let hash = match utils::compute_file_hash(path) {
                    Ok(h) => h,
                    Err(_) => {
                        failed += 1;
                        continue;
                    }
                };
                
                let updated_at = metadata
                    .modified()
                    .or_else(|_| metadata.created())
                    .unwrap_or_else(|_| std::time::SystemTime::now());
                
                let file_meta = FileMeta::new(
                    path.clone(),
                    metadata.len(),
                    plan.file_meta.extension.clone(),
                    plan.file_meta.created_at,
                    updated_at,
                    hash,
                );
                
                // Index with tags, text, and embedding we already computed
                if let Err(e) = idx.index_semantic_file(
                    &file_meta,
                    &plan.tags,
                    plan.text.as_deref(),
                    plan.metadata.as_ref(),
                    plan.embedding.as_deref(),
                ).await {
                    eprintln!("Warning: Failed to index {}: {}", path.display(), e);
                    failed += 1;
                } else {
                    indexed += 1;
                }
            }
            
            println!("✓ Indexed {} files", indexed);
            if failed > 0 {
                println!("  ⚠️  Failed to index {} files", failed);
            }
        }
    }

    Ok(())
}

