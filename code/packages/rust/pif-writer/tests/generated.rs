//! Property-style tests, with no dependency beyond a tiny deterministic generator: for
//! hundreds of random terms the writer's output is accepted by the real reader (so it parses,
//! its indices are dense and backward, its counts are right and every term type-checks), it
//! is a pure function of the seed, and asking for a term twice emits it once.

use lean_kernel::pif::{read, FileClass, GateInputs, Kind};
use lean_kernel::Environment;
use pif_writer::{BinderInfo::Default as D, Expr, Hint, Level, Name, Writer};

/// xorshift64*: deterministic, good enough to vary shapes.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A random level over the parameters `ps`, at most `depth` constructors deep.
fn level(w: &mut Writer, r: &mut Rng, ps: &[Level], depth: u32) -> Level {
    let pick = if depth == 0 { r.below(2) } else { r.below(5) };
    match pick {
        0 => Level::ZERO,
        1 => ps[r.below(ps.len() as u64) as usize],
        2 => {
            let l = level(w, r, ps, depth - 1);
            w.level_succ(l)
        }
        3 => {
            let (a, b) = (level(w, r, ps, depth - 1), level(w, r, ps, depth - 1));
            w.level_max(a, b)
        }
        _ => {
            let (a, b) = (level(w, r, ps, depth - 1), level(w, r, ps, depth - 1));
            w.level_imax(a, b)
        }
    }
}

/// A random proposition built from the atoms by implication.
fn prop(w: &mut Writer, r: &mut Rng, atoms: &[Expr], anon: Name, depth: u32) -> Expr {
    if depth == 0 || r.below(3) == 0 {
        return atoms[r.below(atoms.len() as u64) as usize];
    }
    let (a, b) = (
        prop(w, r, atoms, anon, depth - 1),
        prop(w, r, atoms, anon, depth - 1),
    );
    w.pi(D, anon, a, b)
}

fn reviewed() -> GateInputs<'static> {
    GateInputs {
        class: FileClass::Reviewed,
        ..GateInputs::default()
    }
}

/// One generated file: level-polymorphic axioms, atoms, and identity and K theorems over
/// random propositions. Returns the text.
fn generate(seed: u64) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut w = Writer::new("native", "generated file")
        .unwrap()
        .ascii_names_only();
    let anon = w.name("_").unwrap();
    let (u, v) = (w.name("u").unwrap(), w.name("v").unwrap());
    let (lu, lv) = (w.level_param(u), w.level_param(v));
    for i in 0..1 + r.below(4) {
        let l = level(&mut w, &mut r, &[lu, lv], 4);
        let ty = w.sort(l);
        let n = w.name(&format!("Gen.Ty{i}")).unwrap();
        w.axiom(n, &[u, v], ty).unwrap();
    }
    let prop0 = w.sort(Level::ZERO);
    let mut atoms = vec![];
    for i in 0..1 + r.below(4) {
        let n = w.name(&format!("Gen.P{i}")).unwrap();
        w.axiom(n, &[], prop0).unwrap();
        atoms.push(w.constant(n, &[]));
    }
    let x = w.name("x").unwrap();
    let y = w.name("y").unwrap();
    for i in 0..1 + r.below(6) {
        let a = prop(&mut w, &mut r, &atoms, anon, 3);
        let b = prop(&mut w, &mut r, &atoms, anon, 3);
        let (v0, v1) = (w.var(0), w.var(1));
        // identity: A -> A
        let id_ty = w.pi(D, x, a, a);
        let id_val = w.lam(D, x, a, v0);
        let n = w.name(&format!("Gen.id{i}")).unwrap();
        w.theorem(n, &[], id_ty, id_val).unwrap();
        // K: A -> B -> A
        let inner = w.pi(D, y, b, a);
        let k_ty = w.pi(D, x, a, inner);
        let inner = w.lam(D, y, b, v1);
        let k_val = w.lam(D, x, a, inner);
        let n = w.name(&format!("Gen.k{i}")).unwrap();
        if r.below(2) == 0 {
            w.theorem(n, &[], k_ty, k_val).unwrap();
        } else {
            w.definition(n, &[], k_ty, k_val, Hint::Regular(r.below(9) as u32))
                .unwrap();
        }
    }
    w.to_text()
}

#[test]
fn generated_files_are_read_by_the_reader_under_reviewed_rules() {
    for seed in 0..300 {
        let text = generate(seed);
        let rep = match read(Environment::new(), text.as_bytes(), &reviewed()) {
            Ok(r) => r,
            Err(e) => panic!("seed {seed}: {e:?}\n{text}"),
        };
        assert!(rep
            .decls
            .iter()
            .any(|d| d.kind == Kind::Theorem || d.kind == Kind::Definition));
    }
}

#[test]
fn output_is_a_pure_function_of_the_calls() {
    for seed in 0..50 {
        assert_eq!(generate(seed), generate(seed));
    }
    assert_ne!(generate(1), generate(2));
}

#[test]
fn asking_for_a_term_twice_emits_it_once() {
    for seed in 0..100 {
        let mut r = Rng(seed + 1);
        let mut w = Writer::new("native", "s").unwrap();
        let (u, v) = (w.name("u").unwrap(), w.name("v").unwrap());
        let ps = [w.level_param(u), w.level_param(v)];
        let state = r.0;
        let l1 = level(&mut w, &mut r, &ps, 5);
        let count = w.record_count();
        r.0 = state;
        let l2 = level(&mut w, &mut r, &ps, 5);
        assert_eq!(l1, l2);
        assert_eq!(w.record_count(), count, "seed {seed}");
    }
}

#[test]
fn every_record_is_one_line_with_single_spaces_and_the_count_matches() {
    for seed in 0..50 {
        let text = generate(seed);
        assert!(text.ends_with('\n') && text.is_ascii());
        assert!(!text.contains("  ") && !text.contains("\n\n") && !text.contains(" \n"));
        let lines: Vec<&str> = text.lines().collect();
        let header = 4;
        assert_eq!(
            lines.last().unwrap().to_string(),
            format!("end {}", lines.len() - header - 1)
        );
    }
}
