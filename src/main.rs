use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MODULUS: u64 = 998_244_353;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fp(u64);

impl Fp {
    fn new(value: u64) -> Self {
        Self(value % MODULUS)
    }

    fn zero() -> Self {
        Self(0)
    }

    fn one() -> Self {
        Self(1)
    }

    fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::ops::Add for Fp {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        let mut sum = self.0 + rhs.0;
        if sum >= MODULUS {
            sum -= MODULUS;
        }
        Self(sum)
    }
}

impl std::ops::Sub for Fp {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(self.0 + MODULUS - rhs.0)
        }
    }
}

impl std::ops::Mul for Fp {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let prod = (self.0 as u128 * rhs.0 as u128) % MODULUS as u128;
        Self(prod as u64)
    }
}

impl Display for Fp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
struct Cnf {
    num_vars: usize,
    clauses: Vec<Vec<i32>>,
}

impl Cnf {
    fn parse_dimacs(input: &str) -> Result<Self> {
        let mut num_vars = 0usize;
        let mut clauses = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('c') {
                continue;
            }

            if line.starts_with('p') {
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() < 4 || parts[1] != "cnf" {
                    bail!("invalid DIMACS header: {line}");
                }
                num_vars = parts[2]
                    .parse::<usize>()
                    .context("invalid variable count in DIMACS header")?;
                continue;
            }

            let mut clause = Vec::new();
            for tok in line.split_whitespace() {
                let lit = tok.parse::<i32>().context("invalid literal")?;
                if lit == 0 {
                    break;
                }
                clause.push(lit);
            }
            if !clause.is_empty() {
                clauses.push(clause);
            }
        }

        if num_vars == 0 {
            bail!("DIMACS file does not contain a valid problem line");
        }

        Ok(Self { num_vars, clauses })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Proof {
    prime: u64,
    seed: u64,
    num_vars: usize,
    num_clauses: usize,
    grid_bits: usize,
    clause_weights: Vec<u64>,
    boolean_weights: Vec<u64>,
    grid_root: String,
    round_roots: Vec<String>,
    rounds: Vec<RoundProof>,
    witness_index: usize,
    witness_point: Vec<u8>,
    witness_opening: MerkleOpening,
    claimed_value_at_witness: u64,
    claimed_p_at_witness: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RoundProof {
    round: usize,
    challenge: u64,
    query_index: usize,
    left_opening: MerkleOpening,
    right_opening: MerkleOpening,
    next_opening: MerkleOpening,
}

#[derive(Debug, Serialize, Deserialize)]
struct MerkleOpening {
    index: usize,
    value: u64,
    siblings: Vec<String>,
}

#[derive(Parser)]
#[command(name = "ffsat")]
#[command(about = "Finite-field proof-carrying SAT demo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
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
        #[arg(long, default_value_t = 8)]
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
    Demo,
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
            run_prove(&cnf, &witness, &proof, seed, grid_bits, allow_invalid)?;
        }
        Commands::Verify {
            cnf,
            witness,
            proof,
        } => {
            let accepted = run_verify(&cnf, &witness, &proof)?;
            if accepted {
                println!("VERIFICATION: ACCEPT");
            } else {
                println!("VERIFICATION: REJECT");
            }
        }
        Commands::Demo => run_demo()?,
    }
    Ok(())
}

