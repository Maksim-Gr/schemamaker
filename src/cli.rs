use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "schemamaker",
    about = "Generate ClickHouse migrations from a JSON file",
    version = "0.1.3-beta"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate full Kafka→ClickHouse pipeline migrations (streams, raw, datalake)
    Kafka(KafkaArgs),
    /// Scan JSON fields and suggest suitable ClickHouse table engines
    Scan(ScanArgs),
    /// Generate a simple CREATE TABLE migration from JSON
    Table(TableArgs),
}

#[derive(Parser, Debug)]
pub struct KafkaArgs {
    /// JSONEachRow input file
    pub input: PathBuf,
    /// Override table name (defaults to input file stem)
    #[arg(short, long)]
    pub name: Option<String>,
    /// ClickHouse cluster name
    #[arg(short, long, default_value = "clickhouse_datalake")]
    pub cluster: String,
    /// Kafka collection name
    #[arg(short, long, default_value = "kafka")]
    pub kafka: String,
    /// Output directory for migration files
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,
}

#[derive(Parser, Debug)]
pub struct ScanArgs {
    /// JSONEachRow input file
    pub input: PathBuf,
    /// Override table name (defaults to input file stem)
    #[arg(short, long)]
    pub name: Option<String>,
    /// If set, suggest Replicated engine variants
    #[arg(short, long)]
    pub cluster: Option<String>,
}

#[derive(Parser, Debug)]
pub struct TableArgs {
    /// JSONEachRow input file
    pub input: PathBuf,
    /// Override table name (defaults to input file stem)
    #[arg(short, long)]
    pub name: Option<String>,
    /// Table engine: MergeTree, ReplicatedMergeTree, ReplacingMergeTree, SummingMergeTree
    /// If omitted, inferred automatically from JSON fields
    #[arg(short, long)]
    pub engine: Option<String>,
    /// Comma-separated ORDER BY fields (overrides inference)
    #[arg(long)]
    pub order_by: Option<String>,
    /// ClickHouse cluster name (required for ReplicatedMergeTree)
    #[arg(short, long)]
    pub cluster: Option<String>,
    /// Output directory for migration files
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,
}
