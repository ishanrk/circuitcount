use circuitcount::circuit::aiger::parse_aag_str;
use circuitcount::circuit::bench::parse_bench_str;

#[test]
fn drops_unused_input_and_logic() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
INPUT(d)
OUTPUT(out)
n1 = AND(a,b)
out = OR(n1,c)
junk1 = AND(d,a)
junk2 = XOR(junk1,d)
";
    let original = match parse_bench_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };
    let reduced = match original.restrict_to_output(0) {
        Ok(v) => v,
        Err(e) => panic!("restrict failed: {e}"),
    };

    assert_eq!(reduced.num_inputs(), 3);
    assert!(reduced.num_ands() < original.num_ands());

    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                for d in [false, true] {
                    let orig = original.eval(&[a, b, c, d])[0];
                    let red = reduced.eval(&[a, b, c])[0];
                    assert_eq!(orig, red);
                }
            }
        }
    }
}

#[test]
fn coi_on_aag_demorgan_example() {
    let src = "\
aag 5 3 0 1 2
2
4
6
11
8 2 4
10 9 7
";
    let original = match parse_aag_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };
    let coi = match original.coi(0) {
        Ok(v) => v,
        Err(e) => panic!("coi failed: {e}"),
    };
    assert_eq!(coi.input_ids().len(), original.num_inputs());

    let reduced = match original.restrict_to_output(0) {
        Ok(v) => v,
        Err(e) => panic!("restrict failed: {e}"),
    };
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                let orig = original.eval(&[a, b, c])[0];
                let red = reduced.eval(&[a, b, c])[0];
                assert_eq!(orig, red);
            }
        }
    }
}

#[test]
fn output_selection_matters() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
INPUT(d)
OUTPUT(out1)
OUTPUT(out2)
out1 = AND(a,b)
out2 = AND(c,d)
";
    let original = match parse_bench_str(src) {
        Ok(v) => v,
        Err(e) => panic!("parse failed: {e}"),
    };

    let coi1 = match original.coi(0) {
        Ok(v) => v,
        Err(e) => panic!("coi1 failed: {e}"),
    };
    let coi2 = match original.coi(1) {
        Ok(v) => v,
        Err(e) => panic!("coi2 failed: {e}"),
    };
    assert_eq!(coi1.input_ids(), &[1, 2]);
    assert_eq!(coi2.input_ids(), &[3, 4]);

    let reduced1 = match original.restrict_to_output(0) {
        Ok(v) => v,
        Err(e) => panic!("restrict1 failed: {e}"),
    };
    let reduced2 = match original.restrict_to_output(1) {
        Ok(v) => v,
        Err(e) => panic!("restrict2 failed: {e}"),
    };
    assert_eq!(reduced1.num_inputs(), 2);
    assert_eq!(reduced2.num_inputs(), 2);

    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                for d in [false, true] {
                    let o1 = original.eval(&[a, b, c, d])[0];
                    let o2 = original.eval(&[a, b, c, d])[1];
                    let r1 = reduced1.eval(&[a, b])[0];
                    let r2 = reduced2.eval(&[c, d])[0];
                    assert_eq!(o1, r1);
                    assert_eq!(o2, r2);
                }
            }
        }
    }
}
