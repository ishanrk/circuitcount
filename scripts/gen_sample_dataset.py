#!/usr/bin/env python3
import argparse
import random
from pathlib import Path


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--out_dir", default="datasets/sample")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--num_bench", type=int, default=120)
    p.add_argument("--num_aag", type=int, default=120)
    return p.parse_args()


def choose(rng, seq):
    return seq[rng.randrange(len(seq))]


def gen_bench_instance(rng: random.Random, idx: int) -> str:
    n_inputs = rng.randint(6, 16)
    n_nodes = rng.randint(16, 48)
    inputs = [f"x{i}" for i in range(n_inputs)]
    lines = [f"INPUT({name})" for name in inputs]
    names = list(inputs)

    ops = ["AND", "OR", "XOR", "XNOR", "NOT", "BUF"]
    for i in range(n_nodes):
        lhs = f"n{idx}_{i}"
        op = choose(rng, ops)
        if op in ("NOT", "BUF"):
            a = choose(rng, names)
            line = f"{lhs} = {op}({a})"
        else:
            a = choose(rng, names)
            b = choose(rng, names)
            line = f"{lhs} = {op}({a},{b})"
        lines.append(line)
        names.append(lhs)

    out_name = choose(rng, names)
    lines.append(f"OUTPUT({out_name})")
    return "\n".join(lines) + "\n"


def lit_from_id(node_id: int, neg: bool) -> int:
    return node_id * 2 + (1 if neg else 0)


def gen_aag_instance(rng: random.Random) -> str:
    i_count = rng.randint(6, 16)
    a_count = rng.randint(18, 56)
    m = i_count + a_count
    lines = [f"aag {m} {i_count} 0 1 {a_count}"]

    for inp_id in range(1, i_count + 1):
        lines.append(str(lit_from_id(inp_id, False)))

    for gate_idx in range(a_count):
        gate_id = i_count + 1 + gate_idx
        prev_ids = list(range(0, gate_id))
        a_id = choose(rng, prev_ids)
        b_id = choose(rng, prev_ids)
        a_lit = lit_from_id(a_id, rng.choice([False, True]))
        b_lit = lit_from_id(b_id, rng.choice([False, True]))
        lhs = lit_from_id(gate_id, False)
        lines.append(f"{lhs} {a_lit} {b_lit}")

    out_id = rng.randint(0, m)
    out_lit = lit_from_id(out_id, rng.choice([False, True]))

    # output line goes before and lines in aag
    and_lines = lines[1 + i_count :]
    lines = lines[: 1 + i_count] + [str(out_lit)] + and_lines
    return "\n".join(lines) + "\n"


def main():
    args = parse_args()
    rng = random.Random(args.seed)

    root = Path(args.out_dir)
    bench_dir = root / "bench"
    aag_dir = root / "aag"
    run15_dir = root / "run15"
    bench_dir.mkdir(parents=True, exist_ok=True)
    aag_dir.mkdir(parents=True, exist_ok=True)
    run15_dir.mkdir(parents=True, exist_ok=True)

    for i in range(args.num_bench):
        text = gen_bench_instance(rng, i)
        (bench_dir / f"inst_{i:04d}.bench").write_text(text, encoding="utf-8")

    for i in range(args.num_aag):
        text = gen_aag_instance(rng)
        (aag_dir / f"inst_{i:04d}.aag").write_text(text, encoding="utf-8")

    # fixed subset for benchmark runtime control
    for i in range(8):
        src = bench_dir / f"inst_{i:04d}.bench"
        dst = run15_dir / f"bench_{i:04d}.bench"
        dst.write_text(src.read_text(encoding="utf-8"), encoding="utf-8")
    for i in range(7):
        src = aag_dir / f"inst_{i:04d}.aag"
        dst = run15_dir / f"aag_{i:04d}.aag"
        dst.write_text(src.read_text(encoding="utf-8"), encoding="utf-8")

    print(f"generated bench={args.num_bench} aag={args.num_aag} seed={args.seed}")
    print(f"subset run15={len(list(run15_dir.iterdir()))}")


if __name__ == "__main__":
    main()
