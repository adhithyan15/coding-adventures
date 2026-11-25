//! The VER32 section 4.6 worked example, rebuilt through the writer and compared byte for
//! byte with the committed fixture the reader's tests use. The calls below follow the
//! fixture's creation order; the writer numbers everything itself.

use lean_kernel::pif::{read, GateInputs};
use lean_kernel::Environment;
use pif_writer::{BinderInfo::Default as D, Hint, IndType, Level, Writer};

const FIXTURE: &str = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");

fn build() -> String {
    let mut w = Writer::new("native", "pif-case-split 0.1").unwrap();
    // inductive Bool : Type | false | true
    let bool_n = w.name("Bool").unwrap();
    let ff = w.name("Bool.false").unwrap();
    let tt = w.name("Bool.true").unwrap();
    let l1 = w.level_succ(Level::ZERO);
    let ty = w.sort(l1);
    let bool_t = w.constant(bool_n, &[]);
    let ty_bool = IndType {
        name: bool_n,
        ty,
        ctors: vec![(ff, bool_t), (tt, bool_t)],
    };
    w.inductive(&[], 0, &[ty_bool]).unwrap();

    // inductive Eq.{u} {alpha : Sort u} (a : alpha) : alpha -> Prop | refl : Eq a a
    let eq_n = w.name("Eq").unwrap();
    let refl = w.name("Eq.refl").unwrap();
    let u = w.name("u").unwrap();
    let lu = w.level_param(u);
    let sort_u = w.sort(lu);
    let prop = w.sort(Level::ZERO);
    let (v0, v1) = (w.var(0), w.var(1));
    let (alpha, a, anon) = (
        w.name("alpha").unwrap(),
        w.name("a").unwrap(),
        w.name("_").unwrap(),
    );
    let p1 = w.pi(D, anon, v1, prop);
    let p2 = w.pi(D, a, v0, p1);
    let eq_ty = w.pi(D, alpha, sort_u, p2);
    let eq_c = w.constant(eq_n, &[lu]);
    let e = w.app(eq_c, v1);
    let e = w.app(e, v0);
    let e = w.app(e, v0);
    let c1 = w.pi(D, a, v0, e);
    let refl_ty = w.pi(D, alpha, sort_u, c1);
    let ty_eq = IndType {
        name: eq_n,
        ty: eq_ty,
        ctors: vec![(refl, refl_ty)],
    };
    w.inductive(&[u], 2, &[ty_eq]).unwrap();

    // def Pif.and (a b : Bool) : Bool := Bool.rec (fun _ => Bool) false a b
    let and_n = w.name("Pif.and").unwrap();
    let rec_n = w.name("Bool.rec").unwrap();
    let rec = w.constant(rec_n, &[l1]);
    let (fc, tc) = (w.constant(ff, &[]), w.constant(tt, &[]));
    let b = w.name("b").unwrap();
    let motive = w.lam(D, anon, bool_t, bool_t);
    let r = w.app_many(rec, &[motive, fc, v0, v1]);
    let lb = w.lam(D, b, bool_t, r);
    let and_val = w.lam(D, a, bool_t, lb);
    let arr = w.pi(D, b, bool_t, bool_t);
    let and_ty = w.pi(D, a, bool_t, arr);
    w.definition(and_n, &[], and_ty, and_val, Hint::Regular(1))
        .unwrap();

    // theorem Pif.and_comm : forall a b, Eq Bool (and a b) (and b a)
    let and_c = w.constant(and_n, &[]);
    let eq1 = w.constant(eq_n, &[l1]);
    let ab = w.app_many(and_c, &[v1, v0]);
    let and_b = w.app(and_c, v0);
    let ba = w.app(and_b, v1);
    let eq_bool = w.app(eq1, bool_t);
    let stmt = w.app_many(eq_bool, &[ab, ba]);
    let inner = w.pi(D, b, bool_t, stmt);
    let thm_ty = w.pi(D, a, bool_t, inner);
    let motive_a = w.lam(D, a, bool_t, inner);
    let rec0 = w.constant(rec_n, &[Level::ZERO]);
    let refl1 = w.constant(refl, &[l1]);
    let refl_b = w.app(refl1, bool_t);
    let (rf, rt) = (w.app(refl_b, fc), w.app(refl_b, tc));
    // case a = false: fun b => Bool.rec (fun b => Eq (and false b) (and b false)) rf rf b
    let and_f = w.app(and_c, fc);
    let and_fb = w.app(and_f, v0);
    let and_b_f = w.app(and_b, fc);
    let eq_f = w.app(eq_bool, and_fb);
    let eq_f = w.app(eq_f, and_b_f);
    let m_f = w.lam(D, b, bool_t, eq_f);
    let case_f = w.app_many(rec0, &[m_f, rf, rf, v0]);
    let case_f = w.lam(D, b, bool_t, case_f);
    // case a = true: the same with true in place of false
    let and_t = w.app(and_c, tc);
    let and_tb = w.app(and_t, v0);
    let and_b_t = w.app(and_b, tc);
    let eq_t = w.app(eq_bool, and_tb);
    let eq_t = w.app(eq_t, and_b_t);
    let m_t = w.lam(D, b, bool_t, eq_t);
    let case_t = w.app_many(rec0, &[m_t, rf, rt, v0]);
    let case_t = w.lam(D, b, bool_t, case_t);
    let body = w.app_many(rec0, &[motive_a, case_f, case_t, v0]);
    let thm_val = w.lam(D, a, bool_t, body);
    let thm = w.name("Pif.and_comm").unwrap();
    w.theorem(thm, &[], thm_ty, thm_val).unwrap();
    let digest = "bf1fa0a0d828b019965f46efa44ff2627bf54a59558a1bc2959b52c161f74676";
    w.goal("gates.and_comm", thm, digest).unwrap();
    w.to_text()
}

#[test]
fn the_writer_reproduces_the_worked_example_byte_for_byte() {
    assert_eq!(build(), FIXTURE);
}

#[test]
fn the_rebuilt_example_passes_the_reader_under_the_strictest_rules() {
    let text = build();
    let r = read(Environment::new(), text.as_bytes(), &GateInputs::default()).unwrap();
    assert_eq!(r.goals.len(), 1);
    assert_eq!(r.decls.len(), 9);
}
