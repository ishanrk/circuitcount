use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cnf::{Cnf, bits_to_index};
use crate::field::{Fp, MODULUS, to_field_vec};
use crate::merkle::{MerkleOpening, merkle_open, merkle_root_hex, verify_opening};
use crate::rng::XorShift64;

#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub seed: u64,
    pub requested_grid_bits: usize,
    pub allow_invalid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub prime: u64,
    pub seed: u64,
    pub num_vars: usize,
    pub num_clauses: usize,
    pub grid_bits: usize,
    pub clause_weights: Vec<u64>,
    pub boolean_weights: Vec<u64>,
    pub grid_root: String,
    pub round_roots: Vec<String>,
    pub rounds: Vec<RoundProof>,
    pub witness_index: usize,
    pub witness_point: Vec<u8>,
    pub witness_opening: MerkleOpening,
    pub claimed_p_at_witness: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundProof {
    pub round: usize,
    pub challenge: u64,
    pub query_index: usize,
    pub left_opening: MerkleOpening,
    pub right_opening: MerkleOpening,
    pub next_opening: MerkleOpening,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStats {
    pub build_ms: u128,
    pub grid_bits: usize,
    pub grid_size: usize,
    pub fold_rounds: usize,
    pub field_ops_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProveOutput {
    pub proof: Proof,
    pub stats: ProofStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub accepted: bool,
    pub reason: String,
}

pub fn prove(cnf: &Cnf, witness: &[u8], cfg: &ProverConfig) -> Result<ProveOutput> {
    if witness.len() != cnf.num_vars {
        bail!(
            "witness length {} does not match num_vars {}",
            witness.len(),
            cnf.num_vars
        );
    }

    let started = Instant::now();
    let mut rng = XorShift64::new(cfg.seed);
    let clause_weights = (0..cnf.clauses.len())
        .map(|_| rng.sample_nonzero_fp().as_u64())
        .collect::<Vec<_>>();
    let boolean_weights = (0..cnf.num_vars)
        .map(|_| rng.sample_nonzero_fp().as_u64())
        .collect::<Vec<_>>();
    let clause_weights_fp = to_field_vec(&clause_weights);
    let boolean_weights_fp = to_field_vec(&boolean_weights);

    let p_at_witness = cnf.aggregate_poly_eval(witness, &clause_weights_fp, &boolean_weights_fp)?;
    if !cfg.allow_invalid && p_at_witness != Fp::zero() {
        bail!(
            "witness is invalid for this CNF, P(w) = {} (use --allow-invalid for negative demos)",
            p_at_witness
        );
    }

    let grid_bits = cfg.requested_grid_bits.min(cnf.num_vars).max(1);
    let grid_size = 1usize << grid_bits;

    let mut grid_values = vec![Fp::zero(); grid_size];
    grid_values
        .par_iter_mut()
        .enumerate()
        .try_for_each(|(idx, slot)| -> Result<()> {
            let mut point = witness.to_vec();
            for (bit, coord) in point.iter_mut().take(grid_bits).enumerate() {
                *coord = ((idx >> bit) & 1) as u8;
            }
            *slot = cnf.aggregate_poly_eval(&point, &clause_weights_fp, &boolean_weights_fp)?;
            Ok(())
        })?;

    let mut vectors = vec![grid_values];
    for round in 0..grid_bits {
        let root = merkle_root_hex(&vectors[round]);
        let challenge = fs_challenge(cfg.seed, round, &root);
        vectors.push(fold_vector(&vectors[round], challenge));
    }

    let round_roots = vectors
        .iter()
        .map(|v| merkle_root_hex(v))
        .collect::<Vec<_>>();
    let mut rounds = Vec::<RoundProof>::with_capacity(grid_bits);
    for round in 0..grid_bits {
        let root_i = &round_roots[round];
        let root_next = &round_roots[round + 1];
        let challenge = fs_challenge(cfg.seed, round, root_i);
        let query_index = fs_query(cfg.seed, round, root_i, root_next, vectors[round + 1].len());

        let left_idx = 2 * query_index;
        let right_idx = left_idx + 1;
        let left_opening = merkle_open(&vectors[round], left_idx)?;
        let right_opening = merkle_open(&vectors[round], right_idx)?;
        let next_opening = merkle_open(&vectors[round + 1], query_index)?;

        let folded = Fp::new(left_opening.value) * (Fp::one() - challenge)
            + Fp::new(right_opening.value) * challenge;
        if folded.as_u64() != next_opening.value {
            bail!(
                "internal prover consistency check failed at round {}",
                round
            );
        }
        rounds.push(RoundProof {
            round,
            challenge: challenge.as_u64(),
            query_index,
            left_opening,
            right_opening,
            next_opening,
        });
    }

    let witness_index = bits_to_index(&witness[..grid_bits]);
    let witness_opening = merkle_open(&vectors[0], witness_index)?;

    let proof = Proof {
        prime: MODULUS,
        seed: cfg.seed,
        num_vars: cnf.num_vars,
        num_clauses: cnf.clauses.len(),
        grid_bits,
        clause_weights,
        boolean_weights,
        grid_root: round_roots[0].clone(),
        round_roots,
        rounds,
        witness_index,
        witness_point: witness[..grid_bits].to_vec(),
        witness_opening,
        claimed_p_at_witness: p_at_witness.as_u64(),
    };

    let degree = cnf.aggregate_degree_upper_bound();
    let field_ops_estimate =
        grid_size * (cnf.clauses.len() * degree + cnf.num_vars * 4) + grid_size + (grid_bits * 16);
    let stats = ProofStats {
        build_ms: started.elapsed().as_millis(),
        grid_bits,
        grid_size,
        fold_rounds: grid_bits,
        field_ops_estimate,
    };

    Ok(ProveOutput { proof, stats })
}

pub fn verify(cnf: &Cnf, witness: &[u8], proof: &Proof) -> Result<VerifyReport> {
    macro_rules! reject {
        ($msg:expr) => {
            return Ok(VerifyReport {
                accepted: false,
                reason: $msg.to_string(),
            })
        };
    }

    if proof.prime != MODULUS {
        reject!("prime mismatch");
    }
    if witness.len() != cnf.num_vars {
        reject!("witness length mismatch");
    }
    if proof.num_vars != cnf.num_vars || proof.num_clauses != cnf.clauses.len() {
        reject!("instance metadata mismatch");
    }
    if proof.rounds.len() != proof.grid_bits || proof.round_roots.len() != proof.grid_bits + 1 {
        reject!("round shape mismatch");
    }
    if proof.round_roots[0] != proof.grid_root {
        reject!("root chain mismatch");
    }

    let clause_weights_fp = to_field_vec(&proof.clause_weights);
    let boolean_weights_fp = to_field_vec(&proof.boolean_weights);
    let p_at_witness = cnf.aggregate_poly_eval(witness, &clause_weights_fp, &boolean_weights_fp)?;
    if p_at_witness.as_u64() != proof.claimed_p_at_witness {
        reject!("claimed P(w) mismatch");
    }

    let expected_witness_idx = bits_to_index(&witness[..proof.grid_bits]);
    if proof.witness_index != expected_witness_idx
        || proof.witness_opening.index != expected_witness_idx
        || proof.witness_point != witness[..proof.grid_bits]
    {
        reject!("witness location mismatch");
    }
    if !verify_opening(&proof.grid_root, &proof.witness_opening)? {
        reject!("witness opening not on committed root");
    }
    if proof.witness_opening.value != p_at_witness.as_u64() {
        reject!("witness opening value mismatch");
    }

    for round in 0..proof.grid_bits {
        let rp = &proof.rounds[round];
        let root_i = &proof.round_roots[round];
        let root_next = &proof.round_roots[round + 1];

        if rp.round != round {
            reject!("round index mismatch");
        }

        let expected_challenge = fs_challenge(proof.seed, round, root_i);
        if rp.challenge != expected_challenge.as_u64() {
            reject!("challenge mismatch");
        }
        let expected_query = fs_query(
            proof.seed,
            round,
            root_i,
            root_next,
            1usize << (proof.grid_bits - round - 1),
        );
        if rp.query_index != expected_query {
            reject!("query index mismatch");
        }

        let left_idx = 2 * expected_query;
        let right_idx = left_idx + 1;
        if rp.left_opening.index != left_idx
            || rp.right_opening.index != right_idx
            || rp.next_opening.index != expected_query
        {
            reject!("opening index mismatch");
        }
        if !verify_opening(root_i, &rp.left_opening)?
            || !verify_opening(root_i, &rp.right_opening)?
            || !verify_opening(root_next, &rp.next_opening)?
        {
            reject!("merkle opening failed");
        }

        let rhs = Fp::new(rp.left_opening.value) * (Fp::one() - expected_challenge)
            + Fp::new(rp.right_opening.value) * expected_challenge;
        if rhs.as_u64() != rp.next_opening.value {
            reject!("fold equation mismatch");
        }
    }

    if p_at_witness != Fp::zero() {
        reject!("algebraic identity is non-zero on witness");
    }

    Ok(VerifyReport {
        accepted: true,
        reason: "all transcript checks passed and P(w)=0".to_string(),
    })
}

pub fn write_proof(path: &str, output: &ProveOutput) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).with_context(|| format!("cannot create {:?}", parent))?;
    }
    let json = serde_json::to_string_pretty(output)?;
    fs::write(path, json).with_context(|| format!("cannot write {}", path))?;
    Ok(())
}

