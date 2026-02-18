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

The figures are generated from `results/results.csv` and stored at the following paths.

![time histogram](docs/fig/time_hist.png)

![solve calls histogram](docs/fig/solve_calls_hist.png)

![time vs cnf clauses](docs/fig/time_vs_cnf.png)

The required figure paths are `docs/fig/time_hist.png`, `docs/fig/solve_calls_hist.png`, and `docs/fig/time_vs_cnf.png`.

## References

Schwartz, 1980, Journal of the ACM.

Zippel, 1979, EUROSAM.

Fiat and Shamir, 1986, CRYPTO.

Merkle, 1987, CRYPTO.

Chakraborty, Meel, and Vardi, 2013, CAV.

Chakraborty, Meel, and Vardi, 2016, IJCAI.
