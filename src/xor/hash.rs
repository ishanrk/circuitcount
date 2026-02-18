use anyhow::{Result, bail};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use super::encode::XorConstraint;

pub fn sample_constraints(
    vars: &[u32],
    m: usize,
    p: f64,
    rng: &mut ChaCha8Rng,
) -> Result<Vec<XorConstraint>> {
    if vars.is_empty() {
        return Ok(Vec::new());
    }
    if !(p > 0.0 && p <= 1.0) {
        bail!("sparsity p must be in (0,1], got {}", p);
    }

    let mut out = Vec::with_capacity(m);
    for _ in 0..m {
        let mut picked = Vec::new();
        for &v in vars {
            if rng.random::<f64>() < p {
                picked.push(v);
            }
        }

        // avoid empty xor rows by default
        if picked.is_empty() {
            let idx = rng.random_range(0..vars.len());
            picked.push(vars[idx]);
        }
        picked.sort_unstable();
        picked.dedup();

        out.push(XorConstraint {
            vars: picked,
            rhs: rng.random::<bool>(),
        });
    }
    Ok(out)
}