fn run_prove(
    cnf_path: &str,
    witness_path: &str,
    proof_path: &str,
    seed: u64,
    requested_grid_bits: usize,
    allow_invalid: bool,
) -> Result<()> {
    let cnf = load_cnf(cnf_path)?;
    let witness = load_witness(witness_path, cnf.num_vars)?;
    let mut rng = XorShift64::new(seed);

    let clause_weights = (0..cnf.clauses.len())
        .map(|_| sample_nonzero_field(&mut rng).as_u64())
        .collect::<Vec<_>>();
    let boolean_weights = (0..cnf.num_vars)
        .map(|_| sample_nonzero_field(&mut rng).as_u64())
        .collect::<Vec<_>>();

    let p_at_witness = aggregate_poly_eval(
        &cnf,
        &witness,
        &to_field_vec(&clause_weights),
        &to_field_vec(&boolean_weights),
    );

    if p_at_witness != Fp::zero() && !allow_invalid {
        bail!(
            "witness does not satisfy constraints (P(w) = {}), use --allow-invalid for negative demos",
            p_at_witness
        );
    }

    let grid_bits = requested_grid_bits.min(cnf.num_vars).max(1);
    let grid_size = 1usize << grid_bits;
    let mut grid_values = Vec::with_capacity(grid_size);
    for idx in 0..grid_size {
        let point = mix_witness_with_grid_index(&witness, grid_bits, idx);
        let value = aggregate_poly_eval(
            &cnf,
            &point,
            &to_field_vec(&clause_weights),
            &to_field_vec(&boolean_weights),
        );
        grid_values.push(value);
    }

    let mut vectors = vec![grid_values];
    for r in 0..grid_bits {
        let root_hex = merkle_root_hex(&vectors[r]);
        let challenge = fs_challenge(seed, r, &root_hex);
        let next = fold_vector(&vectors[r], challenge);
        vectors.push(next);
    }

    let round_roots = vectors
        .iter()
        .map(|v| merkle_root_hex(v))
        .collect::<Vec<String>>();
    let grid_root = round_roots[0].clone();

    let mut rounds = Vec::with_capacity(grid_bits);
    for round in 0..grid_bits {
        let root_i = &round_roots[round];
        let root_next = &round_roots[round + 1];
        let challenge = fs_challenge(seed, round, root_i);
        let next_len = vectors[round + 1].len();
        let query_index = fs_query(seed, round, root_i, root_next, next_len);
        let left_idx = 2 * query_index;
        let right_idx = left_idx + 1;

        let left_opening = merkle_open(&vectors[round], left_idx)?;
        let right_opening = merkle_open(&vectors[round], right_idx)?;
        let next_opening = merkle_open(&vectors[round + 1], query_index)?;

        let relation = Fp::new(left_opening.value) * (Fp::one() - challenge)
            + Fp::new(right_opening.value) * challenge;
        if relation.as_u64() != next_opening.value {
            bail!("internal prover error: fold relation failed at round {round}");
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
        seed,
        num_vars: cnf.num_vars,
        num_clauses: cnf.clauses.len(),
        grid_bits,
        clause_weights,
        boolean_weights,
        grid_root,
        round_roots,
        rounds,
        witness_index,
        witness_point: witness[..grid_bits].to_vec(),
        witness_opening,
        claimed_value_at_witness: p_at_witness.as_u64(),
        claimed_p_at_witness: p_at_witness.as_u64(),
    };

    if let Some(parent) = Path::new(proof_path).parent() {
        fs::create_dir_all(parent).with_context(|| format!("cannot create {:?}", parent))?;
    }
    fs::write(proof_path, serde_json::to_string_pretty(&proof)?)
        .with_context(|| format!("cannot write proof file {}", proof_path))?;

    println!(
        "Proof generated at {} with grid bits {} and root {}",
        proof_path, grid_bits, proof.grid_root
    );
    Ok(())
}

fn run_verify(cnf_path: &str, witness_path: &str, proof_path: &str) -> Result<bool> {
    let cnf = load_cnf(cnf_path)?;
    let witness = load_witness(witness_path, cnf.num_vars)?;
    let proof: Proof = serde_json::from_str(
        &fs::read_to_string(proof_path).with_context(|| format!("cannot read {}", proof_path))?,
    )
    .with_context(|| format!("cannot parse proof JSON in {}", proof_path))?;

    if proof.prime != MODULUS {
        return Ok(false);
    }
    if proof.num_vars != cnf.num_vars || proof.num_clauses != cnf.clauses.len() {
        return Ok(false);
    }
    if proof.clause_weights.len() != cnf.clauses.len() {
        return Ok(false);
    }
    if proof.boolean_weights.len() != cnf.num_vars {
        return Ok(false);
    }
    if proof.round_roots.len() != proof.grid_bits + 1 {
        return Ok(false);
    }
    if proof.rounds.len() != proof.grid_bits {
        return Ok(false);
    }
    if proof.round_roots.first() != Some(&proof.grid_root) {
        return Ok(false);
    }

    let clause_weights = to_field_vec(&proof.clause_weights);
    let boolean_weights = to_field_vec(&proof.boolean_weights);
    let p_at_witness = aggregate_poly_eval(&cnf, &witness, &clause_weights, &boolean_weights);
    if p_at_witness.as_u64() != proof.claimed_p_at_witness {
        return Ok(false);
    }
    if proof.claimed_value_at_witness != proof.witness_opening.value {
        return Ok(false);
    }

    let expected_index = bits_to_index(&witness[..proof.grid_bits]);
    if proof.witness_index != expected_index || proof.witness_opening.index != expected_index {
        return Ok(false);
    }
    if proof.witness_point != witness[..proof.grid_bits] {
        return Ok(false);
    }

    if !verify_opening(&proof.grid_root, &proof.witness_opening)? {
        return Ok(false);
    }
    if proof.witness_opening.value != p_at_witness.as_u64() {
        return Ok(false);
    }

    for round in 0..proof.grid_bits {
        let round_proof = &proof.rounds[round];
        if round_proof.round != round {
            return Ok(false);
        }

        let root_i = &proof.round_roots[round];
        let root_next = &proof.round_roots[round + 1];
        let expected_challenge = fs_challenge(proof.seed, round, root_i);
        let expected_query = fs_query(
            proof.seed,
            round,
            root_i,
            root_next,
            1usize << (proof.grid_bits - round - 1),
        );

        if round_proof.challenge != expected_challenge.as_u64() {
            return Ok(false);
        }
        if round_proof.query_index != expected_query {
            return Ok(false);
        }

        let left_idx = 2 * round_proof.query_index;
        let right_idx = left_idx + 1;
        if round_proof.left_opening.index != left_idx {
            return Ok(false);
        }
        if round_proof.right_opening.index != right_idx {
            return Ok(false);
        }
        if round_proof.next_opening.index != round_proof.query_index {
            return Ok(false);
        }

        if !verify_opening(root_i, &round_proof.left_opening)?
            || !verify_opening(root_i, &round_proof.right_opening)?
            || !verify_opening(root_next, &round_proof.next_opening)?
        {
            return Ok(false);
        }

        let lhs = Fp::new(round_proof.next_opening.value);
        let rhs = Fp::new(round_proof.left_opening.value) * (Fp::one() - expected_challenge)
            + Fp::new(round_proof.right_opening.value) * expected_challenge;
        if lhs != rhs {
            return Ok(false);
        }
    }

    Ok(p_at_witness == Fp::zero())
}

fn run_demo() -> Result<()> {
    let sat_cnf = "examples/sat.cnf";
    let sat_witness = "examples/sat.wtns";
    let unsat_cnf = "examples/unsat.cnf";
    let unsat_witness = "examples/unsat.wtns";
    let sat_proof = "artifacts/sat.proof.json";
    let unsat_proof = "artifacts/unsat.proof.json";

    println!("Running SAT demo with a valid witness.");
    run_prove(sat_cnf, sat_witness, sat_proof, 7, 3, false)?;
    let sat_ok = run_verify(sat_cnf, sat_witness, sat_proof)?;
    println!(
        "SAT instance verification result: {}",
        if sat_ok { "ACCEPT" } else { "REJECT" }
    );

    println!("Running UNSAT demo using an invalid witness to show rejection.");
    run_prove(unsat_cnf, unsat_witness, unsat_proof, 11, 1, true)?;
    let unsat_ok = run_verify(unsat_cnf, unsat_witness, unsat_proof)?;
    println!(
        "UNSAT instance verification result: {}",
        if unsat_ok { "ACCEPT" } else { "REJECT" }
    );

    Ok(())
}

fn load_cnf(path: &str) -> Result<Cnf> {
    let content =
        fs::read_to_string(path).with_context(|| format!("cannot read CNF file {path}"))?;
    Cnf::parse_dimacs(&content)
}

fn load_witness(path: &str, num_vars: usize) -> Result<Vec<u8>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("cannot read witness file {path}"))?;
    let mut bits = Vec::new();
    for token in content.split_whitespace() {
        match token {
            "0" => bits.push(0u8),
            "1" => bits.push(1u8),
            _ => bail!("witness contains non-binary value {token}"),
        }
    }
    if bits.len() != num_vars {
        bail!("witness has {} values, expected {}", bits.len(), num_vars);
    }
    Ok(bits)
}

