//! Declaration records and inductive blocks: kind fidelity (a `theorem` record must
//! become a `Theorem`, never an `Axiom`), the kernel's verdict being forwarded, derived
//! names (R-DER-1), blocks, and files composed into a chain.

mod pif_common;

use lean_kernel::pif::{read, GateInputs, Kind, PifError as E};
use lean_kernel::{DeclKind, Environment, KernelError, Name, ReducibilityHints};
use pif_common::{err, lib, ok, B};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

/// `axiom A : Prop`, `axiom a : A`, and the index of the constant `A` (for statements).
fn with_a(b: &mut B) -> (usize, usize, usize) {
    let (_, ac) = b.prop_axiom("A");
    let an = b.name("a");
    b.axiom(an, &[], ac);
    let a_const = b.cnst(an, &[]);
    (ac, an, a_const)
}

// ---- kind fidelity -----------------------------------------------------------------------

#[test]
fn a_theorem_record_becomes_a_theorem_not_an_axiom() {
    let mut b = B::new();
    let (ac, _, a_val) = with_a(&mut b);
    let t = b.name("t");
    b.theorem(t, &[], ac, a_val);
    let r = ok(&b.bytes(), &lib());
    let d = r.env.get(&n("t")).unwrap();
    assert!(matches!(d.kind, DeclKind::Theorem { .. }), "{:?}", d.kind);
    assert!(d.value().is_some(), "a theorem keeps its proof");
    assert_eq!(r.decls.last().unwrap().kind, Kind::Theorem);
    // the two axioms before it are axioms
    assert!(r.decls[..2].iter().all(|d| d.kind == Kind::Axiom));
    assert!(matches!(r.env.get(&n("A")).unwrap().kind, DeclKind::Axiom));
}

#[test]
fn a_def_becomes_a_definition_with_its_hint_and_an_opaque_an_opaque() {
    let mut b = B::new();
    let (ac, _, a_val) = with_a(&mut b);
    for (name, hint) in [("d1", "@abbrev"), ("d2", "@opaque"), ("d3", "@regular 7")] {
        let d = b.name(name);
        b.def(d, &[], ac, a_val, hint);
    }
    let o = b.name("o");
    b.opaque(o, &[], ac, a_val);
    let r = ok(&b.bytes(), &lib());
    let hint = |s: &str| match &r.env.get(&n(s)).unwrap().kind {
        DeclKind::Definition { hints, .. } => *hints,
        k => panic!("{s} is {k:?}"),
    };
    assert_eq!(hint("d1"), ReducibilityHints::Abbrev);
    assert_eq!(hint("d2"), ReducibilityHints::Opaque);
    assert_eq!(hint("d3"), ReducibilityHints::Regular(7));
    assert!(matches!(
        r.env.get(&n("o")).unwrap().kind,
        DeclKind::Opaque { .. }
    ));
    let kinds: Vec<Kind> = r.decls.iter().map(|d| d.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::Axiom,
            Kind::Axiom,
            Kind::Definition,
            Kind::Definition,
            Kind::Definition,
            Kind::Opaque
        ]
    );
}

#[test]
fn the_report_reads_the_kind_back_from_the_environment() {
    // every entry's kind equals the kernel's own classification of that name
    let mut b = B::new();
    b.unit_type("T");
    let (ac, _, a_val) = with_a(&mut b);
    let t = b.name("t");
    b.theorem(t, &[], ac, a_val);
    let r = ok(&b.bytes(), &lib());
    for d in &r.decls {
        let want = match r.env.get(&d.name).unwrap().kind {
            DeclKind::Axiom => Kind::Axiom,
            DeclKind::Definition { .. } => Kind::Definition,
            DeclKind::Theorem { .. } => Kind::Theorem,
            DeclKind::Opaque { .. } => Kind::Opaque,
            DeclKind::Inductive(_) => Kind::Inductive,
            DeclKind::Constructor(_) => Kind::Constructor,
            DeclKind::Recursor(_) => Kind::Recursor,
        };
        assert_eq!(d.kind, want, "{}", d.name);
        assert_eq!(r.env.get(&d.name).unwrap().ty, d.ty);
    }
}

// ---- the kernel's verdict is forwarded ----------------------------------------------------

#[test]
fn a_theorem_whose_value_does_not_prove_its_type_is_rejected() {
    // theorem t : A := b    where b : B, and A, B are unrelated propositions
    let mut b = B::new();
    let (ac, _, _) = with_a(&mut b);
    let (bn, bc) = b.prop_axiom("B");
    let bval_name = b.name("b");
    b.axiom(bval_name, &[], bc);
    let bval = b.cnst(bval_name, &[]);
    let _ = bn;
    let t = b.name("t");
    b.theorem(t, &[], ac, bval);
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::TypeMismatch { .. })),
        "{e:?}"
    );
}

