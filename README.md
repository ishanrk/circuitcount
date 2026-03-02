# circuitcount **Repository Migrated from a depreciated account to this account on Feb 2026**

<img width="470" height="242" alt="image" src="https://github.com/user-attachments/assets/93073cd2-6791-49dd-86f2-43ec8ab027da" />
- An example of a logical circuit

## Model Counting

Model counting can be simply described as a method to count all satsifying solutions to a Boolean formula. This project tries to leverage the fact that certain Boolean formulas come pre-structured as logical circuits to apply some speedups to this model counting process, althought this is still a work in progress in terms of optimizing the process to compete with SOTA model counters. A very simple run of this program should be something like taking in a circuit as a .bench file or an and inverter graph to return the projected model count

Applications are prevalent in cryptography and circuit synthesis.

## Overview

This repository parses `.aag` and `.bench` circuits, builds an AIG, applies cone restriction and simplification, encodes to CNF with Tseitin clauses, and runs projected counting with exact and hash-cell modes. The benchmark artifacts in this repository were generated from `datasets/sample/` and stored in `results/` and `docs/fig/`. Most of the techniques used in this implementation were inspired using the references cited below and especially the model counters GANAK and ApproxMC.


## Build

```bash
cargo build --release
```

```bash
cargo test
```

## Formats

The parser accepts AIGER ASCII with `.aag` extension and BENCH subset with `.bench` extension. The benchmark runner accepts `--format aag`, `--format bench`, and `--format auto`.

## Commands

Commands to run the model counter and get the number of satisfying solutions with different flag examples.

```bash
cargo run --bin circuitcount -- count "datasets/sample/run15/bench_0003.bench" --out 0 --seed 1 --pivot 4096 --trials 1 --p 0.35 --backend varisat --r 5
```

```bash
cargo run --bin bench_dataset -- --dir "datasets/sample/run15" --out 0 --backend varisat --r 5 --seed 1 --timeout_ms 60000 --csv "results/varisat.csv" --format auto --progress
```

```bash
cargo run --bin bench_dataset -- --dir "datasets/sample/run15" --out 0 --backend dpll --r 5 --seed 1 --timeout_ms 60000 --csv "results/dpll.csv" --format auto --progress
```

```bash
python scripts/gen_sample_dataset.py --out_dir datasets/sample --seed 1 --num_bench 120 --num_aag 120
```

```bash
python scripts/plot_results.py --csv results/results.csv --out_dir docs/fig
```

## Counting output fields

```text
inputs_coi=3 ands=2 vars=6 clauses=8 pivot=4096 trials=1
backend=varisat solve_calls=6 mode=exact result=5 m=0 trials=1 r=5
```

The first line reports cone size and CNF size with pivot and trials. The second line reports backend, solve calls, mode, result, selected `m`, trials, and repeat count.

## Benchmarking a dataset

The benchmark command writes one CSV row per instance with this exact header.

```text
path,status,backend,mode,wall_ms,solve_calls,result,m,trials,r,seed,file_bytes,aig_inputs,aig_ands,cone_inputs,cnf_vars,cnf_clauses
```

This repository includes generated runs in `results/varisat.csv`, `results/dpll.csv`, and merged `results/results.csv`. The merged file is built by keeping one header and appending rows from both backend files.

```text
path=datasets/sample/run15\aag_0000.aag status=ok wall_ms=1 mode=exact result=1
path=datasets/sample/run15\bench_0007.bench status=ok wall_ms=2 mode=exact result=3
rows=15
```

The merged benchmark has 30 instances. The median wall time is 0 ms. The p90 wall time is 1 ms. The median solve_calls value is 4.


## References

Schwartz, Jacob T. Fast probabilistic algorithms for verification of polynomial identities. Journal of the ACM. 1980.

Zippel, Richard. Probabilistic algorithms for sparse polynomials. EUROSAM. 1979.

Fiat, Amos and Shamir, Adi. How to prove yourself: practical solutions to identification and signature problems. CRYPTO. 1986.

Merkle, Ralph C. A digital signature based on a conventional encryption function. CRYPTO. 1987.

Chakraborty, Supratik, Meel, Kuldeep S., and Vardi, Moshe Y. A scalable approximate model counter. CP. 2013.

Chakraborty, Supratik, Meel, Kuldeep S., and Vardi, Moshe Y. Algorithmic improvements in approximate counting for probabilistic inference: from linear to logarithmic SAT calls. IJCAI. 2016.

Soos, Mate and Meel, Kuldeep S. BIRD: engineering an efficient cnf-xor sat solver and its applications to approximate model counting. AAAI. 2019.

Biere, Armin. The AIGER and-inverter graph format version 20071012. FMV Report Series, JKU Linz. 2007.
