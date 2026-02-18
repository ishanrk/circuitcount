use std::collections::{BTreeSet, VecDeque};
use std::io::BufRead;

use anyhow::{Context, Result, bail};

use super::aig::{Aig, AigBuilder, AigLit};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    And,
    Or,
    Not,
    Xor,
    Xnor,
    Buf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Assign {
    lhs: String,
    op: Op,
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchNetlist {
    inputs: Vec<String>,
    outputs: Vec<String>,
    assigns: Vec<Assign>,
}

pub fn parse_bench_str(s: &str) -> Result<Aig> {
    parse_bench_reader(std::io::Cursor::new(s.as_bytes()))
}

pub fn parse_bench_reader<R: BufRead>(r: R) -> Result<Aig> {
    let netlist = parse_netlist(r)?;
    lower_netlist(&netlist)
}

fn parse_netlist<R: BufRead>(r: R) -> Result<BenchNetlist> {
    let mut inputs = Vec::<String>::new();
    let mut outputs = Vec::<String>::new();
    let mut assigns = Vec::<Assign>::new();
    let mut defined_inputs = BTreeSet::<String>::new();
    let mut defined_assigns = BTreeSet::<String>::new();

    for (idx, line) in r.lines().enumerate() {
        let line_no = idx + 1;
        let line = line.context("failed to read bench line")?;
        let clean = strip_comment(&line).trim().to_owned();
        if clean.is_empty() {
            continue;
        }

        if clean.starts_with("INPUT(") {
            let name = parse_decl_name(&clean, "INPUT")
                .with_context(|| format!("line {}: invalid INPUT", line_no))?;
            if !is_valid_name(&name) {
                bail!("line {}: invalid name '{}'", line_no, name);
            }
            if defined_inputs.contains(&name) || defined_assigns.contains(&name) {
                bail!("line {}: redefinition of '{}'", line_no, name);
            }
            defined_inputs.insert(name.clone());
            inputs.push(name);
            continue;
        }

        if clean.starts_with("OUTPUT(") {
            let name = parse_decl_name(&clean, "OUTPUT")
                .with_context(|| format!("line {}: invalid OUTPUT", line_no))?;
            if !is_valid_name(&name) {
                bail!("line {}: invalid output name '{}'", line_no, name);
            }
            outputs.push(name);
            continue;
        }

        if has_seq_keyword(&clean) {
            bail!("line {}: sequential constructs are not supported", line_no);
        }

        let assign = parse_assign(&clean).with_context(|| format!("line {}: invalid assign", line_no))?;
        if !is_valid_name(&assign.lhs) {
            bail!("line {}: invalid lhs '{}'", line_no, assign.lhs);
        }
        if defined_inputs.contains(&assign.lhs) || defined_assigns.contains(&assign.lhs) {
            bail!("line {}: redefinition of '{}'", line_no, assign.lhs);
        }
        for arg in &assign.args {
            if arg != "0" && arg != "1" && !is_valid_name(arg) {
                bail!("line {}: invalid arg '{}'", line_no, arg);
            }
        }
        defined_assigns.insert(assign.lhs.clone());
        assigns.push(assign);
    }

    Ok(BenchNetlist {
        inputs,
        outputs,
        assigns,
    })
}

fn lower_netlist(netlist: &BenchNetlist) -> Result<Aig> {
    let mut builder = AigBuilder::new();
    for name in &netlist.inputs {
        builder.input(name)?;
    }

    let order = topo_order(netlist)?;
    for idx in order {
        let asn = &netlist.assigns[idx];
        let rhs = eval_assign_rhs(&mut builder, asn)?;
        builder.set(&asn.lhs, rhs)?;
    }

    let mut out_lits = Vec::<(String, AigLit)>::with_capacity(netlist.outputs.len());
    for name in &netlist.outputs {
        let lit = builder
            .get(name)
            .with_context(|| format!("output references undefined signal '{}'", name))?;
        out_lits.push((name.clone(), lit));
    }

    Ok(builder.finish(out_lits))
}

fn topo_order(netlist: &BenchNetlist) -> Result<Vec<usize>> {
    let mut lhs_to_idx = std::collections::HashMap::<&str, usize>::new();
    for (idx, asn) in netlist.assigns.iter().enumerate() {
        lhs_to_idx.insert(asn.lhs.as_str(), idx);
    }

    let input_set = netlist.inputs.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut indeg = vec![0usize; netlist.assigns.len()];
    let mut uses = vec![Vec::<usize>::new(); netlist.assigns.len()];

    for (idx, asn) in netlist.assigns.iter().enumerate() {
        for arg in &asn.args {
            if arg == "0" || arg == "1" || input_set.contains(arg.as_str()) {
                continue;
            }
            if let Some(&dep_idx) = lhs_to_idx.get(arg.as_str()) {
                indeg[idx] += 1;
                uses[dep_idx].push(idx);
            } else {
                bail!(
                    "undefined signal '{}' used in assignment '{}'",
                    arg,
                    asn.lhs
                );
            }
        }
    }

    for out in &netlist.outputs {
        if !input_set.contains(out.as_str()) && !lhs_to_idx.contains_key(out.as_str()) {
            bail!("output references undefined signal '{}'", out);
        }
    }

    let mut queue = VecDeque::<usize>::new();
    for (idx, &d) in indeg.iter().enumerate() {
        if d == 0 {
            queue.push_back(idx);
        }
    }

    let mut order = Vec::<usize>::with_capacity(netlist.assigns.len());
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &next in &uses[idx] {
            indeg[next] -= 1;
            if indeg[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    if order.len() != netlist.assigns.len() {
        bail!("cycle detected in assignments");
    }
    Ok(order)
}

fn eval_assign_rhs(builder: &mut AigBuilder, asn: &Assign) -> Result<AigLit> {
    let args = asn
        .args
        .iter()
        .map(|a| resolve_arg(builder, a))
        .collect::<Result<Vec<_>>>()?;

    let out = match asn.op {
        Op::And => builder.and(args[0], args[1]),
        Op::Or => builder.or(args[0], args[1]),
        Op::Not => builder.not(args[0]),
        Op::Xor => builder.xor(args[0], args[1]),
        Op::Xnor => builder.xnor(args[0], args[1]),
        Op::Buf => args[0],
    };
    Ok(out)
}

fn resolve_arg(builder: &AigBuilder, arg: &str) -> Result<AigLit> {
    if arg == "0" {
        return Ok(AigLit { id: 0, neg: false });
    }
    if arg == "1" {
        return Ok(AigLit { id: 0, neg: true });
    }
    builder.get(arg)
}

fn parse_assign(s: &str) -> Result<Assign> {
    let (lhs_raw, rhs_raw) = s
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("expected '=' in assignment"))?;
    let lhs = lhs_raw.trim().to_owned();
    let (op, args) = parse_call(rhs_raw.trim())?;

    let expected = match op {
        Op::And | Op::Or | Op::Xor | Op::Xnor => 2,
        Op::Not | Op::Buf => 1,
    };
    if args.len() != expected {
        bail!(
            "wrong arity for op, expected {} args but got {}",
            expected,
            args.len()
        );
    }

    Ok(Assign { lhs, op, args })
}

fn parse_call(s: &str) -> Result<(Op, Vec<String>)> {
    let open = s
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("missing '(' in expression"))?;
    let close = s
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("missing ')' in expression"))?;
    if close < open {
        bail!("malformed expression");
    }
    let name = s[..open].trim();
    let inside = s[open + 1..close].trim();
    if !s[close + 1..].trim().is_empty() {
        bail!("trailing tokens after ')'");
    }

    let op = match name {
        "AND" => Op::And,
        "OR" => Op::Or,
        "NOT" => Op::Not,
        "XOR" => Op::Xor,
        "XNOR" => Op::Xnor,
        "BUF" => Op::Buf,
        _ => bail!("unsupported op '{}'", name),
    };

    let args = if inside.is_empty() {
        Vec::new()
    } else {
        inside
            .split(',')
            .map(|p| p.trim().to_owned())
            .collect::<Vec<_>>()
    };
    if args.iter().any(|a| a.is_empty()) {
        bail!("empty argument in op call");
    }
    Ok((op, args))
}