#[test]
fn a_theorem_must_state_a_proposition() {
    // theorem t : Type := Prop  has the right type but is not a Prop
    let mut b = B::new();
    let t = b.name("t");
    let prop = b.sort(0);
    let one = b.lsucc(0);
    let ty = b.sort(one);
    b.theorem(t, &[], ty, prop);
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::TheoremNotProp(_))),
        "{e:?}"
    );
}

#[test]
fn a_theorem_value_with_a_loose_variable_is_rejected() {
    // there is no record that adds an unchecked theorem: `theorem t : False := x` needs a
    // closed x of type False, and a bare variable is not closed
    let mut b = B::new();
    let (_, fc) = b.prop_axiom("False");
    let t = b.name("t");
    let bogus = b.var(0);
    b.theorem(t, &[], fc, bogus);
    let e = err(&b.bytes(), &lib());
    assert!(matches!(e, E::Kernel(KernelError::LooseBVar(0))), "{e:?}");
}

#[test]
fn kernel_errors_are_forwarded_as_they_are() {
    // duplicate declaration (R-DEC-1)
    let mut b = B::new();
    let (an, _) = b.prop_axiom("A");
    let prop = b.sort(0);
    b.axiom(an, &[], prop);
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::AlreadyDeclared(ref x)) if *x == n("A")),
        "{e:?}"
    );
    // unknown constant
    let mut b = B::new();
    let a = b.name("a");
    let ghost = b.name("Ghost");
    let c = b.cnst(ghost, &[]);
    b.axiom(a, &[], c);
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::UnknownConstant(_))),
        "{e:?}"
    );
    // an undeclared universe parameter
    let mut b = B::new();
    let a = b.name("a");
    let u = b.name("u");
    let l = b.lparam(u);
    let s = b.sort(l);
    b.axiom(a, &[], s);
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::UndeclaredLevelParam(_))),
        "{e:?}"
    );
    // a closed-term violation: a loose bound variable
    let mut b = B::new();
    let a = b.name("a");
    let v = b.var(0);
    b.axiom(a, &[], v);
    let e = err(&b.bytes(), &lib());
    assert!(matches!(e, E::Kernel(KernelError::LooseBVar(0))), "{e:?}");
}

#[test]
fn a_failed_file_returns_no_environment() {
    // the first declaration is fine and the second is not: nothing is returned at all
    let mut b = B::new();
    b.prop_axiom("A");
    let bad = b.name("bad");
    let v = b.var(0);
    b.axiom(bad, &[], v);
    assert!(read(Environment::new(), &b.bytes(), &lib()).is_err());
}

#[test]
fn a_universe_polymorphic_declaration_is_accepted() {
    let mut b = B::new();
    let id = b.name("Sorty");
    let u = b.name("u");
    let l = b.lparam(u);
    let s = b.sort(l);
    let succ = b.lsucc(l);
    let ty = b.sort(succ);
    b.def(id, &[u], ty, s, "@abbrev");
    let r = ok(&b.bytes(), &lib());
    assert_eq!(r.env.get(&n("Sorty")).unwrap().level_params, vec![n("u")]);
}

#[test]
fn level_records_are_built_by_the_matching_constructors() {
    // max and imax of a parameter and a successor: Sort (max u (imax u 1)) is well-formed
    let mut b = B::new();
    let a = b.name("a");
    let u = b.name("u");
    let lu = b.lparam(u);
    let one = b.lsucc(0);
    b.line(&format!("limax {} {lu} {one}", b.nl + 1));
    b.nl += 1;
    let im = b.nl;
    b.line(&format!("lmax {} {lu} {im}", b.nl + 1));
    b.nl += 1;
    let mx = b.nl;
    let s = b.sort(mx);
    let ty = b.sort(mx);
    b.axiom(a, &[u], ty);
    let _ = s;
    ok(&b.bytes(), &lib());
}

#[test]
fn let_and_binder_infos_are_read() {
    // a let-bound identity on Prop: (fun (x : Prop) => x) with every binder info
    let mut b = B::new();
    let a = b.name("a");
    let x = b.name("x");
    let prop = b.sort(0);
    let v0 = b.var(0);
    for info in ["default", "implicit", "strict", "inst"] {
        b.ne += 1;
        b.line(&format!("elam {} {info} {x} {prop} {v0}", b.ne));
    }
    let lam = b.ne;
    b.ne += 1;
    b.line(&format!("elet {} {x} {prop} {prop} {v0}", b.ne));
    let let_e = b.ne;
    let pi = b.pi(x, prop, prop);
    b.axiom(a, &[], pi);
    let t = b.name("t");
    b.def(t, &[], pi, lam - 3, "@abbrev");
    let _ = let_e;
    let r = ok(&b.bytes(), &lib());
    assert_eq!(r.decls.len(), 2);
}

