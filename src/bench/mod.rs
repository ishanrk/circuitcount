use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::circuit::aig::Aig;
use crate::circuit::aiger::parse_aag_reader;
use crate::circuit::bench::parse_bench_reader;
use crate::count::hash_count::{CountBackend, CountMode, CountOptions, count_output_with_options};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Aag,
    Bench,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountConfig {
    pub backend: CountBackend,
    pub seed: u64,
    pub pivot: usize,
    pub trials: usize,
    pub p: f64,
    pub r: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchRow {
    pub path: String,
    pub status: String,
    pub backend: String,
    pub mode: String,
    pub wall_ms: Option<u128>,
    pub solve_calls: Option<usize>,
    pub result: Option<u128>,
    pub m: Option<usize>,
    pub trials: usize,
    pub r: usize,
    pub seed: u64,
    pub file_bytes: Option<u64>,
    pub aig_inputs: Option<usize>,
    pub aig_ands: Option<usize>,
    pub cone_inputs: Option<usize>,
    pub cnf_vars: Option<u32>,
    pub cnf_clauses: Option<usize>,
}

impl BenchRow {
    pub fn csv_header() -> &'static str {
        "path,status,backend,mode,wall_ms,solve_calls,result,m,trials,r,seed,file_bytes,aig_inputs,aig_ands,cone_inputs,cnf_vars,cnf_clauses"
    }

    pub fn to_csv_line(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            esc_csv(&self.path),
            self.status,
            self.backend,
            self.mode,
            opt_u128(self.wall_ms),
            opt_usize(self.solve_calls),
            opt_u128(self.result),
            opt_usize(self.m),
            self.trials,
            self.r,
            self.seed,
            opt_u64(self.file_bytes),
            opt_usize(self.aig_inputs),
            opt_usize(self.aig_ands),
            opt_usize(self.cone_inputs),
            opt_u32(self.cnf_vars),
            opt_usize(self.cnf_clauses),
        )
    }
}

pub fn run_one(path: &Path, out: usize, cfg: CountConfig, timeout: Duration) -> BenchRow {
    let file_bytes = fs::metadata(path).ok().map(|m| m.len());
    let start = Instant::now();
    let p = path.to_path_buf();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let row = run_one_inner(&p, out, cfg, file_bytes);
        let _ = tx.send(row);
    });

    match rx.recv_timeout(timeout) {
        Ok(mut row) => {
            row.wall_ms = Some(start.elapsed().as_millis());
            row
        }
        Err(mpsc::RecvTimeoutError::Timeout) => BenchRow {
            path: path.to_string_lossy().to_string(),
            status: "timeout".to_string(),
            backend: backend_name(cfg.backend).to_string(),
            mode: String::new(),
            wall_ms: Some(start.elapsed().as_millis()),
            solve_calls: None,
            result: None,
            m: None,
            trials: cfg.trials,
            r: cfg.r,
            seed: cfg.seed,
            file_bytes,
            aig_inputs: None,
            aig_ands: None,
            cone_inputs: None,
            cnf_vars: None,
            cnf_clauses: None,
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => BenchRow {
            path: path.to_string_lossy().to_string(),
            status: "internal_error".to_string(),
            backend: backend_name(cfg.backend).to_string(),
            mode: String::new(),
            wall_ms: Some(start.elapsed().as_millis()),
            solve_calls: None,
            result: None,
            m: None,
            trials: cfg.trials,
            r: cfg.r,
            seed: cfg.seed,
            file_bytes,
            aig_inputs: None,
            aig_ands: None,
            cone_inputs: None,
            cnf_vars: None,
            cnf_clauses: None,
        },
    }
}

