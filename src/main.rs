use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use ffsat::cnf::{generate_planted_3sat, load_cnf, load_witness, write_cnf, write_witness};
use ffsat::field::Fp;
use ffsat::ntt::{naive_mul, ntt_mul};
use ffsat::protocol::{
    ProverConfig, VerifyReport, prove, read_proof, schwartz_zippel_failure_upper_bound, verify,
    write_proof,
};
use ffsat::rng::XorShift64;

#[derive(Parser, Debug)]
#[command(name = "ffsat")]
#[command(about = "Finite-field SAT proof-carrying prototype")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Prove {
        #[arg(long)]
        cnf: String,
        #[arg(long)]
        witness: String,
        #[arg(long)]
        proof: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 10)]
        grid_bits: usize,
        #[arg(long)]
        allow_invalid: bool,
    },
    Verify {
        #[arg(long)]
        cnf: String,
        #[arg(long)]
        witness: String,
        #[arg(long)]
        proof: String,
    },
    BatchVerify {
        #[arg(long)]
        manifest: String,
    },
    Profile {
        #[arg(long)]
        cnf: String,
    },
    GenRandom {
        #[arg(long)]
        vars: usize,
        #[arg(long)]
        clauses: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        cnf_out: String,
        #[arg(long)]
        witness_out: String,
        #[arg(long)]
        make_unsat: bool,
    },
    PolyMulDemo {
        #[arg(long, default_value_t = 128)]
        degree: usize,
        #[arg(long, default_value_t = 99)]
        seed: u64,
    },
    Demo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchManifest {
    entries: Vec<BatchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchEntry {
    name: String,
    cnf: String,
    witness: String,
    proof: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Prove {
            cnf,
            witness,
            proof,
            seed,
            grid_bits,
            allow_invalid,
        } => {
            let cnf_inst = load_cnf(&cnf)?;
            let witness_vec = load_witness(&witness, cnf_inst.num_vars)?;
            let output = prove(
                &cnf_inst,
                &witness_vec,
                &ProverConfig {
                    seed,
                    requested_grid_bits: grid_bits,
                    allow_invalid,
                },
            )?;
            write_proof(&proof, &output)?;
            println!(
                "PROVE: wrote {} | root={} | grid_bits={} | build_ms={}",
                proof, output.proof.grid_root, output.stats.grid_bits, output.stats.build_ms
            );
        }
        Commands::Verify {
            cnf,
            witness,
            proof,
        } => {
            let cnf_inst = load_cnf(&cnf)?;
            let witness_vec = load_witness(&witness, cnf_inst.num_vars)?;
            let proof_obj = read_proof(&proof)?;
            let report = verify(&cnf_inst, &witness_vec, &proof_obj.proof)?;
            print_verify_report(&report);
        }
        Commands::BatchVerify { manifest } => {
            let text = std::fs::read_to_string(&manifest)?;
            let parsed: BatchManifest = serde_json::from_str(&text)?;
            let mut accepted = 0usize;
            for entry in &parsed.entries {
                let cnf_inst = load_cnf(&entry.cnf)?;
                let witness_vec = load_witness(&entry.witness, cnf_inst.num_vars)?;
                let proof_obj = read_proof(&entry.proof)?;
                let report = verify(&cnf_inst, &witness_vec, &proof_obj.proof)?;
                let flag = if report.accepted { "ACCEPT" } else { "REJECT" };
                println!("BATCH {}: {} ({})", entry.name, flag, report.reason);
                if report.accepted {
                    accepted += 1;
                }
            }
            println!(
                "BATCH SUMMARY: {}/{} accepted",
                accepted,
                parsed.entries.len()
            );
        }
        Commands::Profile { cnf } => {
            let cnf_inst = load_cnf(&cnf)?;
            let degree = cnf_inst.aggregate_degree_upper_bound();
            let width = cnf_inst.max_clause_width();
            let failure = schwartz_zippel_failure_upper_bound(degree);
            println!("PROFILE");
            println!("num_vars={}", cnf_inst.num_vars);
            println!("num_clauses={}", cnf_inst.clauses.len());
            println!("max_clause_width={}", width);
            println!("degree_upper_bound={}", degree);
            println!("schwartz_zippel_failure_bound<={:.6e}", failure);
            println!("field_prime={}", ffsat::field::MODULUS);
        }
        Commands::GenRandom {
            vars,
            clauses,
            seed,
            cnf_out,
            witness_out,
            make_unsat,
        } => {
            let (cnf_inst, witness_vec) = generate_planted_3sat(vars, clauses, seed, make_unsat);
            write_cnf(&cnf_out, &cnf_inst)?;
            write_witness(&witness_out, &witness_vec)?;
            println!(
                "GEN: wrote {} and {} | vars={} clauses={} unsat={}",
                cnf_out,
                witness_out,
                vars,
                cnf_inst.clauses.len(),
                make_unsat
            );
        }
        Commands::PolyMulDemo { degree, seed } => {
            run_poly_mul_demo(degree, seed)?;
        }
        Commands::Demo => {
            run_demo()?;
        }
    }
    Ok(())
}

