#!/usr/bin/env python3
import argparse
import random
import re
from pathlib import Path


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--out_dir", default="datasets/sample")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--per_family", type=int, default=80)
    return p.parse_args()


def choose(rng, seq):
    return seq[rng.randrange(len(seq))]


def weighted_choice(rng: random.Random, choices):
    total = sum(w for _, w in choices)
    x = rng.random() * total
    cur = 0.0
    for item, w in choices:
        cur += w
        if x <= cur:
            return item
    return choices[-1][0]


def gen_bench_instance(rng: random.Random, idx: int, family: str) -> str:
    n_inputs = rng.randint(6, 16)
    n_nodes = rng.randint(16, 48)
    inputs = [f"x{i}" for i in range(n_inputs)]
    lines = [f"INPUT({name})" for name in inputs]
    names = list(inputs)

    weights = {
        "and_dominant": [("AND", 0.60), ("OR", 0.12), ("XOR", 0.08), ("XNOR", 0.08), ("NOT", 0.08), ("BUF", 0.04)],
        "xor_dominant": [("AND", 0.10), ("OR", 0.10), ("XOR", 0.55), ("XNOR", 0.15), ("NOT", 0.06), ("BUF", 0.04)],
        "or_dominant": [("AND", 0.12), ("OR", 0.60), ("XOR", 0.08), ("XNOR", 0.06), ("NOT", 0.10), ("BUF", 0.04)],
        "xnor_dominant": [("AND", 0.10), ("OR", 0.08), ("XOR", 0.12), ("XNOR", 0.55), ("NOT", 0.10), ("BUF", 0.05)],
        "nand_style": [("AND", 0.45), ("OR", 0.10), ("XOR", 0.10), ("XNOR", 0.05), ("NOT", 0.25), ("BUF", 0.05)],
        "mixed_control": [("AND", 0.25), ("OR", 0.25), ("XOR", 0.15), ("XNOR", 0.10), ("NOT", 0.20), ("BUF", 0.05)],
    }[family]

    for i in range(n_nodes):
        lhs = f"n{family}_{idx}_{i}"
        op = weighted_choice(rng, weights)
        if op in ("NOT", "BUF"):
            a = choose(rng, names)
            line = f"{lhs} = {op}({a})"
        else:
            a = choose(rng, names)
            b = choose(rng, names)
            line = f"{lhs} = {op}({a},{b})"
        lines.append(line)
        names.append(lhs)

        # extra nand-like structure
        if family == "nand_style" and rng.random() < 0.25:
            lhs_and = f"na_{family}_{idx}_{i}"
            lhs_not = f"nn_{family}_{idx}_{i}"
            a = choose(rng, names)
            b = choose(rng, names)
            lines.append(f"{lhs_and} = AND({a},{b})")
            lines.append(f"{lhs_not} = NOT({lhs_and})")
            names.append(lhs_and)
            names.append(lhs_not)

    out_name = choose(rng, names)
    lines.append(f"OUTPUT({out_name})")
    return "\n".join(lines) + "\n"


def lit_from_id(node_id: int, neg: bool) -> int:
    return node_id * 2 + (1 if neg else 0)


def main():
    args = parse_args()
    rng = random.Random(args.seed)

    root = Path(args.out_dir)
    fam_dir = root / "bench_families"
    fam_dir.mkdir(parents=True, exist_ok=True)
    families = [
        "and_dominant",
        "xor_dominant",
        "or_dominant",
        "xnor_dominant",
        "nand_style",
        "mixed_control",
    ]

    total = 0
    for fam in families:
        fd = fam_dir / fam
        fd.mkdir(parents=True, exist_ok=True)
        for i in range(args.per_family):
            text = gen_bench_instance(rng, i, fam)
            (fd / f"{fam}_{i:04d}.bench").write_text(text, encoding="utf-8")
            total += 1

    # one flat directory for simple benchmark commands
    flat_dir = root / "bench_all"
    flat_dir.mkdir(parents=True, exist_ok=True)
    for p in fam_dir.rglob("*.bench"):
        dst = flat_dir / p.name
        dst.write_text(p.read_text(encoding="utf-8"), encoding="utf-8")

    print(f"generated bench_total={total} families={len(families)} seed={args.seed}")
    print(f"flat_bench={len(list(flat_dir.glob('*.bench')))}")


if __name__ == "__main__":
    main()
