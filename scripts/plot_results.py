#!/usr/bin/env python3
import argparse
import os
import re
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--csv", required=True)
    p.add_argument("--out_dir", required=True)
    return p.parse_args()


NUM_COLS = [
    "wall_ms",
    "solve_calls",
    "result",
    "m",
    "trials",
    "r",
    "seed",
    "file_bytes",
    "aig_inputs",
    "aig_ands",
    "cone_inputs",
    "cnf_vars",
    "cnf_clauses",
]


def infer_family_from_file(path_text: str, root_hint: Path) -> str:
    p = Path(path_text)
    if p.exists() and p.suffix.lower() == ".bench":
        text = p.read_text(encoding="utf-8", errors="ignore")
        ops = re.findall(r"=\s*([A-Z]+)\(", text)
        counts = {}
        for op in ops:
            counts[op] = counts.get(op, 0) + 1
        if counts:
            top = max(counts.items(), key=lambda x: x[1])[0]
            return top.lower() + "_dominant"

    # fallback path rule
    p2 = Path(path_text.replace("\\", "/"))
    parts = p2.parts
    root_parts = root_hint.parts
    if len(parts) > len(root_parts) + 1 and parts[: len(root_parts)] == root_parts:
        return parts[len(root_parts)]
    stem = p2.stem
    if "_" in stem:
        return stem.split("_", 1)[0]
    if "-" in stem:
        return stem.split("-", 1)[0]
    return stem


def size_buckets(ok_df: pd.DataFrame) -> np.ndarray:
    vals = ok_df["cnf_clauses"].dropna().astype(float)
    if vals.empty:
        return np.array([0.0, 1.0])
    q = np.unique(np.quantile(vals, [0.0, 0.25, 0.5, 0.75, 1.0]))
    if len(q) < 2:
        q = np.array([vals.min(), vals.max() + 1.0])
    if q[0] == q[-1]:
        q = np.array([q[0], q[0] + 1.0])
    return q


def add_metrics(df: pd.DataFrame, root_hint: Path) -> pd.DataFrame:
    out = df.copy()
    for c in NUM_COLS:
        if c in out.columns:
            out[c] = pd.to_numeric(out[c], errors="coerce")
    out["family"] = out["path"].fillna("").map(lambda p: infer_family_from_file(str(p), root_hint))
    out["time_per_call_ms"] = out["wall_ms"] / out["solve_calls"].clip(lower=1)
    out["vars_per_clause"] = out["cnf_vars"] / out["cnf_clauses"].clip(lower=1)
    out["cone_frac"] = out["cone_inputs"] / out["aig_inputs"].clip(lower=1)
    out["ands_per_cone_in"] = out["aig_ands"] / out["cone_inputs"].clip(lower=1)
    # diversity proxy from cone pressure
    out["diversity_score"] = out["cone_frac"] * out["ands_per_cone_in"]
    return out


def plot_time_hist(ok_df: pd.DataFrame, out_path: Path):
    plt.figure(figsize=(10, 4.5))
    gobj = ok_df.groupby("size_bucket", observed=False)
    groups = [g["wall_ms"].values for _, g in gobj]
    labels = [str(k) for k, _ in gobj]
    if groups:
        plt.boxplot(groups, tick_labels=labels, showfliers=False)
    med = ok_df["wall_ms"].median()
    p90 = ok_df["wall_ms"].quantile(0.9)
    plt.title(f"wall_ms by cnf size bucket | median={med:.2f} p90={p90:.2f}")
    plt.xlabel("cnf size bucket")
    plt.ylabel("wall_ms")
    plt.xticks(rotation=20, ha="right")
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def plot_solve_calls_hist(ok_df: pd.DataFrame, out_path: Path):
    plt.figure(figsize=(10, 4.5))
    gobj = ok_df.groupby("size_bucket", observed=False)
    groups = [g["solve_calls"].values for _, g in gobj]
    labels = [str(k) for k, _ in gobj]
    if groups:
        plt.boxplot(groups, tick_labels=labels, showfliers=False)
    plt.title("solve_calls by cnf size bucket")
    plt.xlabel("cnf size bucket")
    plt.ylabel("solve_calls")
    plt.xticks(rotation=20, ha="right")
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def plot_time_vs_vars_per_clause(ok_df: pd.DataFrame, out_path: Path):
    plt.figure(figsize=(9, 5))
    d = ok_df.dropna(subset=["vars_per_clause", "time_per_call_ms", "diversity_score"]).copy()
    if d.empty:
        plt.title("time per call vs vars per clause and diversity")
        plt.tight_layout()
        plt.savefig(out_path)
        plt.close()
        return

    # color by diversity bucket
    q = np.unique(np.quantile(d["diversity_score"], [0.0, 0.33, 0.66, 1.0]))
    if len(q) < 2:
        q = np.array([d["diversity_score"].min(), d["diversity_score"].max() + 1e-9])
    d["div_bucket"] = pd.cut(d["diversity_score"], bins=q, include_lowest=True, duplicates="drop")
    for key, group in d.groupby("div_bucket", observed=False):
        plt.scatter(group["cnf_clauses"], group["wall_ms"], s=25, label=str(key), alpha=0.8)
    x = d["vars_per_clause"].dropna()
    y = d["time_per_call_ms"].dropna()
    if len(x) >= 4:
        bins = np.unique(np.quantile(x, np.linspace(0.0, 1.0, 8)))
        if len(bins) >= 2:
            idx = np.digitize(x, bins[1:-1], right=True)
            med_x, med_y = [], []
            for b in sorted(set(idx)):
                mask = idx == b
                med_x.append(np.median(x[mask]))
                med_y.append(np.median(y[mask]))
            plt.plot(med_x, med_y, linewidth=2, label="binned median")
    plt.title("time_per_call_ms vs vars_per_clause with diversity")
    plt.xlabel("vars_per_clause")
    plt.ylabel("time_per_call_ms")
    if len(d) > 0:
        plt.legend()
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def plot_family_summary(ok_df: pd.DataFrame, out_path: Path, min_n: int = 5):
    g = ok_df.groupby("family")["wall_ms"]
    agg = g.agg(["count", "median", lambda x: x.quantile(0.9)]).reset_index()
    agg.columns = ["family", "count", "median", "p90"]
    agg = agg[agg["count"] >= min_n].sort_values("median")
    if agg.empty:
        plt.figure(figsize=(8, 4))
        plt.title("family wall_ms summary (count >= 5)")
        plt.tight_layout()
        plt.savefig(out_path)
        plt.close()
        return
    plt.figure(figsize=(10, 5.5))
    x = np.arange(len(agg))
    plt.plot(x, agg["median"], marker="o", label="median wall_ms")
    plt.plot(x, agg["p90"], marker="o", label="p90 wall_ms")
    plt.xticks(x, agg["family"], rotation=20, ha="right")
    plt.title("model count time by circuit family")
    plt.ylabel("wall_ms")
    plt.legend()
    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()


