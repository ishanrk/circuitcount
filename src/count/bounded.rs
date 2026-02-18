use anyhow::{Result, bail};

use crate::cnf::cnf::{Cnf, Lit};
use crate::sat::dpll::solve_model;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCount {
    pub count: usize,
    pub hit_cap: bool,
}

pub fn projected_count_bounded(cnf: &Cnf, projection: &[u32], cap: usize) -> Result<BoundedCount> {
    if projection.iter().any(|&v| v == 0 || v > cnf.num_vars) {
        bail!("projection contains variable outside cnf range");
    }

    let mut work = cnf.clone();
    let mut count = 0usize;

    loop {
        let model = match solve_model(&work) {
            Some(m) => m,
            None => {
                return Ok(BoundedCount {
                    count,
                    hit_cap: false,
                });
            }
        };

        count += 1;
        if count > cap {
            return Ok(BoundedCount {
                count,
                hit_cap: true,
            });
        }

        // block this projected assignment
        let mut block = Vec::with_capacity(projection.len());
        for &v in projection {
            let val = model[v as usize];
            block.push(Lit::new(v, !val));
        }
        work.add_clause(block);
    }
}
