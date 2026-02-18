use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use circuitcount::circuit::aiger::parse_aag_reader;

#[derive(Debug, Parser)]
#[command(name = "circuitcount")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    Parse { path: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Parse { path } => parse_cmd(&path)?,
    }
    Ok(())
}

fn parse_cmd(path: &str) -> Result<()> {
    let ext = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    if ext != "aag" {
        bail!("only .aag ascii files are supported");
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let aig = parse_aag_reader(reader)?;
    println!(
        "inputs={} outputs={} ands={} max_id={}",
        aig.num_inputs(),
        aig.outputs().len(),
        aig.num_ands(),
        aig.max_id
    );
    Ok(())
}
