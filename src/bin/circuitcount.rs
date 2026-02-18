use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use circuitcount::circuit::aiger::parse_aag_reader;
use circuitcount::circuit::bench::parse_bench_reader;
use circuitcount::cnf::dimacs::to_dimacs;
use circuitcount::cnf::tseitin::encode_aig;
use circuitcount::count::hash_count::{
    CountBackend, CountMode, CountOptions, count_output_with_options,
};

#[derive(Debug, Parser)]
#[command(name = "circuitcount")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    Parse { path: String },
    Coi {
        path: String,
        #[arg(long, default_value_t = 0)]
        out: usize,
    },
    Cnf {
        path: String,
        #[arg(long, default_value_t = 0)]
        out: usize,
        #[arg(long)]
        assert1: bool,
        #[arg(long)]
        emit: String,
    },
    Count {
        path: String,
        #[arg(long, default_value_t = 0)]
        out: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, default_value_t = 4096)]
        pivot: usize,
        #[arg(long, default_value_t = 1)]
        trials: usize,
        #[arg(long, default_value_t = 0.35)]
        p: f64,
        #[arg(long, default_value = "varisat")]
        backend: String,
        #[arg(long, default_value_t = false)]
        progress: bool,
        #[arg(long, default_value_t = 3)]
        r: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Parse { path } => parse_cmd(&path)?,
        Cmd::Coi { path, out } => coi_cmd(&path, out)?,
        Cmd::Cnf {
            path,
            out,
            assert1,
            emit,
        } => cnf_cmd(&path, out, assert1, &emit)?,
        Cmd::Count {
            path,
            out,
            seed,
            pivot,
            trials,
            p,
            backend,
            progress,
            r,
        } => count_cmd(&path, out, seed, pivot, trials, p, &backend, progress, r)?,
    }
    Ok(())
}

fn parse_cmd(path: &str) -> Result<()> {
    let aig = load_aig(path)?;
    println!(
        "inputs={} outputs={} ands={} max_id={}",
        aig.num_inputs(),
        aig.outputs().len(),
        aig.num_ands(),
        aig.max_id
    );
    Ok(())
}

fn coi_cmd(path: &str, out: usize) -> Result<()> {
    let aig = load_aig(path)?;
    let coi = aig.coi(out)?;
    let ext = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    println!(
        "inputs_total={} inputs_coi={} ands_total={} ands_coi={}",
        aig.num_inputs(),
        coi.input_ids().len(),
        aig.num_ands(),
        coi.ands_in_cone()
    );
    println!(
        "outputs_total={} out_idx={} format={}",
        aig.outputs().len(),
        out,
        ext
    );
    Ok(())
}

fn load_aig(path: &str) -> Result<circuitcount::circuit::aig::Aig> {
    let ext = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    match ext {
        "aag" => parse_aag_reader(reader),
        "bench" => parse_bench_reader(reader),
        _ => bail!("unsupported extension: expected .aag or .bench"),
    }
}

fn cnf_cmd(path: &str, out: usize, assert1: bool, emit: &str) -> Result<()> {
    let aig = load_aig(path)?;
    let simple = aig.simplify_output(out)?;
    let mut enc = encode_aig(&simple)?;

    if assert1 {
        if enc.output_lits.is_empty() {
            bail!("simplified circuit has no outputs");
        }
        let out_lit = enc.output_lits[0];
        enc.cnf.add_clause(vec![out_lit]);
    }

    let text = to_dimacs(&enc.cnf);
    std::fs::write(emit, text)?;
    println!(
        "vars={} clauses={} inputs={} ands={}",
        enc.cnf.num_vars,
        enc.cnf.clauses.len(),
        simple.num_inputs(),
        simple.num_ands()
    );
    Ok(())
}

fn count_cmd(
    path: &str,
    out: usize,
    seed: u64,
    pivot: usize,
    trials: usize,
    p: f64,
    backend: &str,
    progress: bool,
    r: usize,
) -> Result<()> {
    let aig = load_aig(path)?;
    let backend = match backend {
        "dpll" => CountBackend::Dpll,
        "varisat" => CountBackend::Varisat,
        _ => bail!("unknown backend '{}', expected dpll|varisat", backend),
    };
    let report = count_output_with_options(
        &aig,
        out,
        CountOptions {
            seed,
            pivot,
            trials,
            sparsity: p,
            backend,
            progress,
            repeats: r,
        },
    )?;
    let mode = match report.mode {
        CountMode::Exact => "exact",
        CountMode::Hash => "hash",
    };
    println!(
        "inputs_coi={} ands={} vars={} clauses={} pivot={} trials={}",
        report.inputs_coi, report.ands, report.vars, report.clauses, report.pivot, report.trials
    );
    println!(
        "backend={} solve_calls={} mode={} result={} m={} trials={} r={}",
        report.backend, report.solve_calls, mode, report.result, report.m_used, report.trials, r
    );
    Ok(())
}
