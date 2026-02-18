use circuitcount::circuit::bench::parse_bench_str;

#[test]
fn duplicate_and_becomes_const_zero() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
n1 = AND(a,b)
n2 = AND(a,b)
out = XOR(n1,n2)
";
    let original = parse_bench_str(src).expect("parse");
    let simplified = original.simplify_output(0).expect("simplify");

    assert_eq!(simplified.num_ands(), 0);
    assert_eq!(simplified.num_inputs(), 0);

    let mut true_count = 0usize;
    for a in [false, true] {
        for b in [false, true] {
            let out = original.eval(&[a, b])[0];
            if out {
                true_count += 1;
            }
            let simp = simplified.eval(&[])[0];
            assert_eq!(out, simp);
        }
    }
    assert_eq!(true_count, 0);
}

#[test]
fn commutative_hashing_merges_ands() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
n1 = AND(a,b)
n2 = AND(b,a)
out = XOR(n1,n2)
";
    let original = parse_bench_str(src).expect("parse");
    let simplified = original.simplify_output(0).expect("simplify");

    assert_eq!(simplified.num_ands(), 0);
    assert_eq!(simplified.num_inputs(), 0);

    for a in [false, true] {
        for b in [false, true] {
            let out = original.eval(&[a, b])[0];
            let simp = simplified.eval(&[])[0];
            assert_eq!(out, simp);
        }
    }
}

#[test]
fn constant_folding_through_or_lowering() {
    let src = "\
INPUT(a)
OUTPUT(out)
n1 = AND(a, 1)
n2 = AND(a, 0)
out = OR(n1, n2)
";
    let original = parse_bench_str(src).expect("parse");
    let simplified = original.simplify_output(0).expect("simplify");

    assert!(simplified.num_ands() <= 1);

    for a in [false, true] {
        let out = original.eval(&[a])[0];
        let simp = simplified.eval(&[a])[0];
        assert_eq!(out, a);
        assert_eq!(simp, a);
    }
}
