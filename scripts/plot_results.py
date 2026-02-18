#!/usr/bin/env python3
import argparse
import csv
import os

import matplotlib.pyplot as plt


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--csv", required=True)
    p.add_argument("--out_dir", required=True)
    return p.parse_args()


def to_num(x, cast):
    if x is None:
        return None
    x = x.strip()
    if x == "":
        return None
    try:
        return cast(x)
    except Exception:
        return None


def load_rows(path):
    rows = []
    with open(path, "r", newline="", encoding="utf-8") as f:
        r = csv.DictReader(f)
        for row in r:
            rows.append(
                {
                    "backend": row.get("backend", ""),
                    "wall_ms": to_num(row.get("wall_ms", ""), float),
                    "solve_calls": to_num(row.get("solve_calls", ""), float),
                    "cnf_clauses": to_num(row.get("cnf_clauses", ""), float),
                }
            )
    return rows


def by_backend(rows, key):
    out = {}
    for row in rows:
        b = row.get("backend", "")
        v = row.get(key)
        if b == "" or v is None:
            continue
        out.setdefault(b, []).append(v)
    return out


def plot_hist(data, title, xlabel, out_path):
    plt.figure(figsize=(8, 4))
    for backend, vals in sorted(data.items()):
        if len(vals) == 0:
            continue
        plt.hist(vals, bins=20, alpha=0.5, label=backend)
    plt.title(title)
    plt.xlabel(xlabel)
    plt.ylabel("count")
    if len(data) > 0:
        plt.legend()
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def plot_scatter(rows, out_path):
    plt.figure(figsize=(8, 4))
    by_b = {}
    for row in rows:
        b = row.get("backend", "")
        x = row.get("cnf_clauses")
        y = row.get("wall_ms")
        if b == "" or x is None or y is None:
            continue
        by_b.setdefault(b, {"x": [], "y": []})
        by_b[b]["x"].append(x)
        by_b[b]["y"].append(y)
    for backend, vals in sorted(by_b.items()):
        plt.scatter(vals["x"], vals["y"], label=backend, s=18)
    plt.title("wall_ms vs cnf_clauses")
    plt.xlabel("cnf_clauses")
    plt.ylabel("wall_ms")
    if len(by_b) > 0:
        plt.legend()
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def main():
    args = parse_args()
    os.makedirs(args.out_dir, exist_ok=True)
    rows = load_rows(args.csv)

    wall = by_backend(rows, "wall_ms")
    calls = by_backend(rows, "solve_calls")

    plot_hist(
        wall,
        "wall time histogram",
        "wall_ms",
        os.path.join(args.out_dir, "time_hist.png"),
    )
    plot_hist(
        calls,
        "solve calls histogram",
        "solve_calls",
        os.path.join(args.out_dir, "solve_calls_hist.png"),
    )
    plot_scatter(rows, os.path.join(args.out_dir, "time_vs_cnf.png"))


if __name__ == "__main__":
    main()
