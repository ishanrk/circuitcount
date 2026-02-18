use anyhow::Result;

use crate::cnf::cnf::{Cnf, Lit};
use crate::solver::IncrementalSolver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XorConstraint {
    pub vars: Vec<u32>,
    pub rhs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XorBlockMode {
    Plain,
    Gated { activate: bool },
}

pub fn append_xor_block(
    cnf: &mut Cnf,
    constraints: &[XorConstraint],
    mode: XorBlockMode,
) -> Result<Option<u32>> {
    let act_var = match mode {
        XorBlockMode::Plain => None,
        XorBlockMode::Gated { .. } => Some(cnf.fresh_var()),
    };

    if let XorBlockMode::Gated { activate: true } = mode {
        let t = act_var.expect("activation var exists in gated mode");
        cnf.add_clause(vec![Lit::new(t, true)]);
    }

    for c in constraints {
        append_one(cnf, c, act_var)?;
    }
    Ok(act_var)
}

fn append_one(cnf: &mut Cnf, c: &XorConstraint, act_var: Option<u32>) -> Result<()> {
    if c.vars.is_empty() {
        if c.rhs {
            push_clause(cnf, Vec::new(), act_var);
        }
        return Ok(());
    }

    if c.vars.len() == 1 {
        let lit = Lit::new(c.vars[0], c.rhs);
        push_clause(cnf, vec![lit], act_var);
        return Ok(());
    }

    // chain xors with aux vars
    let mut acc = c.vars[0];
    for &next in &c.vars[1..] {
        let out = cnf.fresh_var();
        append_xor3(cnf, acc, next, out, act_var);
        acc = out;
    }

    let end = Lit::new(acc, c.rhs);
    push_clause(cnf, vec![end], act_var);
    Ok(())
}

fn append_xor3(cnf: &mut Cnf, x: u32, y: u32, z: u32, act_var: Option<u32>) {
    // z = x xor y
    push_clause(
        cnf,
        vec![Lit::new(x, true), Lit::new(y, true), Lit::new(z, false)],
        act_var,
    );
    push_clause(
        cnf,
        vec![Lit::new(x, false), Lit::new(y, false), Lit::new(z, false)],
        act_var,
    );
    push_clause(
        cnf,
        vec![Lit::new(x, true), Lit::new(y, false), Lit::new(z, true)],
        act_var,
    );
    push_clause(
        cnf,
        vec![Lit::new(x, false), Lit::new(y, true), Lit::new(z, true)],
        act_var,
    );
}

fn push_clause(cnf: &mut Cnf, mut clause: Vec<Lit>, act_var: Option<u32>) {
    if let Some(t) = act_var {
        let mut gated = Vec::with_capacity(clause.len() + 1);
        gated.push(Lit::new(t, false));
        gated.append(&mut clause);
        cnf.add_clause(gated);
    } else {
        cnf.add_clause(clause);
    }
}

pub fn append_xor_constraint_to_solver<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    constraint: &XorConstraint,
    activation: Option<Lit>,
) -> Result<()> {
    if constraint.vars.is_empty() {
        if constraint.rhs {
            push_solver_clause(solver, Vec::new(), activation);
        }
        return Ok(());
    }

    if constraint.vars.len() == 1 {
        push_solver_clause(
            solver,
            vec![Lit::new(constraint.vars[0], constraint.rhs)],
            activation,
        );
        return Ok(());
    }

    let mut acc = constraint.vars[0];
    for &next in &constraint.vars[1..] {
        let out = solver.new_var();
        append_xor3_solver(solver, acc, next, out, activation);
        acc = out;
    }
    push_solver_clause(solver, vec![Lit::new(acc, constraint.rhs)], activation);
    Ok(())
}

fn append_xor3_solver<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    x: u32,
    y: u32,
    z: u32,
    activation: Option<Lit>,
) {
    push_solver_clause(
        solver,
        vec![Lit::new(x, true), Lit::new(y, true), Lit::new(z, false)],
        activation,
    );
    push_solver_clause(
        solver,
        vec![Lit::new(x, false), Lit::new(y, false), Lit::new(z, false)],
        activation,
    );
    push_solver_clause(
        solver,
        vec![Lit::new(x, true), Lit::new(y, false), Lit::new(z, true)],
        activation,
    );
    push_solver_clause(
        solver,
        vec![Lit::new(x, false), Lit::new(y, true), Lit::new(z, true)],
        activation,
    );
}

fn push_solver_clause<S: IncrementalSolver + ?Sized>(
    solver: &mut S,
    mut clause: Vec<Lit>,
    activation: Option<Lit>,
) {
    if let Some(act) = activation {
        let mut scoped = Vec::with_capacity(clause.len() + 1);
        scoped.push(act.neg());
        scoped.append(&mut clause);
        solver.add_clause(scoped);
    } else {
        solver.add_clause(clause);
    }
}
