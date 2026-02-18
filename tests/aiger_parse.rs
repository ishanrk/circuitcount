use circuitcount::circuit::aiger::parse_aag_str;

#[test]
fn parse_and_eval_sanity_or_via_demorgan() {
    let src = "\
aag 5 3 0 1 2
2
4
6
11
8 2 4
10 9 7
";

    let aig = match parse_aag_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };

    let mut count_true = 0usize;
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                let out = aig.eval(&[a, b, c]);
                if out[0] {
                    count_true += 1;
                }
            }
        }
    }
    assert_eq!(count_true, 5);
}

#[test]
fn reject_latches() {
    let src = "aag 1 0 1 0 0\n";
    let err = match parse_aag_str(src) {
        Ok(_) => panic!("expected parser error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("L must be 0"));
}

#[test]
fn reject_non_topological_ands() {
    let src = "\
aag 3 1 0 1 2
2
6
4 6 2
6 2 2
";
    let err = match parse_aag_str(src) {
        Ok(_) => panic!("expected parser error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("topo order"));
}
