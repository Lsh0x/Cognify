use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    embeddings::{EmbeddingProvider, LocalEmbeddingProvider, MultiOllamaEmbeddingProvider, TeiEmbeddingProvider},
    file::FileFactory,
    indexer::{Indexer, MeilisearchIndexer},
    models::FileMeta,
    utils,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cognifs-tag")]
#[command(about = "Tag a single file and optionally save to index")]
#[command(version)]
struct Cli {
    /// File to tag
    #[arg(value_name = "FILE")]
    file: PathBuf,
    
    /// Save tags to Meilisearch index
    #[arg(long)]
    save: bool,
    
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    
    println!("Tagging file: {}", cli.file.display());
    
    // Get file metadata
    let metadata = std::fs::metadata(&cli.file)
        .with_context(|| format!("Failed to read file: {}", cli.file.display()))?;
    
    let extension = utils::get_extension(&cli.file);
    let hash = utils::compute_file_hash(&cli.file)?;
    let created_at = metadata
        .created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| std::time::SystemTime::now());
    let updated_at = metadata
        .modified()
        .or_else(|_| metadata.created())
        .unwrap_or_else(|_| std::time::SystemTime::now());

    let file_meta = FileMeta::new(
        cli.file.clone(),
        metadata.len(),
        extension,
        created_at,
        updated_at,
        hash,
    );

    // Use SemanticSource for text extraction and tagging
    let semantic_source = FileFactory::create_from_meta(&file_meta);
    let content = semantic_source.to_text().await?;
    println!("Extracted {} bytes of text", content.len());

    // Generate tags
    let tags = semantic_source.generate_tags(&content).await?;
    println!("Tags: {:?}", tags);

    // Optionally compute embedding
    let embedding_provider: Box<dyn EmbeddingProvider> = if config.embedding_provider == "tei" {
        Box::new(TeiEmbeddingProvider::new(
            Some(&config.tei.url),
            Some(config.tei.dims),
        ))
    } else {
        let embedding_model = config.ollama.model.as_str();
        let embedding_dims = config.ollama.dims;
        
        // Use multi-Ollama if multiple URLs are configured, otherwise single Ollama
        if let Some(ref urls) = config.ollama.urls {
            if !urls.is_empty() {
                Box::new(MultiOllamaEmbeddingProvider::new(
                    urls.clone(),
                    Some(embedding_model),
                    Some(embedding_dims),
                ))
            } else {
                let ollama_url = config.ollama.url.as_str();
                Box::new(LocalEmbeddingProvider::new(
                    Some(ollama_url),
                    Some(embedding_model),
                    Some(embedding_dims),
                ))
            }
        } else {
            let ollama_url = config.ollama.url.as_str();
            Box::new(LocalEmbeddingProvider::new(
                Some(ollama_url),
                Some(embedding_model),
                Some(embedding_dims),
            ))
        }
    };
    let embedding = match embedding_provider.compute_embedding(&content).await {
        Ok(emb) => {
            println!("Embedding computed: {} dimensions", emb.len());
            Some(emb)
        }
        Err(e) => {
            eprintln!("Warning: Failed to compute embedding: {}", e);
            None
        }
    };

    // Save to index if requested
    if cli.save {
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

        let file_metadata = semantic_source.to_metadata().await?;
        
        indexer.index_semantic_file(
            &file_meta,
            &tags,
            Some(&content),
            file_metadata.as_ref(),
            embedding.as_deref(),
        )
        .await
        .context("Failed to index file")?;
        
        println!("✓ File indexed to Meilisearch");
    } else {
        println!("ℹ️  Use --save to index this file to Meilisearch");
    }

    Ok(())
}