pub fn read_proof(path: &str) -> Result<ProveOutput> {
    let text = fs::read_to_string(path).with_context(|| format!("cannot read {}", path))?;
    let parsed: ProveOutput = serde_json::from_str(&text).with_context(|| {
        format!(
            "cannot parse proof JSON in {} (expected ProveOutput with proof+stats)",
            path
        )
    })?;
    Ok(parsed)
}

pub fn fold_vector(values: &[Fp], r: Fp) -> Vec<Fp> {
    values
        .chunks_exact(2)
        .map(|pair| pair[0] * (Fp::one() - r) + pair[1] * r)
        .collect::<Vec<_>>()
}

pub fn fs_challenge(seed: u64, round: usize, root_hex: &str) -> Fp {
    let mut h = Sha256::new();
    h.update(b"challenge");
    h.update(seed.to_le_bytes());
    h.update((round as u64).to_le_bytes());
    h.update(root_hex.as_bytes());
    let digest = h.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    Fp::new(u64::from_le_bytes(out))
}

pub fn fs_query(seed: u64, round: usize, root_hex: &str, next_root_hex: &str, len: usize) -> usize {
    let mut h = Sha256::new();
    h.update(b"query");
    h.update(seed.to_le_bytes());
    h.update((round as u64).to_le_bytes());
    h.update(root_hex.as_bytes());
    h.update(next_root_hex.as_bytes());
    let digest = h.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    (u64::from_le_bytes(out) as usize) % len.max(1)
}

pub fn schwartz_zippel_failure_upper_bound(degree: usize) -> f64 {
    degree as f64 / MODULUS as f64
}