fn literal_eval(lit: i32, point: &[u8]) -> Result<Fp> {
    let idx = lit.unsigned_abs() as usize - 1;
    if idx >= point.len() {
        return Err(anyhow!("literal index {} out of range", idx + 1));
    }
    let x = Fp::new(point[idx] as u64);
    if lit > 0 { Ok(x) } else { Ok(Fp::one() - x) }
}

fn clause_unsat_indicator(clause: &[i32], point: &[u8]) -> Result<Fp> {
    let mut acc = Fp::one();
    for &lit in clause {
        let lit_val = literal_eval(lit, point)?;
        acc = acc * (Fp::one() - lit_val);
    }
    Ok(acc)
}

fn aggregate_poly_eval(
    cnf: &Cnf,
    point: &[u8],
    clause_weights: &[Fp],
    boolean_weights: &[Fp],
) -> Fp {
    let mut acc = Fp::zero();
    for (j, clause) in cnf.clauses.iter().enumerate() {
        let u = clause_unsat_indicator(clause, point).unwrap_or(Fp::one());
        acc = acc + clause_weights[j] * u;
    }
    for i in 0..cnf.num_vars {
        let x = Fp::new(point[i] as u64);
        let b = x * (x - Fp::one());
        acc = acc + boolean_weights[i] * b;
    }
    acc
}

fn mix_witness_with_grid_index(witness: &[u8], grid_bits: usize, index: usize) -> Vec<u8> {
    let mut point = witness.to_vec();
    for i in 0..grid_bits {
        point[i] = ((index >> i) & 1) as u8;
    }
    point
}

