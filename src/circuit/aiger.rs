use std::io::BufRead;

use anyhow::{Context, Result, bail};

use super::aig::{Aig, AigLit, AndGate};

pub fn parse_aag_str(s: &str) -> Result<Aig> {
    parse_aag_reader(std::io::Cursor::new(s.as_bytes()))
}

pub fn parse_aag_reader<R: BufRead>(r: R) -> Result<Aig> {
    let lines = r
        .lines()
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read aag input")?;

    if lines.is_empty() {
        bail!("empty input");
    }

    let header_line = lines[0].trim();
    let header_parts = header_line.split_whitespace().collect::<Vec<_>>();
    if header_parts.len() != 6 || header_parts[0] != "aag" {
        bail!("invalid header, expected: aag M I L O A");
    }

    let max_id = parse_u32_token(header_parts[1], "M")?;
    let num_inputs = parse_u32_token(header_parts[2], "I")? as usize;
    let num_latches = parse_u32_token(header_parts[3], "L")?;
    let num_outputs = parse_u32_token(header_parts[4], "O")? as usize;
    let num_ands = parse_u32_token(header_parts[5], "A")? as usize;

    if num_latches != 0 {
        bail!("only combinational aag is supported (L must be 0)");
    }

    let needed = 1 + num_inputs + num_outputs + num_ands;
    if lines.len() < needed {
        bail!(
            "truncated aag: expected at least {} lines, found {}",
            needed,
            lines.len()
        );
    }

    let mut cursor = 1usize;
    let mut inputs = Vec::with_capacity(num_inputs);
    let mut outputs = Vec::with_capacity(num_outputs);
    let mut ands = Vec::with_capacity(num_ands);
    let mut max_ref_id = 0u32;

    for i in 0..num_inputs {
        let lit = parse_single_lit(lines[cursor].trim(), cursor + 1, "input")?;
        cursor += 1;

        if lit == 0 || lit % 2 == 1 {
            bail!("invalid input literal on line {}: must be even and nonzero", cursor);
        }
        if lit > 2 * max_id {
            bail!("input literal on line {} exceeds 2*M", cursor);
        }

        let id = lit / 2;
        max_ref_id = max_ref_id.max(id);
        inputs.push(id);

        if id == 0 {
            bail!("invalid input id 0 at input {}", i);
        }
    }

    for _ in 0..num_outputs {
        let lit = parse_single_lit(lines[cursor].trim(), cursor + 1, "output")?;
        cursor += 1;
        let out = lit_to_aig_lit(lit);
        max_ref_id = max_ref_id.max(out.id);
        outputs.push(out);
    }

    for _ in 0..num_ands {
        let line_no = cursor + 1;
        let parts = lines[cursor].split_whitespace().collect::<Vec<_>>();
        cursor += 1;

        if parts.len() != 3 {
            bail!("invalid and line {}: expected three literals", line_no);
        }

        let lhs = parse_u32_token(parts[0], "and lhs")?;
        let rhs0 = parse_u32_token(parts[1], "and rhs0")?;
        let rhs1 = parse_u32_token(parts[2], "and rhs1")?;

        if lhs == 0 || lhs % 2 == 1 {
            bail!("invalid and lhs on line {}: must be even and nonzero", line_no);
        }

        let id = lhs / 2;
        let a = lit_to_aig_lit(rhs0);
        let b = lit_to_aig_lit(rhs1);

        if id <= a.id || id <= b.id {
            bail!(
                "and gate on line {} violates topo order: id {} depends on {} and {}",
                line_no,
                id,
                a.id,
                b.id
            );
        }

        max_ref_id = max_ref_id.max(id).max(a.id).max(b.id);
        ands.push(AndGate { id, a, b });
    }

    if max_ref_id > max_id {
        bail!("header M={} is smaller than referenced id {}", max_id, max_ref_id);
    }

    Ok(Aig {
        max_id,
        inputs,
        outputs,
        ands,
    })
}

fn parse_single_lit(line: &str, line_no: usize, kind: &str) -> Result<u32> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 1 {
        bail!(
            "invalid {} line {}: expected one literal, got {} fields",
            kind,
            line_no,
            parts.len()
        );
    }
    parse_u32_token(parts[0], kind)
}

fn parse_u32_token(token: &str, what: &str) -> Result<u32> {
    token
        .parse::<u32>()
        .with_context(|| format!("invalid {} value: {}", what, token))
}

fn lit_to_aig_lit(lit: u32) -> AigLit {
    AigLit {
        id: lit / 2,
        neg: lit % 2 == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_aag_str;

    #[test]
    fn parse_tiny_aag() {
        let src = "\
aag 2 1 0 1 1
2
4
4 2 2
";
        let aig = match parse_aag_str(src) {
            Ok(v) => v,
            Err(e) => panic!("parse failed: {e}"),
        };

        assert_eq!(aig.max_id, 2);
        assert_eq!(aig.num_inputs(), 1);
        assert_eq!(aig.num_ands(), 1);
        assert_eq!(aig.input_ids(), &[1]);
        assert_eq!(aig.outputs().len(), 1);
        assert_eq!(aig.outputs()[0].id, 2);
        assert!(!aig.outputs()[0].neg);
        assert_eq!(aig.ands[0].id, 2);
        assert_eq!(aig.ands[0].a.id, 1);
        assert_eq!(aig.ands[0].b.id, 1);
    }
}
