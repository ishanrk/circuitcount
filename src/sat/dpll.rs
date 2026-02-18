use crate::cnf::cnf::{Cnf, Lit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SatResult {
    Sat(Vec<bool>),
    Unsat,
}

pub fn solve(cnf: &Cnf) -> SatResult {
    let mut assignment = vec![None; cnf.num_vars as usize + 1];
    if search(cnf, &mut assignment) {
        // fill free vars deterministically
        let model = assignment
            .into_iter()
            .map(|v| v.unwrap_or(false))
            .collect::<Vec<_>>();
        SatResult::Sat(model)
    } else {
        SatResult::Unsat
    }
}

pub fn is_sat(cnf: &Cnf) -> bool {
    matches!(solve(cnf), SatResult::Sat(_))
}

pub fn solve_model(cnf: &Cnf) -> Option<Vec<bool>> {
    match solve(cnf) {
        SatResult::Sat(m) => Some(m),
        SatResult::Unsat => None,
    }
}

fn search(cnf: &Cnf, assignment: &mut [Option<bool>]) -> bool {
    if !unit_propagate(cnf, assignment) {
        return false;
    }
    match cnf.eval_formula_partial(assignment) {
        Some(true) => return true,
        Some(false) => return false,
        None => {}
    }

    let var = first_unassigned(assignment);
    let Some(var) = var else {
        return false;
    };

    let mut try_true = assignment.to_vec();
    try_true[var] = Some(true);
    if search(cnf, &mut try_true) {
        assignment.copy_from_slice(&try_true);
        return true;
    }

    let mut try_false = assignment.to_vec();
    try_false[var] = Some(false);
    if search(cnf, &mut try_false) {
        assignment.copy_from_slice(&try_false);
        return true;
    }

    false
}

fn unit_propagate(cnf: &Cnf, assignment: &mut [Option<bool>]) -> bool {
    loop {
        let mut changed = false;

        for clause in &cnf.clauses {
            let mut open_count = 0usize;
            let mut last_open = Lit::new(0, true);
            let mut has_true = false;

            for &lit in clause {
                match Cnf::eval_lit_partial(lit, assignment) {
                    Some(true) => {
                        has_true = true;
                        break;
                    }
                    Some(false) => {}
                    None => {
                        open_count += 1;
                        last_open = lit;
                    }
                }
            }

            if has_true {
                continue;
            }
            if open_count == 0 {
                return false;
            }
            if open_count == 1 {
                let var = last_open.var as usize;
                let need = last_open.sign;
                match assignment[var] {
                    Some(v) if v != need => return false,
                    Some(_) => {}
                    None => {
                        assignment[var] = Some(need);
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return true;
        }
    }
}

fn first_unassigned(assignment: &[Option<bool>]) -> Option<usize> {
    (1..assignment.len()).find(|&i| assignment[i].is_none())
}
