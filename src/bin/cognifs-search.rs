use anyhow::{Context, Result};
use clap::Parser;
use cognifs::{
    config::Config,
    indexer::{Indexer, MeilisearchIndexer},
};

#[derive(Parser)]
#[command(name = "cognifs-search")]
#[command(about = "Search for files using Meilisearch")]
#[command(version)]
struct Cli {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    
    println!("Searching for: {}", cli.query);
    
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

    let results = indexer.search(&cli.query).await?;

    println!("\nFound {} results:", results.len());
    for (i, file) in results.iter().enumerate() {
        println!("{}. {}", i + 1, file.path.display());
    }

    Ok(())
}