fn bits_to_index(bits: &[u8]) -> usize {
    let mut index = 0usize;
    for (i, &bit) in bits.iter().enumerate() {
        index |= (bit as usize) << i;
    }
    index
}

fn fold_vector(values: &[Fp], r: Fp) -> Vec<Fp> {
    let mut next = Vec::with_capacity(values.len() / 2);
    for pair in values.chunks_exact(2) {
        let left = pair[0];
        let right = pair[1];
        next.push(left * (Fp::one() - r) + right * r);
    }
    next
}

fn hash_leaf(value: Fp) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"leaf");
    hasher.update(value.as_u64().to_le_bytes());
    hasher.finalize().into()
}

fn hash_node(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"node");
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn merkle_levels(values: &[Fp]) -> Vec<Vec<[u8; 32]>> {
    let mut levels = Vec::new();
    let mut current = values.iter().copied().map(hash_leaf).collect::<Vec<_>>();
    levels.push(current.clone());
    while current.len() > 1 {
        let mut next = Vec::with_capacity(current.len().div_ceil(2));
        let mut i = 0usize;
        while i < current.len() {
            let left = current[i];
            let right = if i + 1 < current.len() {
                current[i + 1]
            } else {
                current[i]
            };
            next.push(hash_node(left, right));
            i += 2;
        }
        current = next;
        levels.push(current.clone());
    }
    levels
}

fn merkle_root_hex(values: &[Fp]) -> String {
    let levels = merkle_levels(values);
    to_hex(
        levels
            .last()
            .and_then(|v| v.first())
            .copied()
            .unwrap_or([0u8; 32]),
    )
}

fn merkle_open(values: &[Fp], mut index: usize) -> Result<MerkleOpening> {
    if index >= values.len() {
        bail!(
            "opening index {index} out of bounds for {} leaves",
            values.len()
        );
    }
    let original_index = index;
    let original_value = values[index].as_u64();
    let levels = merkle_levels(values);
    let mut siblings = Vec::new();
    for level in &levels[..levels.len() - 1] {
        let sibling = if index % 2 == 0 {
            if index + 1 < level.len() {
                level[index + 1]
            } else {
                level[index]
            }
        } else {
            level[index - 1]
        };
        siblings.push(to_hex(sibling));
        index /= 2;
    }
    Ok(MerkleOpening {
        index: original_index,
        value: original_value,
        siblings,
    })
}

fn verify_opening(root_hex: &str, opening: &MerkleOpening) -> Result<bool> {
    let mut hash = hash_leaf(Fp::new(opening.value));
    let mut idx = opening.index;
    for sibling_hex in &opening.siblings {
        let sibling = from_hex_32(sibling_hex)?;
        if idx % 2 == 0 {
            hash = hash_node(hash, sibling);
        } else {
            hash = hash_node(sibling, hash);
        }
        idx /= 2;
    }
    Ok(to_hex(hash) == root_hex)
}

fn fs_challenge(seed: u64, round: usize, root_hex: &str) -> Fp {
    let mut hasher = Sha256::new();
    hasher.update(b"challenge");
    hasher.update(seed.to_le_bytes());
    hasher.update((round as u64).to_le_bytes());
    hasher.update(root_hex.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    Fp::new(u64::from_le_bytes(bytes))
}

fn fs_query(seed: u64, round: usize, root_hex: &str, next_root_hex: &str, len: usize) -> usize {
    let mut hasher = Sha256::new();
    hasher.update(b"query");
    hasher.update(seed.to_le_bytes());
    hasher.update((round as u64).to_le_bytes());
    hasher.update(root_hex.as_bytes());
    hasher.update(next_root_hex.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    (u64::from_le_bytes(bytes) as usize) % len.max(1)
}

fn to_field_vec(raw: &[u64]) -> Vec<Fp> {
    raw.iter().copied().map(Fp::new).collect()
}

fn sample_nonzero_field(rng: &mut XorShift64) -> Fp {
    loop {
        let x = Fp::new(rng.next_u64());
        if x != Fp::zero() {
            return x;
        }
    }
}

fn to_hex(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(hex_digit((b >> 4) & 0x0f));
        out.push(hex_digit(b & 0x0f));
    }
    out
}

fn from_hex_32(s: &str) -> Result<[u8; 32]> {
    if s.len() != 64 {
        bail!("invalid hex length {}", s.len());
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_value(s.as_bytes()[2 * i])?;
        let lo = hex_value(s.as_bytes()[2 * i + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => '0',
    }
}

fn hex_value(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(10 + c - b'a'),
        b'A'..=b'F' => Ok(10 + c - b'A'),
        _ => bail!("invalid hex char {}", c as char),
    }
}

#[derive(Debug, Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}
