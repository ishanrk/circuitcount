use circuitcount::cnf::cnf::{Cnf, Lit};
use circuitcount::count::bounded::projected_count_bounded;

#[test]
fn bounded_count_on_or_clause() {
    let mut cnf = Cnf::new(2);
    cnf.add_clause(vec![Lit::new(1, true), Lit::new(2, true)]);

    let full = projected_count_bounded(&cnf, &[1, 2], 10).expect("count");
    assert!(!full.hit_cap);
    assert_eq!(full.count, 3);

    let capped = projected_count_bounded(&cnf, &[1, 2], 2).expect("count");
    assert!(capped.hit_cap);
    assert_eq!(capped.count, 3);
}
