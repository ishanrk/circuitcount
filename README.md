# FFTSAT

FFTSAT is a finite-field proof-carrying SAT prototype written in Rust. The project encodes a CNF instance into polynomial constraints over a prime field, combines those constraints into a single aggregate identity with random linear combinations, commits to evaluation tables with a Merkle tree, and emits a sumcheck-style folding transcript that a verifier can replay. The verifier accepts when the witness is consistent with the committed transcript and the aggregate identity evaluates to zero at the witness point.

This repository is an executable reference implementation focused on end-to-end reproducibility. It includes DIMACS parsing, witness loading, finite-field arithmetic over `Fp` with a fixed prime modulus, clause-to-polynomial mapping, booleanity constraints, Fiat-Shamir challenges, Merkle authentication paths for transcript openings, and a command line interface that runs both positive and negative demonstrations.

## Algebraic Encoding

Given a clause `C = (l1 OR l2 OR ... OR lk)`, FFTSAT uses the unsatisfied-indicator polynomial

`U_C(x) = product_i (1 - eval(li, x))`

where `eval(li, x)` maps a positive literal `xi` to `xi` and a negated literal `NOT xi` to `1 - xi`. For Boolean assignments, `U_C(x)` is zero when the clause is satisfied and one when the clause is falsified. Booleanity is enforced with `B_i(x) = xi(xi - 1)`.

The aggregate polynomial is

`P(x) = sum_j r_j U_{C_j}(x) + sum_i s_i B_i(x)`

with random coefficients sampled from `Fp`. If the witness satisfies every clause and each coordinate is Boolean, then `P(w) = 0`. The random combination gives the standard Schwartz-Zippel style soundness intuition: a false identity is unlikely to vanish on random checks over a sufficiently large field.

## Proof Object

The prover commits to evaluations of `P` on a small subspace grid formed by varying the first `k` witness coordinates and fixing the rest. It then generates `k` folding rounds. Each round computes a random challenge, folds adjacent values linearly, commits to the next vector root, and provides Merkle openings for one query location chosen from Fiat-Shamir data. The final proof is serialized as JSON and contains field parameters, random coefficients, commitment roots, authenticated openings, the witness index on the grid, and claimed witness evaluations.

The verifier recomputes challenges and queries, checks Merkle paths, verifies fold consistency equations for every round, recomputes `P(w)`, and accepts only when all transcript checks pass and `P(w) = 0`.

## Complete Demo

Clone the repository and run the integrated demonstration command:

```bash
cargo run -- demo
```

The expected behavior is that the satisfiable instance is accepted and the unsatisfiable instance is rejected. A successful run prints output similar to:

```text
Running SAT demo with a valid witness.
Proof generated at artifacts/sat.proof.json with grid bits 3 and root ...
SAT instance verification result: ACCEPT
Running UNSAT demo using an invalid witness to show rejection.
Proof generated at artifacts/unsat.proof.json with grid bits 1 and root ...
UNSAT instance verification result: REJECT
```

After this command, `artifacts/sat.proof.json` and `artifacts/unsat.proof.json` are created so you can inspect the transcript and Merkle openings directly.

## Manual End-to-End Run

You can generate and verify a satisfiable proof directly with:

```bash
cargo run -- prove --cnf examples/sat.cnf --witness examples/sat.wtns --proof artifacts/sat.proof.json --seed 7 --grid-bits 3
cargo run -- verify --cnf examples/sat.cnf --witness examples/sat.wtns --proof artifacts/sat.proof.json
```

The verify command should print `VERIFICATION: ACCEPT`.

To demonstrate rejection on an invalid witness for an unsatisfiable instance, run:

```bash
cargo run -- prove --cnf examples/unsat.cnf --witness examples/unsat.wtns --proof artifacts/unsat.proof.json --seed 11 --grid-bits 1 --allow-invalid
cargo run -- verify --cnf examples/unsat.cnf --witness examples/unsat.wtns --proof artifacts/unsat.proof.json
```

The verify command should print `VERIFICATION: REJECT`.

## Repository Layout

The `src/main.rs` file contains the full CLI, finite-field arithmetic, DIMACS parser, polynomial encoding, Merkle commitment logic, transcript generation, and verifier checks. The `examples` directory contains one satisfiable and one unsatisfiable DIMACS benchmark plus matching witness files used by the demo command. The `artifacts` directory is generated at runtime and stores proof JSON files.

## Notes

This codebase is intentionally compact so the full proving and verification path is easy to audit and run locally. The implementation currently prioritizes correctness and transparency over high-performance kernels. It is straightforward to extend with Montgomery multiplication, SIMD batched operations, NTT-based polynomial arithmetic, and multicore interpolation once the protocol surface is fixed.
