#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit {
    pub var: u32,
    pub sign: bool,
}

impl Lit {
    pub fn new(var: u32, sign: bool) -> Self {
        Self { var, sign }
    }

    pub fn neg(self) -> Self {
        Self {
            var: self.var,
            sign: !self.sign,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cnf {
    pub num_vars: u32,
    pub clauses: Vec<Vec<Lit>>,
}

impl Cnf {
    pub fn new(num_vars: u32) -> Self {
        Self {
            num_vars,
            clauses: Vec::new(),
        }
    }

    pub fn add_clause(&mut self, clause: Vec<Lit>) {
        self.clauses.push(clause);
    }

    pub fn fresh_var(&mut self) -> u32 {
        self.num_vars = self.num_vars.saturating_add(1);
        self.num_vars
    }

    pub fn eval_lit_partial(lit: Lit, assignment: &[Option<bool>]) -> Option<bool> {
        let var = lit.var as usize;
        if var >= assignment.len() {
            return None;
        }
        assignment[var].map(|v| if lit.sign { v } else { !v })
    }

    pub fn eval_clause_partial(clause: &[Lit], assignment: &[Option<bool>]) -> Option<bool> {
        let mut any_unknown = false;
        for &lit in clause {
            match Self::eval_lit_partial(lit, assignment) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => any_unknown = true,
            }
        }
        if any_unknown { None } else { Some(false) }
    }

    pub fn eval_formula_partial(&self, assignment: &[Option<bool>]) -> Option<bool> {
        let mut all_true = true;
        for clause in &self.clauses {
            match Self::eval_clause_partial(clause, assignment) {
                Some(true) => {}
                Some(false) => return Some(false),
                None => all_true = false,
            }
        }
        if all_true { Some(true) } else { None }
    }
}
