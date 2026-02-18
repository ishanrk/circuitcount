use anyhow::{Result, bail};

use crate::circuit::aig::{Aig, AigLit};

use super::cnf::{Cnf, Lit};

#[derive(Debug, Clone)]
pub struct TseitinCnf {
    pub cnf: Cnf,
    pub input_vars: Vec<u32>,
    pub output_lits: Vec<Lit>,
    pub false_var: u32,
}

pub fn encode_aig(aig: &Aig) -> Result<TseitinCnf> {
    let false_var = aig
        .max_id
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("max_id is too large for false var allocation"))?;
    let mut cnf = Cnf::new(false_var);

    // pin the false var to false
    cnf.add_clause(vec![Lit::new(false_var, false)]);

    for &id in &aig.inputs {
        if id == 0 || id > aig.max_id {
            bail!("input id {} is invalid for max_id {}", id, aig.max_id);
        }
    }

    for gate in &aig.ands {
        if gate.id == 0 || gate.id > aig.max_id {
            bail!("and gate id {} is invalid for max_id {}", gate.id, aig.max_id);
        }
        if gate.a.id > aig.max_id || gate.b.id > aig.max_id {
            bail!(
                "and gate {} has fanin outside max_id {}",
                gate.id,
                aig.max_id
            );
        }

        let g = Lit::new(gate.id, true);
        let a = lit_from_aig(gate.a, false_var);
        let b = lit_from_aig(gate.b, false_var);

        // g -> a
        cnf.add_clause(vec![g.neg(), a]);
        // g -> b
        cnf.add_clause(vec![g.neg(), b]);
        // a & b -> g
        cnf.add_clause(vec![g, a.neg(), b.neg()]);
    }

    let mut out_lits = Vec::with_capacity(aig.outputs.len());
    for &out in &aig.outputs {
        if out.id > aig.max_id {
            bail!("output id {} is invalid for max_id {}", out.id, aig.max_id);
        }
        out_lits.push(lit_from_aig(out, false_var));
    }

    Ok(TseitinCnf {
        cnf,
        input_vars: aig.inputs.clone(),
        output_lits: out_lits,
        false_var,
    })
}

fn lit_from_aig(lit: AigLit, false_var: u32) -> Lit {
    let var = if lit.id == 0 { false_var } else { lit.id };
    Lit::new(var, !lit.neg)
}
