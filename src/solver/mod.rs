pub mod dpll_backend;
pub mod scope;
pub mod varisat;

use crate::cnf::cnf::Lit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveResult {
    Sat,
    Unsat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SolverStats {
    pub solve_calls: usize,
    pub decisions: usize,
    pub conflicts: usize,
}

pub trait IncrementalSolver {
    fn new_var(&mut self) -> u32;
    fn add_clause(&mut self, clause: Vec<Lit>);
    fn solve(&mut self, assumptions: &[Lit]) -> SolveResult;
    fn model_value(&self, var: u32) -> Option<bool>;
    fn stats(&self) -> SolverStats;
    fn backend_name(&self) -> &'static str;
}