fn parse_decl_name(s: &str, kind: &str) -> Result<String> {
    let open = s
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("missing '(' in {}", kind))?;
    let close = s
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("missing ')' in {}", kind))?;
    if s[..open].trim() != kind {
        bail!("invalid {} syntax", kind);
    }
    if !s[close + 1..].trim().is_empty() {
        bail!("trailing text after {}", kind);
    }
    Ok(s[open + 1..close].trim().to_owned())
}

fn strip_comment(s: &str) -> &str {
    if let Some(idx) = s.find('#') {
        &s[..idx]
    } else {
        s
    }
}

fn has_seq_keyword(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    upper.contains("LATCH") || upper.contains("DFF") || upper.contains("REG")
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::{parse_assign, parse_bench_str};

    #[test]
    fn parse_line_with_spaces_and_comment() {
        let src = "\
        INPUT(a) # input
        INPUT(b)
        OUTPUT(out)
        out = OR( a , b ) # logic
        ";
        let aig = match parse_bench_str(src) {
            Ok(v) => v,
            Err(e) => panic!("parse failed: {e}"),
        };
        assert_eq!(aig.num_inputs(), 2);
        assert_eq!(aig.outputs().len(), 1);
    }

    #[test]
    fn parse_assign_commas() {
        let asn = match parse_assign("x = XOR(a, b)") {
            Ok(v) => v,
            Err(e) => panic!("parse failed: {e}"),
        };
        assert_eq!(asn.lhs, "x");
        assert_eq!(asn.args, vec!["a".to_string(), "b".to_string()]);
    }
}
