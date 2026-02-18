use crate::cnf::cnf::{Cnf, Lit};
use crate::sat::dpll::solve_model;
use crate::solver::{IncrementalSolver, SolveResult, SolverStats};

#[derive(Debug, Clone)]
pub struct DpllSolverBackend {
    cnf: Cnf,
    last_model: Option<Vec<bool>>,
    stats: SolverStats,
}

impl DpllSolverBackend {
    pub fn new() -> Self {
        Self {
            cnf: Cnf::new(0),
            last_model: None,
            stats: SolverStats::default(),
        }
    }
}

impl Default for DpllSolverBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalSolver for DpllSolverBackend {
    fn new_var(&mut self) -> u32 {
        self.cnf.fresh_var()
    }

    fn add_clause(&mut self, clause: Vec<Lit>) {
        self.cnf.add_clause(clause);
    }

    fn solve(&mut self, assumptions: &[Lit]) -> SolveResult {
        self.stats.solve_calls += 1;
        let mut work = self.cnf.clone();
        for &a in assumptions {
            work.add_clause(vec![a]);
        }
        self.last_model = solve_model(&work);
        if self.last_model.is_some() {
            SolveResult::Sat
        } else {
            SolveResult::Unsat
        }
    }

    fn model_value(&self, var: u32) -> Option<bool> {
        let idx = var as usize;
        self.last_model
            .as_ref()
            .and_then(|m| if idx < m.len() { Some(m[idx]) } else { None })
    }

    fn stats(&self) -> SolverStats {
        self.stats
    }

    fn backend_name(&self) -> &'static str {
        "dpll"
    }
}
