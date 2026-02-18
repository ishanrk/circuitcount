use circuitcount::circuit::bench::parse_bench_str;
use circuitcount::cnf::cnf::Lit;
use circuitcount::cnf::tseitin::encode_aig;
use circuitcount::count::bounded::projected_count_bounded_session;
use circuitcount::solver::dpll_backend::DpllSolverBackend;
use circuitcount::solver::scope::{add_scoped_clause, new_scope};
use circuitcount::solver::{IncrementalSolver, SolveResult};
use circuitcount::xor::encode::{XorConstraint, append_xor_constraint_to_solver};

#[test]
fn scoped_blocking_clauses_do_not_leak() {
    let mut solver = DpllSolverBackend::new();
    solver.new_var();
    solver.new_var();
    solver.add_clause(vec![Lit::new(1, true), Lit::new(2, true)]);

    let c1 = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[]).expect("count");
    let c2 = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[]).expect("count");
    assert_eq!(c1.count, 3);
    assert_eq!(c2.count, 3);
    assert!(!c1.hit_cap && !c2.hit_cap);
}

#[test]
fn scoped_input_fixing_flips_without_rebuild() {
    let src = "\
INPUT(a)
INPUT(b)
OUTPUT(out)
out = XOR(a,b)
";
    let aig = parse_bench_str(src)
        .expect("parse")
        .simplify_output(0)
        .expect("simplify");
    let mut enc = encode_aig(&aig).expect("encode");
    enc.cnf.add_clause(vec![enc.output_lits[0]]);

    let mut solver = DpllSolverBackend::new();
    for _ in 0..enc.cnf.num_vars {
        solver.new_var();
    }
    for clause in &enc.cnf.clauses {
        solver.add_clause(clause.clone());
    }

    let s0 = new_scope(&mut solver);
    add_scoped_clause(&mut solver, &s0, vec![Lit::new(enc.input_vars[0], false)]);
    add_scoped_clause(&mut solver, &s0, vec![Lit::new(enc.input_vars[1], false)]);
    let r0 = solver.solve(&[s0.act]);
    assert_eq!(r0, SolveResult::Unsat);

    let s1 = new_scope(&mut solver);
    add_scoped_clause(&mut solver, &s1, vec![Lit::new(enc.input_vars[0], false)]);
    add_scoped_clause(&mut solver, &s1, vec![Lit::new(enc.input_vars[1], true)]);
    let r1 = solver.solve(&[s1.act]);
    assert_eq!(r1, SolveResult::Sat);
}

#[test]
fn incremental_xor_activation_works() {
    let mut solver = DpllSolverBackend::new();
    solver.new_var(); // x
    solver.new_var(); // y

    let c1 = XorConstraint {
        vars: vec![1, 2],
        rhs: false,
    };
    let c2 = XorConstraint {
        vars: vec![1, 2],
        rhs: true,
    };

    let a1 = Lit::new(solver.new_var(), true);
    append_xor_constraint_to_solver(&mut solver, &c1, Some(a1)).expect("xor1");
    let a2 = Lit::new(solver.new_var(), true);
    append_xor_constraint_to_solver(&mut solver, &c2, Some(a2)).expect("xor2");

    let all = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[]).expect("count");
    assert_eq!(all.count, 4);

    let only1 = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[a1]).expect("count");
    assert_eq!(only1.count, 2);

    let only2 = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[a2]).expect("count");
    assert_eq!(only2.count, 2);

    let both = projected_count_bounded_session(&mut solver, &[1, 2], 100, &[a1, a2]).expect("count");
    assert_eq!(both.count, 0);
}
