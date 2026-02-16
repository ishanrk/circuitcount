use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::field::Fp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleOpening {
    pub index: usize,
    pub value: u64,
    pub siblings: Vec<String>,
}

pub fn merkle_root_hex(values: &[Fp]) -> String {
    let levels = merkle_levels(values);
    to_hex(
        levels
            .last()
            .and_then(|x| x.first())
            .copied()
            .unwrap_or([0u8; 32]),
    )
}

pub fn merkle_open(values: &[Fp], mut index: usize) -> Result<MerkleOpening> {
    if index >= values.len() {
        bail!(
            "opening index {} out of bounds for {} leaves",
            index,
            values.len()
        );
    }
    let original_index = index;
    let value = values[index].as_u64();
    let levels = merkle_levels(values);
    let mut siblings = Vec::<String>::new();
    for level in &levels[..levels.len() - 1] {
        let sibling_hash = if index % 2 == 0 {
            if index + 1 < level.len() {
                level[index + 1]
            } else {
                level[index]
            }
        } else {
            level[index - 1]
        };
        siblings.push(to_hex(sibling_hash));
        index /= 2;
    }
    Ok(MerkleOpening {
        index: original_index,
        value,
        siblings,
    })
}

pub fn verify_opening(root_hex: &str, opening: &MerkleOpening) -> Result<bool> {
    let mut h = hash_leaf(Fp::new(opening.value));
    let mut idx = opening.index;
    for sib_hex in &opening.siblings {
        let sib = from_hex_32(sib_hex)?;
        h = if idx % 2 == 0 {
            hash_node(h, sib)
        } else {
            hash_node(sib, h)
        };
        idx /= 2;
    }
    Ok(to_hex(h) == root_hex)
}

fn merkle_levels(values: &[Fp]) -> Vec<Vec<[u8; 32]>> {
    let mut levels = Vec::<Vec<[u8; 32]>>::new();
    let mut current = values.iter().copied().map(hash_leaf).collect::<Vec<_>>();
    levels.push(current.clone());
    while current.len() > 1 {
        let mut next = Vec::<[u8; 32]>::with_capacity(current.len().div_ceil(2));
        let mut i = 0usize;
        while i < current.len() {
            let left = current[i];
            let right = if i + 1 < current.len() {
                current[i + 1]
            } else {
                current[i]
            };
            next.push(hash_node(left, right));
            i += 2;
        }
        current = next;
        levels.push(current.clone());
    }
    levels
}

fn hash_leaf(v: Fp) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"leaf");
    h.update(v.as_u64().to_le_bytes());
    h.finalize().into()
}

fn hash_node(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"node");
    h.update(left);
    h.update(right);
    h.finalize().into()
}

pub fn to_hex(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(hex_digit((b >> 4) & 0x0f));
        out.push(hex_digit(b & 0x0f));
    }
    out
}

fn from_hex_32(s: &str) -> Result<[u8; 32]> {
    if s.len() != 64 {
        bail!("hex string has length {}, expected 64", s.len());
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_value(s.as_bytes()[2 * i])?;
        let lo = hex_value(s.as_bytes()[2 * i + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_digit(x: u8) -> char {
    match x {
        0..=9 => (b'0' + x) as char,
        _ => (b'a' + (x - 10)) as char,
    }
}

fn hex_value(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(10 + c - b'a'),
        b'A'..=b'F' => Ok(10 + c - b'A'),
        _ => bail!("invalid hex char {}", c as char),
    }
}
