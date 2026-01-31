use std::path::PathBuf;

use clap::{Parser, Subcommand};

use steganos::{decode_png_to_file, encode_file_to_png};

#[derive(Parser)]
#[command(name = "steganos", version, about = "Encode files into PNGs and decode them back")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Encode {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Decode {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Encode { input, output } => {
            let start = std::time::Instant::now();
            encode_file_to_png(&input, output.as_deref()).map(|p| {
                let elapsed = start.elapsed();
                let input_size = file_size_bytes(&input);
                let output_size = file_size_bytes(&p);
                println!("Encoded to {}", p.display());
                let secs = elapsed.as_secs();
                let millis = elapsed.subsec_millis();
                println!("Encode time: {}.{:03} s", secs, millis);
                println!(
                    "Encode sizes: input {} MB, output {} MB",
                    format_size(input_size),
                    format_size(output_size)
                );
            })
        }
        Commands::Decode { input, output } => {
            let start = std::time::Instant::now();
            decode_png_to_file(&input, output.as_deref()).map(|p| {
                let elapsed = start.elapsed();
                let input_size = file_size_bytes(&input);
                let output_size = file_size_bytes(&p);
                println!("Decoded to {}", p.display());
                let secs = elapsed.as_secs();
                let millis = elapsed.subsec_millis();
                println!("Decode time: {}.{:03} s", secs, millis);
                println!(
                    "Decode sizes: input {} MB, output {} MB",
                    format_size(input_size),
                    format_size(output_size)
                );
            })
        }
    };

    if let Err(err) = result {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn file_size_bytes(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn format_size(bytes: u64) -> String {
    let megabytes = (bytes as f64) / 1_000_000.0;
    format!("{:.3}", megabytes)
}
