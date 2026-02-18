use crate::cnf::cnf::Lit;
use crate::solver::IncrementalSolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    pub act: Lit,
}

pub fn new_scope<S: IncrementalSolver + ?Sized>(solver: &mut S) -> Scope {
    let v = solver.new_var();
    Scope {
        act: Lit::new(v, true),
    }
}

pub fn add_scoped_clause<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    scope: &Scope,
    mut clause: Vec<Lit>,
) {
    // clause is active only when scope literal is assumed true
    let mut scoped = Vec::with_capacity(clause.len() + 1);
    scoped.push(scope.act.neg());
    scoped.append(&mut clause);
    solver.add_clause(scoped);
}