fn print_verify_report(report: &VerifyReport) {
    if report.accepted {
        println!("VERIFICATION: ACCEPT ({})", report.reason);
    } else {
        println!("VERIFICATION: REJECT ({})", report.reason);
    }
}

fn run_poly_mul_demo(degree: usize, seed: u64) -> Result<()> {
    let mut rng = XorShift64::new(seed);
    let a = (0..degree)
        .map(|_| Fp::new(rng.next_u64()))
        .collect::<Vec<_>>();
    let b = (0..degree)
        .map(|_| Fp::new(rng.next_u64()))
        .collect::<Vec<_>>();
    let naive = naive_mul(&a, &b);
    let fast = ntt_mul(&a, &b)?;
    println!("POLY_MUL_DEMO degree={degree}");
    println!("naive_output_len={}", naive.len());
    println!("ntt_output_len={}", fast.len());
    println!(
        "consistency={}",
        if naive == fast { "MATCH" } else { "MISMATCH" }
    );
    Ok(())
}

fn run_demo() -> Result<()> {
    let sat_cnf = "examples/sat.cnf";
    let sat_wtns = "examples/sat.wtns";
    let unsat_cnf = "examples/unsat.cnf";
    let unsat_wtns = "examples/unsat.wtns";
    let large_sat_cnf = "examples/large_sat.cnf";
    let large_sat_wtns = "examples/large_sat.wtns";
    let large_unsat_cnf = "examples/large_unsat.cnf";
    let large_unsat_wtns = "examples/large_unsat.wtns";

    let sat_out = prove_and_verify(sat_cnf, sat_wtns, "artifacts/sat.proof.json", 7, 3, false)?;
    println!("DEMO SAT: {}", sat_out);

    let unsat_out = prove_and_verify(
        unsat_cnf,
        unsat_wtns,
        "artifacts/unsat.proof.json",
        11,
        1,
        true,
    )?;
    println!("DEMO UNSAT: {}", unsat_out);

    let large_sat_out = prove_and_verify(
        large_sat_cnf,
        large_sat_wtns,
        "artifacts/large_sat.proof.json",
        19,
        10,
        false,
    )?;
    println!("DEMO LARGE SAT: {}", large_sat_out);

    let large_unsat_out = prove_and_verify(
        large_unsat_cnf,
        large_unsat_wtns,
        "artifacts/large_unsat.proof.json",
        23,
        9,
        true,
    )?;
    println!("DEMO LARGE UNSAT: {}", large_unsat_out);

    Ok(())
}

fn prove_and_verify(
    cnf_path: &str,
    witness_path: &str,
    proof_path: &str,
    seed: u64,
    grid_bits: usize,
    allow_invalid: bool,
) -> Result<String> {
    let cnf_inst = load_cnf(cnf_path)?;
    let witness_vec = load_witness(witness_path, cnf_inst.num_vars)?;
    let output = prove(
        &cnf_inst,
        &witness_vec,
        &ProverConfig {
            seed,
            requested_grid_bits: grid_bits,
            allow_invalid,
        },
    )?;
    write_proof(proof_path, &output)?;
    let report = verify(&cnf_inst, &witness_vec, &output.proof)?;
    let outcome = if report.accepted { "ACCEPT" } else { "REJECT" };
    Ok(format!(
        "{} root={} grid={}ms={} reason={}",
        outcome,
        output.proof.grid_root,
        output.stats.grid_bits,
        output.stats.build_ms,
        report.reason
    ))
}
