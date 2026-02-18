# circuitcount

## Overview

This repository parses `.aag` and `.bench` circuits, builds an AIG, applies cone restriction and simplification, encodes to CNF with Tseitin clauses, and runs projected counting with exact and hash-cell modes. The benchmark artifacts in this repository were generated from `datasets/sample/` and stored in `results/` and `docs/fig/`.

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

## Figures

`wall_ms` is end-to-end wall clock time in milliseconds for one benchmark row. `solve_calls` is the number of SAT solves used by the counting loop for that row. `time_per_call_ms` is `wall_ms / max(solve_calls, 1)`. `clause_density` is `cnf_clauses / max(cnf_vars, 1)`. `diversity_score` is `cone_frac * ands_per_cone_in`, where `cone_frac = cone_inputs / max(aig_inputs, 1)` and `ands_per_cone_in = aig_ands / max(cone_inputs, 1)`. p90 is the 90th percentile: the value such that 90% of the data is at or below it.

The figures are generated from `results/results.csv`. The script also writes `docs/fig/report.md` with numeric summaries used in this section.

```bash
python scripts/plot_results.py --csv results/results.csv --out_dir docs/fig
```

This figure shows wall time by CNF size bucket. It highlights how latency changes with formula size and reports median and p90 directly in the title.

![time histogram](docs/fig/time_hist.png)

This figure shows solve call distribution by the same size buckets. 

![solve calls histogram](docs/fig/solve_calls_hist.png)

This figure shows model count time per solve against vars per clause. 

![time vs cnf clauses](docs/fig/time_vs_cnf.png)


The current report values are dataset_rows=1440, ok_rows=1398, timeout_rows=0, median_wall_ms_ok=1.000, and p90_wall_ms_ok=1.300. The largest size bucket is (8.0, 17.0], with largest_bucket_median_wall_ms_ok=1.000 and largest_bucket_timeout_rate=0.000.

Interpretation from this run is that lag appears in the largest CNF bucket by wall time. Timeout concentration is not present in this sample. The vars-per-clause to time-per-call correlation is positive at 0.041.

## References

Schwartz, Jacob T. Fast probabilistic algorithms for verification of polynomial identities. Journal of the ACM. 1980.

Zippel, Richard. Probabilistic algorithms for sparse polynomials. EUROSAM. 1979.

Fiat, Amos and Shamir, Adi. How to prove yourself: practical solutions to identification and signature problems. CRYPTO. 1986.

Merkle, Ralph C. A digital signature based on a conventional encryption function. CRYPTO. 1987.

Chakraborty, Supratik, Meel, Kuldeep S., and Vardi, Moshe Y. A scalable approximate model counter. CP. 2013.

Chakraborty, Supratik, Meel, Kuldeep S., and Vardi, Moshe Y. Algorithmic improvements in approximate counting for probabilistic inference: from linear to logarithmic SAT calls. IJCAI. 2016.

Soos, Mate and Meel, Kuldeep S. BIRD: engineering an efficient cnf-xor sat solver and its applications to approximate model counting. AAAI. 2019.

Biere, Armin. The AIGER and-inverter graph format version 20071012. FMV Report Series, JKU Linz. 2007.