pub fn run_dataset(
    dir: &Path,
    out_idx: usize,
    format: InputFormat,
    cfg: CountConfig,
    timeout: Duration,
    csv_path: &Path,
    progress: bool,
) -> std::io::Result<Vec<BenchRow>> {
    let paths = discover_paths(dir, format)?;
    let mut csv = File::create(csv_path)?;
    writeln!(csv, "{}", BenchRow::csv_header())?;
    csv.flush()?;

    let mut rows = Vec::new();
    for path in paths {
        let row = run_one(&path, out_idx, cfg, timeout);
        if progress {
            println!(
                "path={} status={} wall_ms={} mode={} result={}",
                row.path,
                row.status,
                row.wall_ms.unwrap_or_default(),
                row.mode,
                row.result.unwrap_or_default()
            );
        }
        writeln!(csv, "{}", row.to_csv_line())?;
        csv.flush()?;
        rows.push(row);
    }

    Ok(rows)
}

fn run_one_inner(path: &Path, out: usize, cfg: CountConfig, file_bytes: Option<u64>) -> BenchRow {
    let base = BenchRow {
        path: path.to_string_lossy().to_string(),
        status: "ok".to_string(),
        backend: backend_name(cfg.backend).to_string(),
        mode: String::new(),
        wall_ms: None,
        solve_calls: None,
        result: None,
        m: None,
        trials: cfg.trials,
        r: cfg.r,
        seed: cfg.seed,
        file_bytes,
        aig_inputs: None,
        aig_ands: None,
        cone_inputs: None,
        cnf_vars: None,
        cnf_clauses: None,
    };

    let parsed = match parse_any(path) {
        Ok(v) => v,
        Err(_) => {
            let mut row = base.clone();
            row.status = "parse_error".to_string();
            return row;
        }
    };
    let restricted = match parsed.restrict_to_output(out) {
        Ok(v) => v,
        Err(_) => {
            let mut row = base.clone();
            row.status = "internal_error".to_string();
            return row;
        }
    };

    let mut row = base.clone();
    row.aig_inputs = Some(restricted.num_inputs());
    row.aig_ands = Some(restricted.num_ands());
    row.cone_inputs = Some(restricted.num_inputs());

    let report = match count_output_with_options(
        &restricted,
        0,
        CountOptions {
            seed: cfg.seed,
            pivot: cfg.pivot,
            trials: cfg.trials,
            sparsity: cfg.p,
            backend: cfg.backend,
            progress: false,
            repeats: cfg.r,
        },
    ) {
        Ok(v) => v,
        Err(_) => {
            row.status = "internal_error".to_string();
            return row;
        }
    };

    row.mode = match report.mode {
        CountMode::Exact => "exact".to_string(),
        CountMode::Hash => "hash".to_string(),
    };
    row.solve_calls = Some(report.solve_calls);
    row.result = Some(report.result);
    row.m = Some(report.m_used);
    row.cnf_vars = Some(report.vars);
    row.cnf_clauses = Some(report.clauses);
    if report.mode == CountMode::Exact && report.result == 0 {
        row.status = "unsat".to_string();
    }
    row
}

fn discover_paths(dir: &Path, format: InputFormat) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::<PathBuf>::new();
    collect_paths(dir, format, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_paths(dir: &Path, format: InputFormat, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_paths(&p, format, out)?;
            continue;
        }
        if matches_format(&p, format) {
            out.push(p);
        }
    }
    Ok(())
}

fn matches_format(path: &Path, format: InputFormat) -> bool {
    let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
    match format {
        InputFormat::Aag => ext == "aag",
        InputFormat::Bench => ext == "bench",
        InputFormat::Auto => ext == "aag" || ext == "bench",
    }
}

fn parse_any(path: &Path) -> anyhow::Result<Aig> {
    let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    match ext {
        "aag" => parse_aag_reader(reader),
        "bench" => parse_bench_reader(reader),
        _ => anyhow::bail!("unsupported extension"),
    }
}

fn backend_name(b: CountBackend) -> &'static str {
    match b {
        CountBackend::Dpll => "dpll",
        CountBackend::Varisat => "varisat",
    }
}

fn opt_usize(v: Option<usize>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}
fn opt_u32(v: Option<u32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}
fn opt_u64(v: Option<u64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}
fn opt_u128(v: Option<u128>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn esc_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
