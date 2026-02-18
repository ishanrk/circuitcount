use anyhow::{Result, bail};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::circuit::aig::Aig;
use crate::cnf::tseitin::encode_aig;
use crate::count::bounded::projected_count_bounded;
use crate::xor::encode::{XorBlockMode, append_xor_block};
use crate::xor::hash::sample_constraints;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    Exact,
    Hash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountReport {
    pub inputs_coi: usize,
    pub ands: usize,
    pub vars: u32,
    pub clauses: usize,
    pub pivot: usize,
    pub trials: usize,
    pub result: u128,
    pub mode: CountMode,
    pub m_used: usize,
}

pub fn count_output(
    aig: &Aig,
    out_idx: usize,
    seed: u64,
    pivot: usize,
    trials: usize,
    sparsity: f64,
) -> Result<CountReport> {
    if trials == 0 {
        bail!("trials must be >= 1");
    }
    if pivot == 0 {
        bail!("pivot must be >= 1");
    }
    if !(sparsity > 0.0 && sparsity <= 1.0) {
        bail!("p must be in (0,1], got {}", sparsity);
    }

    let simple = aig.simplify_output(out_idx)?;
    let mut enc = encode_aig(&simple)?;
    let out_lit = *enc
        .output_lits
        .first()
        .ok_or_else(|| anyhow::anyhow!("simplified circuit has no output"))?;
    enc.cnf.add_clause(vec![out_lit]);

    let proj = enc.input_vars.clone();
    let base_count = projected_count_bounded(&enc.cnf, &proj, pivot)?;
    if !base_count.hit_cap {
        return Ok(CountReport {
            inputs_coi: simple.num_inputs(),
            ands: simple.num_ands(),
            vars: enc.cnf.num_vars,
            clauses: enc.cnf.clauses.len(),
            pivot,
            trials,
            result: base_count.count as u128,
            mode: CountMode::Exact,
            m_used: 0,
        });
    }

    let mut trial_estimates = Vec::<(u128, usize)>::new();
    for i in 0..trials {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(i as u64));
        let mut picked = None::<(u128, usize)>;

        for m in 1..=proj.len() {
            let constraints = sample_constraints(&proj, m, sparsity, &mut rng)?;
            let mut trial_cnf = enc.cnf.clone();
            append_xor_block(
                &mut trial_cnf,
                &constraints,
                XorBlockMode::Gated { activate: true },
            )?;

            let cell = projected_count_bounded(&trial_cnf, &proj, pivot)?;
            if !cell.hit_cap && cell.count >= 1 && cell.count <= pivot {
                let scale = 1u128
                    .checked_shl(m as u32)
                    .ok_or_else(|| anyhow::anyhow!("m={} is too large for u128 scaling", m))?;
                picked = Some(((cell.count as u128) * scale, m));
                break;
            }
        }

        trial_estimates.push(picked.unwrap_or((0, 0)));
    }

    trial_estimates.sort_by_key(|x| x.0);
    let mid = trial_estimates[trial_estimates.len() / 2];
    Ok(CountReport {
        inputs_coi: simple.num_inputs(),
        ands: simple.num_ands(),
        vars: enc.cnf.num_vars,
        clauses: enc.cnf.clauses.len(),
        pivot,
        trials,
        result: mid.0,
        mode: CountMode::Hash,
        m_used: mid.1,
    })
}
