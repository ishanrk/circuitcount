use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use circuitcount::bench::{CountConfig, InputFormat, BenchRow, run_dataset, run_one};
use circuitcount::count::hash_count::CountBackend;

#[test]
fn benchmark_csv_has_expected_shape() {
    let root = temp_dataset_dir("bench_csv_shape");
    fs::create_dir_all(&root).expect("mkdir");

    let bench_path = root.join("tiny1.bench");
    fs::write(
        &bench_path,
        "INPUT(a)\nINPUT(b)\nOUTPUT(out)\nout = XOR(a,b)\n",
    )
    .expect("write bench");

    let aag_path = root.join("tiny2.aag");
    fs::write(
        &aag_path,
        "aag 5 3 0 1 2\n2\n4\n6\n11\n8 2 4\n10 9 7\n",
    )
    .expect("write aag");

    let csv_path = root.join("results.csv");
    let cfg = CountConfig {
        backend: CountBackend::Dpll,
        seed: 0,
        pivot: 1000,
        trials: 1,
        p: 0.35,
        r: 3,
    };
    let rows = run_dataset(
        &root,
        0,
        InputFormat::Auto,
        cfg,
        Duration::from_millis(10_000),
        &csv_path,
        false,
    )
    .expect("run dataset");
    assert_eq!(rows.len(), 2);

    let csv_text = fs::read_to_string(&csv_path).expect("read csv");
    let mut lines = csv_text.lines();
    let header = lines.next().unwrap_or("");
    assert_eq!(header, BenchRow::csv_header());
    let data = lines.collect::<Vec<_>>();
    assert_eq!(data.len(), 2);

    for line in &data {
        let cols = line.split(',').collect::<Vec<_>>();
        assert_eq!(cols.len(), 17);
        assert_eq!(cols[1], "ok");
        assert_eq!(cols[2], "dpll");
        let wall_ms = cols[4].parse::<u128>().expect("wall_ms");
        assert!(wall_ms <= u128::MAX);
    }
}

#[test]
fn run_one_seed_is_deterministic_in_hash_mode() {
    let root = temp_dataset_dir("bench_csv_seed");
    fs::create_dir_all(&root).expect("mkdir");
    let bench_path = root.join("hash_target.bench");
    fs::write(
        &bench_path,
        "INPUT(a)\nINPUT(b)\nINPUT(c)\nOUTPUT(out)\nn1 = AND(a,b)\nout = OR(n1,c)\n",
    )
    .expect("write bench");

    let cfg = CountConfig {
        backend: CountBackend::Dpll,
        seed: 7,
        pivot: 2,
        trials: 3,
        p: 0.35,
        r: 3,
    };

    let row1 = run_one(&bench_path, 0, cfg, Duration::from_millis(10_000));
    let row2 = run_one(&bench_path, 0, cfg, Duration::from_millis(10_000));
    assert_eq!(row1.status, "ok");
    assert_eq!(row2.status, "ok");
    assert_eq!(row1.mode, "hash");
    assert_eq!(row2.mode, "hash");
    assert_eq!(row1.m, row2.m);
    assert_eq!(row1.result, row2.result);
    assert_eq!(row1.trials, row2.trials);
}

fn temp_dataset_dir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    p.push(format!("circuitcount_{}_{}", tag, t));
    p
}
