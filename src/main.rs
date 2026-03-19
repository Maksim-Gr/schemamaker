mod cli;
mod schema;
mod inference;
mod generator;

use std::fs;
use clap::Parser;
use generator::Generator;

fn main() {
      let args = cli::Args::parse();

      let table_name = args.name.unwrap_or_else(|| {
          args.input
              .file_stem()
              .expect("input file has no stem")
              .to_string_lossy()
              .to_string()
      });

      let content = fs::read_to_string(&args.input).unwrap_or_else(|e| {
          eprintln!("Error reading {:?}: {}", args.input, e);
          std::process::exit(1);
      });

      let schema = inference::infer_schema(&content, &table_name).unwrap_or_else(|e| {
          eprintln!("Error inferring schema: {}", e);
          std::process::exit(1);
      });

      let generator = Generator::new(&schema, args.cluster, args.kafka);

      let up_path = args.output_dir.join(format!("{}_up.sql", table_name));
      let down_path = args.output_dir.join(format!("{}_down.sql", table_name));

      fs::write(&up_path, generator.generate_up()).unwrap_or_else(|e| {
          eprintln!("Error writing {:?}: {}", up_path, e);
          std::process::exit(1);
      });

      fs::write(&down_path, generator.generate_down()).unwrap_or_else(|e| {
          eprintln!("Error writing {:?}: {}", down_path, e);
          std::process::exit(1);
      });

      eprintln!("Written: {}", up_path.display());
      eprintln!("Written: {}", down_path.display());
  }
