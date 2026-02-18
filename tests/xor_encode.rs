use circuitcount::cnf::cnf::Cnf;
use circuitcount::count::bounded::projected_count_bounded;
use circuitcount::xor::encode::{XorBlockMode, XorConstraint, append_xor_block};

#[test]
fn xor2_eq0_has_two_solutions() {
    let mut cnf = Cnf::new(2);
    append_xor_block(
        &mut cnf,
        &[XorConstraint {
            vars: vec![1, 2],
            rhs: false,
        }],
        XorBlockMode::Plain,
    )
    .expect("encode");
    let res = projected_count_bounded(&cnf, &[1, 2], 100).expect("count");
    assert!(!res.hit_cap);
    assert_eq!(res.count, 2);
}

#[test]
fn xor2_eq1_has_two_solutions() {
    let mut cnf = Cnf::new(2);
    append_xor_block(
        &mut cnf,
        &[XorConstraint {
            vars: vec![1, 2],
            rhs: true,
        }],
        XorBlockMode::Plain,
    )
    .expect("encode");
    let res = projected_count_bounded(&cnf, &[1, 2], 100).expect("count");
    assert!(!res.hit_cap);
    assert_eq!(res.count, 2);
}

#[test]
fn xor3_eq1_has_four_solutions() {
    let mut cnf = Cnf::new(3);
    append_xor_block(
        &mut cnf,
        &[XorConstraint {
            vars: vec![1, 2, 3],
            rhs: true,
        }],
        XorBlockMode::Plain,
    )
    .expect("encode");
    let res = projected_count_bounded(&cnf, &[1, 2, 3], 100).expect("count");
    assert!(!res.hit_cap);
    assert_eq!(res.count, 4);
}
