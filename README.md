# FFTSAT

FFTSAT is a finite-field proof-carrying SAT implementation in Rust. Given a CNF formula and witness, the prover maps constraints into polynomial identities over `Fp` with `p = 998244353`, commits to a subspace evaluation table with a Merkle tree, and emits a Fiat-Shamir transcript of fold rounds. The verifier checks Merkle openings, fold equations, and the terminal condition `P(w) = 0`.

## Algebraic Formulation

For variable vector `x in {0,1}^n` and clause `C_j = (l_{j,1} OR ... OR l_{j,k_j})`, define the unsatisfied-indicator polynomial

`U_j(x) = product_t (1 - E(l_{j,t}, x))`,

where `E(x_i, x) = x_i` and `E(not x_i, x) = 1 - x_i`. On Boolean points, `U_j(x) = 1` iff clause `j` is unsatisfied, otherwise `0`. Enforce Booleanity with `B_i(x) = x_i(x_i - 1)`. Sample random coefficients `r_j, s_i in Fp` and define

`P(x) = sum_j r_j U_j(x) + sum_i s_i B_i(x)`.

If the witness satisfies all clauses and is Boolean, `P(w) = 0`. For a non-zero polynomial `Q` of degree `d`, Schwartz-Zippel bounds random zero-test failure by `d / |Fp|`, so the prototype reports `d / p` as its explicit soundness upper bound.

## Protocol Sketch

The prover fixes `k` coordinates as a Boolean hypercube and evaluates `P` on `2^k` points. Let this vector be `v_0`. It commits to `v_0` via a Merkle root. For rounds `r = 0..k-1`, it derives challenge `alpha_r` with Fiat-Shamir, folds pairs as

`v_{r+1}[i] = (1 - alpha_r) v_r[2i] + alpha_r v_r[2i+1]`,

commits to `v_{r+1}`, and records Merkle openings at one FS-derived query index. The verifier re-derives challenges/queries, checks all openings and fold equations, checks witness inclusion against the first root, recomputes `P(w)`, and accepts iff all checks hold and `P(w) = 0`.

## Codebase Structure

`src/field.rs` implements finite-field arithmetic (`Fp`, inversion, roots of unity). `src/ntt.rs` provides naive and NTT polynomial multiplication for consistency and performance testing. `src/cnf.rs` implements robust DIMACS parsing, witness IO, clause evaluation, aggregate polynomial evaluation, and planted 3-SAT generation. `src/merkle.rs` provides Merkle commitments and authentication path verification. `src/protocol.rs` implements proving, verification, transcript serialization, fold checks, and soundness-bound utilities. `src/main.rs` is the CLI entrypoint with operational commands.

## CLI Surface

`prove` builds a proof JSON with transcript and stats. `verify` validates one proof. `batch-verify` validates a manifest of proof instances. `profile` computes structural metrics (`max_clause_width`, degree bound, Schwartz-Zippel bound). `gen-random` generates large planted SAT or forced-UNSAT instances. `poly-mul-demo` compares naive convolution and NTT. `demo` runs small and large end-to-end accept/reject paths.

## Reproducible Runs

Run full demo:

```bash
cargo run -- demo
```

Generate and verify a large SAT instance:

```bash
cargo run -- gen-random --vars 64 --clauses 320 --seed 1337 --cnf-out examples/large_sat.cnf --witness-out examples/large_sat.wtns
cargo run -- prove --cnf examples/large_sat.cnf --witness examples/large_sat.wtns --proof artifacts/large_sat.proof.json --seed 19 --grid-bits 10
cargo run -- verify --cnf examples/large_sat.cnf --witness examples/large_sat.wtns --proof artifacts/large_sat.proof.json
```

Generate and verify a large UNSAT stress case (forced contradiction):

```bash
cargo run -- gen-random --vars 96 --clauses 640 --seed 2026 --cnf-out examples/large_unsat.cnf --witness-out examples/large_unsat.wtns --make-unsat
cargo run -- prove --cnf examples/large_unsat.cnf --witness examples/large_unsat.wtns --proof artifacts/large_unsat.proof.json --seed 23 --grid-bits 9 --allow-invalid
cargo run -- verify --cnf examples/large_unsat.cnf --witness examples/large_unsat.wtns --proof artifacts/large_unsat.proof.json
```

Run complexity and arithmetic diagnostics:

```bash
cargo run -- profile --cnf examples/large_sat.cnf
cargo run -- poly-mul-demo --degree 256 --seed 314159
cargo run -- batch-verify --manifest examples/batch_manifest.json
```

## References

[1] J. T. Schwartz, *Fast Probabilistic Algorithms for Verification of Polynomial Identities*, J. ACM, 1980.  
[2] R. Zippel, *Probabilistic Algorithms for Sparse Polynomials*, EUROSAM, 1979.  
[3] C. Lund, L. Fortnow, H. Karloff, N. Nisan, *Algebraic Methods for Interactive Proof Systems*, J. ACM, 1992.  
[4] R. C. Merkle, *A Digital Signature Based on a Conventional Encryption Function*, CRYPTO, 1987.  
[5] A. Fiat, A. Shamir, *How to Prove Yourself: Practical Solutions to Identification and Signature Problems*, CRYPTO, 1986.  
[6] J. W. Cooley, J. W. Tukey, *An Algorithm for the Machine Calculation of Complex Fourier Series*, Math. Comp., 1965.
