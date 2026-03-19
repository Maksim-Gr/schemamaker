use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "schemamaker",
    about = "Generate Clickhouse migrations from a JSON file",
    version = "0.1.0"
)]
pub struct Args {
    /// JSONEachRow input file
    pub input: PathBuf,
    /// Override table name (defaults to input file name)
    #[arg(short, long)]
    pub name: Option<String>,
    /// ClickHouse cluster name
    #[arg(short, long, default_value = "clickhouse_datalake")]
    pub cluster: String,
    /// Kafka collection name
    #[arg(short, long, default_value = "kafka")]
    pub kafka: String,
    /// Output directory for the migration file
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,
}