// ---- inductive blocks ---------------------------------------------------------------------

#[test]
fn an_inductive_block_adds_the_type_its_constructors_and_a_derived_recursor() {
    let mut b = B::new();
    b.unit_type("T");
    let r = ok(&b.bytes(), &lib());
    let got: Vec<(Kind, String)> = r
        .decls
        .iter()
        .map(|d| (d.kind, d.name.to_string()))
        .collect();
    let want = [
        (Kind::Inductive, "T"),
        (Kind::Constructor, "T.mk"),
        (Kind::Recursor, "T.rec"),
    ];
    assert_eq!(got, want.map(|(k, s)| (k, s.to_string())));
    assert!(matches!(
        r.env.get(&n("T.rec")).unwrap().kind,
        DeclKind::Recursor(_)
    ));
}

#[test]
fn r_der_1_a_derived_name_that_is_taken_refuses_the_block() {
    // axiom T.rec declared before inductive T
    let mut b = B::new();
    let t = b.name("T");
    let rec = b.nstr(t, "rec");
    let prop = b.sort(0);
    b.axiom(rec, &[], prop);
    let one = b.lsucc(0);
    let ty = b.sort(one);
    let tc = b.cnst(t, &[]);
    let mk = b.nstr(t, "mk");
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 1"));
    b.line(&format!("ictor {mk} {tc}"));
    assert_eq!(err(&b.bytes(), &lib()), E::DerivedNameTaken);
    // a constructor named T.rec
    let mut b = B::new();
    let t = b.name("T");
    let rec = b.nstr(t, "rec");
    let one = b.lsucc(0);
    let ty = b.sort(one);
    let tc = b.cnst(t, &[]);
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 1"));
    b.line(&format!("ictor {rec} {tc}"));
    assert_eq!(err(&b.bytes(), &lib()), E::DerivedNameTaken);
    // the type itself already declared is the kernel's AlreadyDeclared
    let mut b = B::new();
    b.prop_axiom("T");
    let t = 1;
    let one = b.lsucc(0);
    let ty = b.sort(one);
    let tc = b.cnst(t, &[]);
    let mk = b.nstr(t, "mk");
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 1"));
    b.line(&format!("ictor {mk} {tc}"));
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::AlreadyDeclared(_))),
        "{e:?}"
    );
}

#[test]
fn an_inductive_the_kernel_rejects_is_rejected() {
    // a constructor that does not return its type
    let mut b = B::new();
    let t = b.name("T");
    let mk = b.nstr(t, "mk");
    let one = b.lsucc(0);
    let ty = b.sort(one);
    let prop = b.sort(0);
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 1"));
    b.line(&format!("ictor {mk} {prop}"));
    let e = err(&b.bytes(), &lib());
    assert!(
        matches!(e, E::Kernel(KernelError::InvalidInductive(_))),
        "{e:?}"
    );
    // a type whose own type mentions loose variables
    let mut b = B::new();
    let t = b.name("T");
    let v = b.var(0);
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {v} 0"));
    let e = err(&b.bytes(), &lib());
    assert!(matches!(e, E::Kernel(KernelError::LooseBVar(_))), "{e:?}");
}

#[test]
fn a_declared_parameter_count_the_types_do_not_have_is_the_kernels_error() {
    let mut b = B::new();
    let t = b.name("T");
    let one = b.lsucc(0);
    let ty = b.sort(one);
    b.line("inductive [] 3 1");
    b.line(&format!("itype {t} {ty} 0"));
    let e = err(&b.bytes(), &lib());
    assert!(matches!(e, E::Kernel(_)), "{e:?}");
}

// ---- composition ----------------------------------------------------------------------------

#[test]
fn files_compose_through_an_environment_chain() {
    let digest = "c".repeat(64);
    let mut first = B::new();
    first.prop_axiom("A");
    let g1 = lib();
    let r1 = read(Environment::new(), &first.bytes(), &g1).expect("first file");
    // the second file's indices are its own; it refers to A by name only
    let mut second = B::new();
    second.header.push(format!("import sha256:{digest}"));
    let a = second.name("A");
    let t = second.name("t");
    let ac = second.cnst(a, &[]);
    let prop = second.sort(0);
    second.axiom(t, &[], prop);
    let u = second.name("u");
    second.axiom(u, &[], ac);
    let g2 = GateInputs {
        expected_imports: vec![digest.clone()],
        ..lib()
    };
    let r2 = read(r1.env, &second.bytes(), &g2).expect("second file");
    assert_eq!(r2.decls.len(), 2);
    assert!(r2.env.contains(&n("A")) && r2.env.contains(&n("u")));
    // the same file read again into the extended environment redeclares and is refused
    let again = read(r2.env, &second.bytes(), &g2);
    assert!(matches!(
        again,
        Err(E::Kernel(KernelError::AlreadyDeclared(_)))
    ));
}
