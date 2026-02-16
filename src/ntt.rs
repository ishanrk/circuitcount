use anyhow::{Result, bail};

use crate::field::Fp;

pub fn naive_mul(a: &[Fp], b: &[Fp]) -> Vec<Fp> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![Fp::zero(); a.len() + b.len() - 1];
    for i in 0..a.len() {
        for j in 0..b.len() {
            out[i + j] += a[i] * b[j];
        }
    }
    out
}

pub fn ntt_mul(a: &[Fp], b: &[Fp]) -> Result<Vec<Fp>> {
    if a.is_empty() || b.is_empty() {
        return Ok(vec![]);
    }
    let target_len = a.len() + b.len() - 1;
    let n = target_len.next_power_of_two();
    if n > (1usize << 23) {
        bail!("input too large for modulus root-of-unity capacity");
    }
    let mut fa = vec![Fp::zero(); n];
    let mut fb = vec![Fp::zero(); n];
    fa[..a.len()].copy_from_slice(a);
    fb[..b.len()].copy_from_slice(b);

    ntt(&mut fa, false)?;
    ntt(&mut fb, false)?;
    for i in 0..n {
        fa[i] *= fb[i];
    }
    ntt(&mut fa, true)?;
    fa.truncate(target_len);
    Ok(fa)
}

pub fn ntt(values: &mut [Fp], invert: bool) -> Result<()> {
    let n = values.len();
    if !n.is_power_of_two() {
        bail!("NTT size {} is not a power of two", n);
    }
    let log_n = n.trailing_zeros() as usize;
    if n > (1usize << 23) {
        bail!("NTT size {} exceeds root-of-unity domain", n);
    }

    bit_reverse_permute(values);

    let mut len = 2usize;
    while len <= n {
        let step_log = len.trailing_zeros() as usize;
        let mut w_len = Fp::root_of_unity(step_log);
        if invert {
            w_len = w_len.inv();
        }

        for start in (0..n).step_by(len) {
            let mut w = Fp::one();
            for i in 0..(len / 2) {
                let u = values[start + i];
                let v = values[start + i + len / 2] * w;
                values[start + i] = u + v;
                values[start + i + len / 2] = u - v;
                w *= w_len;
            }
        }
        len <<= 1;
    }

    if invert {
        let inv_n = Fp::new(n as u64).inv();
        for x in values {
            *x *= inv_n;
        }
    }

    if log_n == 0 {
        return Ok(());
    }
    Ok(())
}

fn bit_reverse_permute(values: &mut [Fp]) {
    let n = values.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            values.swap(i, j);
        }
    }
}
