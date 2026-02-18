use anyhow::{Result, bail};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::circuit::aig::Aig;
use crate::cnf::cnf::Lit;
use crate::cnf::tseitin::encode_aig;
use crate::count::bounded::{BoundedCount, projected_count_bounded_session};
use crate::solver::dpll_backend::DpllSolverBackend;
use crate::solver::varisat::VarisatSolver;
use crate::solver::IncrementalSolver;
use crate::xor::encode::{XorConstraint, append_xor_constraint_to_solver};
use crate::xor::hash::sample_constraints;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    Exact,
    Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountBackend {
    Dpll,
    Varisat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountOptions {
    pub seed: u64,
    pub pivot: usize,
    pub trials: usize,
    pub sparsity: f64,
    pub backend: CountBackend,
    pub progress: bool,
    pub repeats: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountReport {
    pub inputs_coi: usize,
    pub ands: usize,
    pub vars: u32,
    pub clauses: usize,
    pub pivot: usize,
    pub trials: usize,
    pub result: u128,
    pub mode: CountMode,
    pub m_used: usize,
    pub backend: &'static str,
    pub solve_calls: usize,
}

pub fn count_output(
    aig: &Aig,
    out_idx: usize,
    seed: u64,
    pivot: usize,
    trials: usize,
    sparsity: f64,
) -> Result<CountReport> {
    count_output_with_options(
        aig,
        out_idx,
        CountOptions {
            seed,
            pivot,
            trials,
            sparsity,
            backend: CountBackend::Dpll,
            progress: false,
            repeats: 3,
        },
    )
}

pub fn count_output_with_options(
    aig: &Aig,
    out_idx: usize,
    opts: CountOptions,
) -> Result<CountReport> {
    if opts.trials == 0 {
        bail!("trials must be >= 1");
    }
    if opts.pivot == 0 {
        bail!("pivot must be >= 1");
    }
    if opts.repeats == 0 {
        bail!("r must be >= 1");
    }
    if !(opts.sparsity > 0.0 && opts.sparsity <= 1.0) {
        bail!("p must be in (0,1], got {}", opts.sparsity);
    }

    let simple = aig.simplify_output(out_idx)?;
    let mut enc = encode_aig(&simple)?;
    let out_lit = *enc
        .output_lits
        .first()
        .ok_or_else(|| anyhow::anyhow!("simplified circuit has no output"))?;
    enc.cnf.add_clause(vec![out_lit]);
    let proj = enc.input_vars.clone();

    let mut solver = build_solver(opts.backend, enc.cnf.num_vars);
    for clause in &enc.cnf.clauses {
        solver.add_clause(clause.clone());
    }

    let exact = projected_count_bounded_session(&mut *solver, &proj, opts.pivot, &[])?;
    if !exact.hit_cap {
        let st = solver.stats();
        return Ok(CountReport {
            inputs_coi: simple.num_inputs(),
            ands: simple.num_ands(),
            vars: enc.cnf.num_vars,
            clauses: enc.cnf.clauses.len(),
            pivot: opts.pivot,
            trials: opts.trials,
            result: exact.count as u128,
            mode: CountMode::Exact,
            m_used: 0,
            backend: solver.backend_name(),
            solve_calls: st.solve_calls,
        });
    }

    let mut trial_est = Vec::<(u128, usize)>::new();
    for t in 0..opts.trials {
        let mut rng = ChaCha8Rng::seed_from_u64(opts.seed.wrapping_add(t as u64));
        let max_m = proj.len();
        let base_rows = sample_constraints(&proj, max_m, opts.sparsity, &mut rng)?;
        let row_acts = add_trial_rows(&mut *solver, &base_rows)?;

        let mut low = 0usize;
        let mut high = max_m + 1;
        let mut m = 1usize;
        while m <= max_m {
            let c = cell_count_with_prefix(&mut *solver, &proj, opts.pivot, &row_acts, m)?;
            if opts.progress {
                println!("trial={} stage=ramp m={} cell={}", t, m, c.count);
            }
            if c.hit_cap {
                low = m;
                m = (m * 2).min(max_m + 1);
            } else {
                high = m;
                break;
            }
        }

        if high == max_m + 1 {
            trial_est.push((0, 0));
            continue;
        }

        let mut chosen = None::<(usize, BoundedCount)>;
        let mut lo = low + 1;
        let mut hi = high;
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let c = cell_count_with_prefix(&mut *solver, &proj, opts.pivot, &row_acts, mid)?;
            if opts.progress {
                println!("trial={} stage=bin m={} cell={}", t, mid, c.count);
            }
            if c.hit_cap {
                lo = mid + 1;
            } else if c.count == 0 {
                if mid == 0 {
                    break;
                }
                hi = mid.saturating_sub(1);
            } else {
                chosen = Some((mid, c));
                if mid == 0 {
                    break;
                }
                hi = mid.saturating_sub(1);
            }
        }

        let Some((m_pick, first_cell)) = chosen else {
            trial_est.push((0, 0));
            continue;
        };

        let mut cells = vec![first_cell.count as u128];
        for rep in 1..opts.repeats {
            let rows = sample_constraints(&proj, m_pick, opts.sparsity, &mut rng)?;
            let acts = add_trial_rows(&mut *solver, &rows)?;
            let c = cell_count_with_prefix(&mut *solver, &proj, opts.pivot, &acts, m_pick)?;
            if opts.progress {
                println!("trial={} stage=rep m={} rep={} cell={}", t, m_pick, rep, c.count);
            }
            cells.push(c.count as u128);
        }
        cells.sort_unstable();
        let med_cell = cells[cells.len() / 2];
        let scale = 1u128
            .checked_shl(m_pick as u32)
            .ok_or_else(|| anyhow::anyhow!("m={} is too large for u128 scaling", m_pick))?;
        trial_est.push((med_cell * scale, m_pick));
    }

    trial_est.sort_by_key(|x| x.0);
    let mid = trial_est[trial_est.len() / 2];
    let st = solver.stats();
    Ok(CountReport {
        inputs_coi: simple.num_inputs(),
        ands: simple.num_ands(),
        vars: enc.cnf.num_vars,
        clauses: enc.cnf.clauses.len(),
        pivot: opts.pivot,
        trials: opts.trials,
        result: mid.0,
        mode: CountMode::Hash,
        m_used: mid.1,
        backend: solver.backend_name(),
        solve_calls: st.solve_calls,
    })
}

fn build_solver(kind: CountBackend, num_vars: u32) -> Box<dyn IncrementalSolver> {
    match kind {
        CountBackend::Dpll => {
            let mut s = DpllSolverBackend::new();
            for _ in 0..num_vars {
                s.new_var();
            }
            Box::new(s)
        }
        CountBackend::Varisat => {
            let mut s = VarisatSolver::new();
            for _ in 0..num_vars {
                s.new_var();
            }
            Box::new(s)
        }
    }
}

fn add_trial_rows(solver: &mut dyn IncrementalSolver, rows: &[XorConstraint]) -> Result<Vec<Lit>> {
    let mut acts = Vec::with_capacity(rows.len());
    for row in rows {
        let a = Lit::new(solver.new_var(), true);
        append_xor_constraint_to_solver(solver, row, Some(a))?;
        acts.push(a);
    }
    Ok(acts)
}

fn cell_count_with_prefix(
    solver: &mut dyn IncrementalSolver,
    projection: &[u32],
    pivot: usize,
    acts: &[Lit],
    m: usize,
) -> Result<BoundedCount> {
    let end = m.min(acts.len());
    projected_count_bounded_session(solver, projection, pivot, &acts[..end])
}
