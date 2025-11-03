use anyhow::Result;
use clap::{Parser, Subcommand};
use cognifs::config::Config;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cognifs")]
#[command(about = "The Cognitive File System — it understands and organizes your files automatically")]
#[command(version)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch a directory for file changes
    #[command(name = "watch")]
    Watch {
        /// Directory to watch
        #[arg(value_name = "DIR")]
        dir: PathBuf,
        /// Automatically index files when they change
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
    },
    /// Tag a single file and optionally save to index
    #[command(name = "tag")]
    Tag {
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
    },
    /// Index files in a directory to Meilisearch
    #[command(name = "index")]
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
    #[command(name = "search")]
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
    #[command(name = "organize")]
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
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration (falls back to defaults if not found)
    let config = Config::load().unwrap_or_default();

    match cli.command {
        Commands::Watch { dir, auto_index, meili_url, meili_key, index_name } => {
            // Delegate to cognifs-watch binary
            eprintln!("Use 'cognifs-watch' binary directly, or run: cargo run --bin cognifs-watch -- {}", dir.display());
            if auto_index {
                eprintln!("  --auto-index");
            }
            if let Some(url) = meili_url {
                eprintln!("  --meili-url {}", url);
            }
            if let Some(key) = meili_key {
                eprintln!("  --meili-key {}", key);
            }
            if let Some(name) = index_name {
                eprintln!("  --index-name {}", name);
            }
        }
        Commands::Tag { file, save, meili_url, meili_key, index_name } => {
            // Delegate to cognifs-tag binary
            eprintln!("Use 'cognifs-tag' binary directly, or run: cargo run --bin cognifs-tag -- {} {}", 
                if save { "--save" } else { "" }, file.display());
        }
        Commands::Index { dir, meili_url, meili_key, index_name } => {
            // Delegate to cognifs-index binary
            eprintln!("Use 'cognifs-index' binary directly, or run: cargo run --bin cognifs-index -- {}", dir.display());
        }
        Commands::Search { query, meili_url, meili_key, index_name } => {
            // Delegate to cognifs-search binary
            eprintln!("Use 'cognifs-search' binary directly, or run: cargo run --bin cognifs-search -- {}", query);
        }
        Commands::Organize { dir, dry_run, yes, use_llm, index, meili_url, meili_key, index_name } => {
            // Delegate to cognifs-organize binary
            eprintln!("Use 'cognifs-organize' binary directly, or run: cargo run --bin cognifs-organize -- {}", dir.display());
        }
    }

    Ok(())
}

