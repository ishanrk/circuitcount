use super::cnf::{Cnf, Lit};

pub fn to_dimacs(cnf: &Cnf) -> String {
    let mut out = String::new();
    out.push_str(&format!("p cnf {} {}\n", cnf.num_vars, cnf.clauses.len()));
    for clause in &cnf.clauses {
        for &lit in clause {
            out.push_str(&format!("{} ", lit_to_dimacs_int(lit)));
        }
        out.push_str("0\n");
    }
    out
}

fn lit_to_dimacs_int(lit: Lit) -> i64 {
    let v = lit.var as i64;
    if lit.sign { v } else { -v }
}
