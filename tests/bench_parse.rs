use circuitcount::circuit::bench::parse_bench_str;

#[test]
fn parse_and_eval_or_sanity() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
OUTPUT(out)
n1 = AND(a,b)
out = OR(n1,c)
";
    let aig = match parse_bench_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };

    let mut count_true = 0usize;
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                if aig.eval(&[a, b, c])[0] {
                    count_true += 1;
                }
            }
        }
    }
    assert_eq!(count_true, 5);
}

#[test]
fn xor_sanity() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
out = XOR(a,b)
";
    let aig = match parse_bench_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };

    let mut count_true = 0usize;
    for a in [false, true] {
        for b in [false, true] {
            if aig.eval(&[a, b])[0] {
                count_true += 1;
            }
        }
    }
    assert_eq!(count_true, 2);
}

#[test]
fn forward_reference() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
out = OR(n1,a)
n1 = AND(a,b)
";
    let aig = match parse_bench_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };

    let mut count_true = 0usize;
    for a in [false, true] {
        for b in [false, true] {
            if aig.eval(&[a, b])[0] {
                count_true += 1;
            }
        }
    }
    assert_eq!(count_true, 2);
}

#[test]
fn reject_cycles() {
    let src = "\
INPUT(a)
OUTPUT(out)
n1 = NOT(out)
out = NOT(n1)
";
    let err = match parse_bench_str(src) {
        Ok(_) => panic!("expected cycle error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("cycle"));
}
