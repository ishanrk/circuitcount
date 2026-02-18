use anyhow::{Result, bail};
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AigLit {
    pub id: u32,
    pub neg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndGate {
    pub id: u32,
    pub a: AigLit,
    pub b: AigLit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aig {
    pub max_id: u32,
    pub inputs: Vec<u32>,
    pub outputs: Vec<AigLit>,
    pub ands: Vec<AndGate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoiInfo {
    input_ids: Vec<u32>,
    ands_in_cone: usize,
    in_cone: Vec<bool>,
}

impl Aig {
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    pub fn num_ands(&self) -> usize {
        self.ands.len()
    }

    pub fn input_ids(&self) -> &[u32] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[AigLit] {
        &self.outputs
    }

    pub fn eval(&self, input_bits: &[bool]) -> Vec<bool> {
        assert_eq!(
            input_bits.len(),
            self.inputs.len(),
            "input_bits length must match number of inputs"
        );

        let mut values = vec![false; self.max_id as usize + 1];

        for (idx, &id) in self.inputs.iter().enumerate() {
            values[id as usize] = input_bits[idx];
        }

        for gate in &self.ands {
            let av = lit_value(gate.a, &values);
            let bv = lit_value(gate.b, &values);
            values[gate.id as usize] = av & bv;
        }

        self.outputs
            .iter()
            .map(|&lit| lit_value(lit, &values))
            .collect()
    }

    pub fn coi(&self, output_idx: usize) -> Result<CoiInfo> {
        if output_idx >= self.outputs.len() {
            bail!(
                "output index {} out of range (outputs={})",
                output_idx,
                self.outputs.len()
            );
        }

        let mut input_mask = vec![false; self.max_id as usize + 1];
        for &id in &self.inputs {
            if id > self.max_id {
                bail!("input id {} exceeds max_id {}", id, self.max_id);
            }
            input_mask[id as usize] = true;
        }

        let mut gate_fanins = vec![None::<(AigLit, AigLit)>; self.max_id as usize + 1];
        for gate in &self.ands {
            if gate.id == 0 || gate.id > self.max_id {
                bail!("and gate id {} is invalid for max_id {}", gate.id, self.max_id);
            }
            if gate.a.id > self.max_id || gate.b.id > self.max_id {
                bail!(
                    "and gate {} has fanin outside max_id {}",
                    gate.id,
                    self.max_id
                );
            }
            gate_fanins[gate.id as usize] = Some((gate.a, gate.b));
        }

        let mut in_cone = vec![false; self.max_id as usize + 1];
        let mut stack = vec![self.outputs[output_idx].id];
        while let Some(id) = stack.pop() {
            if id == 0 {
                continue;
            }
            if id > self.max_id {
                bail!("output references id {} beyond max_id {}", id, self.max_id);
            }
            let pos = id as usize;
            if in_cone[pos] {
                continue;
            }
            in_cone[pos] = true;

            if let Some((a, b)) = gate_fanins[pos] {
                if a.id > self.max_id || b.id > self.max_id {
                    bail!("and gate {} has invalid fanin id", id);
                }
                stack.push(a.id);
                stack.push(b.id);
            } else if !input_mask[pos] {
                bail!("node {} is referenced but not defined as input or and", id);
            }
        }

        let input_ids = self
            .inputs
            .iter()
            .copied()
            .filter(|&id| id <= self.max_id && in_cone[id as usize])
            .collect::<Vec<_>>();
        let ands_in_cone = self
            .ands
            .iter()
            .filter(|g| g.id <= self.max_id && in_cone[g.id as usize])
            .count();

        Ok(CoiInfo {
            input_ids,
            ands_in_cone,
            in_cone,
        })
    }

    pub fn restrict_to_output(&self, output_idx: usize) -> Result<Aig> {
        let coi = self.coi(output_idx)?;
        let mut remap = IndexMap::<u32, u32>::new();
        remap.insert(0, 0);

        let mut next_id = 1u32;
        let mut new_inputs = Vec::<u32>::new();
        for &old_id in &self.inputs {
            if old_id <= self.max_id && coi.contains_node(old_id) {
                remap.insert(old_id, next_id);
                new_inputs.push(next_id);
                next_id += 1;
            }
        }

        let mut new_ands = Vec::<AndGate>::new();
        for gate in &self.ands {
            if !coi.contains_node(gate.id) {
                continue;
            }
            if gate.a.id > self.max_id || gate.b.id > self.max_id {
                bail!(
                    "and gate {} has fanin outside max_id {}",
                    gate.id,
                    self.max_id
                );
            }
            let a = rewrite_lit(gate.a, &remap)?;
            let b = rewrite_lit(gate.b, &remap)?;
            let new_id = next_id;
            next_id += 1;
            remap.insert(gate.id, new_id);
            new_ands.push(AndGate { id: new_id, a, b });
        }

        let old_out = self.outputs[output_idx];
        let new_out = rewrite_lit(old_out, &remap)?;
        let new_max = next_id.saturating_sub(1);

        Ok(Aig {
            max_id: new_max,
            inputs: new_inputs,
            outputs: vec![new_out],
            ands: new_ands,
        })
    }

    pub fn simplify_output(&self, output_idx: usize) -> Result<Aig> {
        let reduced = self.restrict_to_output(output_idx)?;
        let rewritten = reduced.rewrite_single_output()?;
        rewritten.restrict_to_output(0)
    }

    fn rewrite_single_output(&self) -> Result<Aig> {
        if self.outputs.len() != 1 {
            bail!(
                "rewrite_single_output expects one output, got {}",
                self.outputs.len()
            );
        }

        let mut mapped = vec![None::<AigLit>; self.max_id as usize + 1];
        mapped[0] = Some(false_lit());

        let mut next_id = 1u32;
        let mut new_inputs = Vec::<u32>::new();
        let mut new_ands = Vec::<AndGate>::new();
        let mut hash = HashMap::<AndKey, AigLit>::new();

        for &id in &self.inputs {
            if id == 0 || id > self.max_id {
                bail!("input id {} is invalid for max_id {}", id, self.max_id);
            }
            let lit = AigLit { id: next_id, neg: false };
            next_id += 1;
            mapped[id as usize] = Some(lit);
            new_inputs.push(lit.id);
        }

        for gate in &self.ands {
            if gate.id == 0 || gate.id > self.max_id {
                bail!("and gate id {} is invalid for max_id {}", gate.id, self.max_id);
            }
            if gate.a.id > self.max_id || gate.b.id > self.max_id {
                bail!(
                    "and gate {} has fanin outside max_id {}",
                    gate.id,
                    self.max_id
                );
            }

            let a_base = mapped[gate.a.id as usize]
                .ok_or_else(|| anyhow::anyhow!("fanin {} not mapped yet", gate.a.id))?;
            let b_base = mapped[gate.b.id as usize]
                .ok_or_else(|| anyhow::anyhow!("fanin {} not mapped yet", gate.b.id))?;
            let a = apply_neg(a_base, gate.a.neg);
            let b = apply_neg(b_base, gate.b.neg);

            // fold basic and identities early
            let out = if let Some(lit) = fold_and(a, b) {
                lit
            } else {
                // normalize key so and is commutative
                let key = AndKey::new(a, b);
                if let Some(&lit) = hash.get(&key) {
                    lit
                } else {
                    let lit = AigLit {
                        id: next_id,
                        neg: false,
                    };
                    next_id += 1;
                    new_ands.push(AndGate {
                        id: lit.id,
                        a: key.a,
                        b: key.b,
                    });
                    hash.insert(key, lit);
                    lit
                }
            };
            mapped[gate.id as usize] = Some(out);
        }

        let out_old = self.outputs[0];
        if out_old.id > self.max_id {
            bail!(
                "output id {} is invalid for max_id {}",
                out_old.id,
                self.max_id
            );
        }
        let out_base = mapped[out_old.id as usize]
            .ok_or_else(|| anyhow::anyhow!("output id {} not mapped", out_old.id))?;
        let out_lit = apply_neg(out_base, out_old.neg);

        Ok(Aig {
            max_id: next_id.saturating_sub(1),
            inputs: new_inputs,
            outputs: vec![out_lit],
            ands: new_ands,
        })
    }
}

fn lit_value(lit: AigLit, values: &[bool]) -> bool {
    let base = if lit.id == 0 {
        false
    } else {
        values[lit.id as usize]
    };
    if lit.neg { !base } else { base }
}

impl CoiInfo {
    pub fn input_ids(&self) -> &[u32] {
        &self.input_ids
    }

    pub fn ands_in_cone(&self) -> usize {
        self.ands_in_cone
    }

    pub fn contains_node(&self, id: u32) -> bool {
        (id as usize) < self.in_cone.len() && self.in_cone[id as usize]
    }
}

fn rewrite_lit(lit: AigLit, remap: &IndexMap<u32, u32>) -> Result<AigLit> {
    let id = remap
        .get(&lit.id)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing remap for node {}", lit.id))?;
    Ok(AigLit { id, neg: lit.neg })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AndKey {
    a: AigLit,
    b: AigLit,
}

impl AndKey {
    fn new(a: AigLit, b: AigLit) -> Self {
        if lit_order(a, b) {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

fn false_lit() -> AigLit {
    AigLit { id: 0, neg: false }
}

fn is_false(x: AigLit) -> bool {
    x.id == 0 && !x.neg
}

fn is_true(x: AigLit) -> bool {
    x.id == 0 && x.neg
}

fn apply_neg(x: AigLit, flip: bool) -> AigLit {
    if flip {
        AigLit {
            id: x.id,
            neg: !x.neg,
        }
    } else {
        x
    }
}

fn fold_and(a: AigLit, b: AigLit) -> Option<AigLit> {
    if is_false(a) || is_false(b) {
        return Some(false_lit());
    }
    if is_true(a) {
        return Some(b);
    }
    if is_true(b) {
        return Some(a);
    }
    if a == b {
        return Some(a);
    }
    if a.id == b.id && a.neg != b.neg {
        return Some(false_lit());
    }
    None
}

fn lit_order(a: AigLit, b: AigLit) -> bool {
    if a.id != b.id {
        a.id < b.id
    } else {
        (a.neg as u8) <= (b.neg as u8)
    }
}

#[derive(Debug, Default)]
pub struct AigBuilder {
    next_id: u32,
    names: IndexMap<String, AigLit>,
    inputs: Vec<u32>,
    ands: Vec<AndGate>,
}

impl AigBuilder {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            names: IndexMap::new(),
            inputs: Vec::new(),
            ands: Vec::new(),
        }
    }

    pub fn input(&mut self, name: &str) -> Result<AigLit> {
        if self.names.contains_key(name) {
            bail!("name already defined: {}", name);
        }
        let id = self.alloc_id();
        let lit = AigLit { id, neg: false };
        self.names.insert(name.to_owned(), lit);
        self.inputs.push(id);
        Ok(lit)
    }

    pub fn get(&self, name: &str) -> Result<AigLit> {
        self.names
            .get(name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("unknown signal: {}", name))
    }

    pub fn set(&mut self, name: &str, lit: AigLit) -> Result<()> {
        if self.names.contains_key(name) {
            bail!("name already defined: {}", name);
        }
        self.names.insert(name.to_owned(), lit);
        Ok(())
    }

    pub fn not(&self, x: AigLit) -> AigLit {
        AigLit {
            id: x.id,
            neg: !x.neg,
        }
    }

    pub fn and(&mut self, a: AigLit, b: AigLit) -> AigLit {
        let id = self.alloc_id();
        self.ands.push(AndGate { id, a, b });
        AigLit { id, neg: false }
    }

    pub fn or(&mut self, a: AigLit, b: AigLit) -> AigLit {
        let t = self.and(self.not(a), self.not(b));
        self.not(t)
    }

    pub fn xor(&mut self, a: AigLit, b: AigLit) -> AigLit {
        let l = self.and(a, self.not(b));
        let r = self.and(self.not(a), b);
        self.or(l, r)
    }

    pub fn xnor(&mut self, a: AigLit, b: AigLit) -> AigLit {
        let x = self.xor(a, b);
        self.not(x)
    }

    pub fn finish(self, outputs: Vec<(String, AigLit)>) -> Aig {
        let max_id = self.next_id.saturating_sub(1);
        let output_lits = outputs.into_iter().map(|(_, lit)| lit).collect::<Vec<_>>();
        Aig {
            max_id,
            inputs: self.inputs,
            outputs: output_lits,
            ands: self.ands,
        }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}
