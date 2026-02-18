use circuitcount::circuit::bench::parse_bench_str;
use circuitcount::count::hash_count::{CountMode, count_output};

#[test]
fn exact_count_and_or_circuit() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
OUTPUT(out)
n1 = AND(a,b)
out = OR(n1,c)
";
    let aig = parse_bench_str(src).expect("parse");
    let rep = count_output(&aig, 0, 0, 1000, 1, 0.35).expect("count");
    assert_eq!(rep.mode, CountMode::Exact);
    assert_eq!(rep.result, 5);
}

#[test]
fn exact_count_xor_circuit() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
out = XOR(a,b)
";
    let aig = parse_bench_str(src).expect("parse");
    let rep = count_output(&aig, 0, 0, 1000, 1, 0.35).expect("count");
    assert_eq!(rep.mode, CountMode::Exact);
    assert_eq!(rep.result, 2);
}

#[test]
fn hash_mode_smoke_range() {
    let src = "\
INPUT(a)
INPUT(b)
INPUT(c)
OUTPUT(out)
n1 = AND(a,b)
out = OR(n1,c)
";
    let aig = parse_bench_str(src).expect("parse");
    let rep = count_output(&aig, 0, 0, 2, 3, 0.35).expect("count");
    assert_eq!(rep.mode, CountMode::Hash);
    assert!((1..=8).contains(&rep.result));
}