def write_report(df: pd.DataFrame, ok_df: pd.DataFrame, out_path: Path):
    total = len(df)
    ok_n = int((df["status"] == "ok").sum())
    timeout_n = int((df["status"] == "timeout").sum())
    med = ok_df["wall_ms"].median() if not ok_df.empty else float("nan")
    p90 = ok_df["wall_ms"].quantile(0.9) if not ok_df.empty else float("nan")

    fam = ok_df.groupby("family")["wall_ms"].agg(["count", "median"]).reset_index()
    fam_best = fam.sort_values("median").head(5)
    fam_worst = fam.sort_values("median", ascending=False).head(5)

    largest_bucket = (
        df["size_bucket_all"].dropna().astype(str).sort_values().iloc[-1]
        if df["size_bucket_all"].notna().any()
        else ""
    )
    largest = df[df["size_bucket_all"].astype(str) == largest_bucket]
    largest_med = (
        largest[largest["status"] == "ok"]["wall_ms"].median()
        if not largest.empty
        else float("nan")
    )
    largest_timeout = (
        (largest["status"] == "timeout").mean() if not largest.empty else float("nan")
    )

    lines = []
    lines.append(f"dataset_rows: {total}")
    lines.append(f"ok_rows: {ok_n}")
    lines.append(f"timeout_rows: {timeout_n}")
    lines.append(f"median_wall_ms_ok: {med:.3f}" if pd.notna(med) else "median_wall_ms_ok: nan")
    lines.append(f"p90_wall_ms_ok: {p90:.3f}" if pd.notna(p90) else "p90_wall_ms_ok: nan")
    lines.append("best_families_by_median_wall_ms:")
    for _, r in fam_best.iterrows():
        lines.append(f"  {r['family']}: median={r['median']:.3f}, count={int(r['count'])}")
    lines.append("worst_families_by_median_wall_ms:")
    for _, r in fam_worst.iterrows():
        lines.append(f"  {r['family']}: median={r['median']:.3f}, count={int(r['count'])}")
    lines.append(f"largest_size_bucket: {largest_bucket}")
    lines.append(
        f"largest_bucket_median_wall_ms_ok: {largest_med:.3f}"
        if pd.notna(largest_med)
        else "largest_bucket_median_wall_ms_ok: nan"
    )
    lines.append(
        f"largest_bucket_timeout_rate: {largest_timeout:.3f}"
        if pd.notna(largest_timeout)
        else "largest_bucket_timeout_rate: nan"
    )
    corr = ok_df[["vars_per_clause", "time_per_call_ms"]].dropna()
    if len(corr) >= 2:
        cval = corr["vars_per_clause"].corr(corr["time_per_call_ms"])
        lines.append(f"corr_vars_per_clause_time_per_call: {cval:.3f}")
    else:
        lines.append("corr_vars_per_clause_time_per_call: nan")
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main():
    args = parse_args()
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    df = pd.read_csv(args.csv)

    root_hint = Path(os.path.commonpath([str(p).replace("\\", "/") for p in df["path"].dropna()]))
    df = add_metrics(df, root_hint)
    ok_df = df[df["status"] == "ok"].copy()

    bucket_idx = size_buckets(ok_df if not ok_df.empty else df.fillna(0))
    if not ok_df.empty:
        ok_df["size_bucket"] = pd.cut(
            ok_df["cnf_clauses"], bins=bucket_idx, include_lowest=True, duplicates="drop"
        )
    else:
        ok_df["size_bucket"] = pd.Series(dtype="object")
    df["size_bucket_all"] = pd.cut(
        df["cnf_clauses"], bins=bucket_idx, include_lowest=True, duplicates="drop"
    )

    plot_time_hist(ok_df, out_dir / "time_hist.png")
    plot_solve_calls_hist(ok_df, out_dir / "solve_calls_hist.png")
    plot_time_vs_vars_per_clause(ok_df, out_dir / "time_vs_cnf.png")
    plot_family_summary(ok_df, out_dir / "family_summary.png", min_n=5)
    write_report(df, ok_df, out_dir / "report.md")


if __name__ == "__main__":
    main()
