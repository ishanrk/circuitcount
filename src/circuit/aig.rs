use anyhow::{Result, bail};
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

fn lit_value(lit: AigLit, values: &[bool]) -> bool {
    let base = if lit.id == 0 {
        false
    } else {
        values[lit.id as usize]
    };
    if lit.neg { !base } else { base }
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
        self.not(self.xor(a, b))
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
