use circuitcount::circuit::aiger::parse_aag_str;
use circuitcount::circuit::bench::parse_bench_str;
use circuitcount::cnf::cnf::{Cnf, Lit};
use circuitcount::cnf::tseitin::encode_aig;
use circuitcount::sat::dpll::is_sat;

#[test]
fn bench_and_or_equiv_with_sat() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
OUTPUT(out)
n1 = AND(a,b)
out = OR(n1,c)
";
    let aig = parse_bench_str(src)
        .expect("parse bench")
        .simplify_output(0)
        .expect("simplify");
    let enc = encode_aig(&aig).expect("encode");

    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                let expected = aig.eval(&[a, b, c])[0];
                let mut cnf = enc.cnf.clone();
                add_input_units(&mut cnf, &enc.input_vars, &[a, b, c]);
                cnf.add_clause(vec![enc.output_lits[0]]);
                let sat = is_sat(&cnf);
                assert_eq!(sat, expected);
            }
        }
    }

    // pick one output-true assignment and force output false
    let mut bad = enc.cnf.clone();
    add_input_units(&mut bad, &enc.input_vars, &[true, true, false]);
    bad.add_clause(vec![enc.output_lits[0].neg()]);
    assert!(!is_sat(&bad));
}

#[test]
fn bench_xor_equiv_with_sat() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
out = XOR(a,b)
";
    let aig = parse_bench_str(src)
        .expect("parse bench")
        .simplify_output(0)
        .expect("simplify");
    let enc = encode_aig(&aig).expect("encode");

    for a in [false, true] {
        for b in [false, true] {
            let expected = aig.eval(&[a, b])[0];
            let mut cnf = enc.cnf.clone();
            add_input_units(&mut cnf, &enc.input_vars, &[a, b]);
            cnf.add_clause(vec![enc.output_lits[0]]);
            let sat = is_sat(&cnf);
            assert_eq!(sat, expected);
        }
    }
}

#[test]
fn aag_demorgan_equiv_with_sat() {
    let src = "\
aag 5 3 0 1 2
2
4
6
11
8 2 4
10 9 7
";
    let aig = parse_aag_str(src)
        .expect("parse aag")
        .simplify_output(0)
        .expect("simplify");
    let enc = encode_aig(&aig).expect("encode");

    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                let expected = aig.eval(&[a, b, c])[0];
                let mut cnf = enc.cnf.clone();
                add_input_units(&mut cnf, &enc.input_vars, &[a, b, c]);
                cnf.add_clause(vec![enc.output_lits[0]]);
                let sat = is_sat(&cnf);
                assert_eq!(sat, expected);
            }
        }
    }
}

fn add_input_units(cnf: &mut Cnf, input_vars: &[u32], bits: &[bool]) {
    assert_eq!(input_vars.len(), bits.len());
    for (&var, &bit) in input_vars.iter().zip(bits.iter()) {
        cnf.add_clause(vec![Lit::new(var, bit)]);
    }
}
