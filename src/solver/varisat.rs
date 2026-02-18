use crate::cnf::cnf::Lit;
use crate::solver::{IncrementalSolver, SolveResult, SolverStats};
use varisat::ExtendFormula;

pub struct VarisatSolver {
    inner: varisat::Solver<'static>,
    vars: Vec<varisat::Var>,
    last_model: Option<Vec<varisat::Lit>>,
    stats: SolverStats,
}

impl VarisatSolver {
    pub fn new() -> Self {
        Self {
            inner: varisat::Solver::new(),
            vars: Vec::new(),
            last_model: None,
            stats: SolverStats::default(),
        }
    }

    fn to_var(&self, v: u32) -> Option<varisat::Var> {
        if v == 0 {
            return None;
        }
        self.vars.get(v as usize - 1).copied()
    }

    fn to_lit(&self, lit: Lit) -> Option<varisat::Lit> {
        let var = self.to_var(lit.var)?;
        Some(varisat::Lit::from_var(var, lit.sign))
    }
}

impl Default for VarisatSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalSolver for VarisatSolver {
    fn new_var(&mut self) -> u32 {
        let v = self.inner.new_var();
        self.vars.push(v);
        self.vars.len() as u32
    }

    fn add_clause(&mut self, clause: Vec<Lit>) {
        let lits = clause
            .into_iter()
            .filter_map(|x| self.to_lit(x))
            .collect::<Vec<_>>();
        self.inner.add_clause(&lits);
    }

    fn solve(&mut self, assumptions: &[Lit]) -> SolveResult {
        self.stats.solve_calls += 1;
        let assumps = assumptions
            .iter()
            .copied()
            .filter_map(|x| self.to_lit(x))
            .collect::<Vec<_>>();
        self.inner.assume(&assumps);
        match self.inner.solve() {
            Ok(true) => {
                self.last_model = self.inner.model();
                SolveResult::Sat
            }
            Ok(false) => {
                self.last_model = None;
                SolveResult::Unsat
            }
            Err(_) => {
                self.last_model = None;
                SolveResult::Unsat
            }
        }
    }

    fn model_value(&self, var: u32) -> Option<bool> {
        let v = self.to_var(var)?;
        let model = self.last_model.as_ref()?;
        let pos = v.lit(true);
        let neg = v.lit(false);
        if model.contains(&pos) {
            Some(true)
        } else if model.contains(&neg) {
            Some(false)
        } else {
            None
        }
    }

    fn stats(&self) -> SolverStats {
        self.stats
    }

    fn backend_name(&self) -> &'static str {
        "varisat"
    }
}
