use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail};
use clap::Parser;

use circuitcount::bench::{CountConfig, InputFormat, run_dataset};
use circuitcount::count::hash_count::CountBackend;

#[derive(Debug, Parser)]
#[command(name = "bench_dataset")]
struct Cli {
    #[arg(long)]
    dir: String,
    #[arg(long, default_value_t = 0)]
    out: usize,
    #[arg(long, default_value = "varisat")]
    backend: String,
    #[arg(long, default_value_t = 3)]
    r: usize,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long = "timeout_ms", default_value_t = 30000)]
    timeout_ms: u64,
    #[arg(long)]
    csv: String,
    #[arg(long, default_value = "auto")]
    format: String,
    #[arg(long, default_value_t = false)]
    progress: bool,
    #[arg(long, default_value_t = 4096)]
    pivot: usize,
    #[arg(long, default_value_t = 1)]
    trials: usize,
    #[arg(long, default_value_t = 0.35)]
    p: f64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run_from_args(&cli)
}

fn run_from_args(cli: &Cli) -> Result<()> {
    let backend = parse_backend(&cli.backend)?;
    let format = parse_format(&cli.format)?;
    let cfg = CountConfig {
        backend,
        seed: cli.seed,
        pivot: cli.pivot,
        trials: cli.trials,
        p: cli.p,
        r: cli.r,
    };
    let rows = run_dataset(
        &PathBuf::from(&cli.dir),
        cli.out,
        format,
        cfg,
        Duration::from_millis(cli.timeout_ms),
        &PathBuf::from(&cli.csv),
        cli.progress,
    )?;
    println!("rows={}", rows.len());
    Ok(())
}

fn parse_backend(s: &str) -> Result<CountBackend> {
    match s {
        "dpll" => Ok(CountBackend::Dpll),
        "varisat" => Ok(CountBackend::Varisat),
        _ => bail!("unknown backend '{}', expected dpll|varisat", s),
    }
}

fn parse_format(s: &str) -> Result<InputFormat> {
    match s {
        "aag" => Ok(InputFormat::Aag),
        "bench" => Ok(InputFormat::Bench),
        "auto" => Ok(InputFormat::Auto),
        _ => bail!("unknown format '{}', expected aag|bench|auto", s),
    }
}
