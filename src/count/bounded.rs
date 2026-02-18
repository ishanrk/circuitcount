use anyhow::{Result, bail};

use crate::cnf::cnf::{Cnf, Lit};
use crate::solver::dpll_backend::DpllSolverBackend;
use crate::solver::scope::{Scope, add_scoped_clause, new_scope};
use crate::solver::{IncrementalSolver, SolveResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCount {
    pub count: usize,
    pub hit_cap: bool,
}

pub fn projected_count_bounded(cnf: &Cnf, projection: &[u32], cap: usize) -> Result<BoundedCount> {
    let mut solver = DpllSolverBackend::new();
    for _ in 0..cnf.num_vars {
        solver.new_var();
    }
    for clause in &cnf.clauses {
        solver.add_clause(clause.clone());
    }
    let always = Vec::<Lit>::new();
    projected_count_bounded_session(&mut solver, projection, cap, &always)
}

pub fn projected_count_bounded_session<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    projection: &[u32],
    cap: usize,
    base_assumptions: &[Lit],
) -> Result<BoundedCount> {
    if projection.iter().any(|&v| v == 0) {
        bail!("projection contains variable 0");
    }
    let count_scope = new_scope(solver);
    projected_count_bounded_in_scope(solver, projection, cap, base_assumptions, &count_scope)
}

pub fn projected_count_bounded_in_scope<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    projection: &[u32],
    cap: usize,
    base_assumptions: &[Lit],
    count_scope: &Scope,
) -> Result<BoundedCount> {
    let mut count = 0usize;

    loop {
        let mut assumptions = Vec::with_capacity(base_assumptions.len() + 1);
        assumptions.extend_from_slice(base_assumptions);
        assumptions.push(count_scope.act);

        match solver.solve(&assumptions) {
            SolveResult::Unsat => {
                return Ok(BoundedCount {
                    count,
                    hit_cap: false,
                });
            }
            SolveResult::Sat => {}
        }

        count += 1;
        if count > cap {
            return Ok(BoundedCount {
                count,
                hit_cap: true,
            });
        }

        // block this projected assignment
        let mut block = Vec::with_capacity(projection.len());
        for &v in projection {
            let val = solver
                .model_value(v)
                .ok_or_else(|| anyhow::anyhow!("missing model value for var {}", v))?;
            block.push(Lit::new(v, !val));
        }
        add_scoped_clause(solver, count_scope, block);
    }
}
