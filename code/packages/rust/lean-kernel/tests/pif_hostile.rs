//! Hostile input (VER32 section 7 and 10): every limit, a DAG denoting 2^1000 nodes, and
//! thousands of deterministic mutations of the worked example. Every test runs on a
//! large-stack thread, so "passes" means an error (or an acceptance) was *returned*, not
//! that the stack happened to be big enough for a panic-free crash.

mod pif_common;

use lean_kernel::pif::{read, FileClass, GateInputs, PifError as E, PifLimits};
use lean_kernel::{Environment, KernelError};
use pif_common::{err, lib, ok, B};
use std::time::{Duration, Instant};

// Time bounds are deliberately generous (60 s): they exist to catch an exponential or
// quadratic blow-up, which takes hours, not to benchmark; instrumented builds are slow.

fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

fn limited(f: impl FnOnce(&mut PifLimits)) -> GateInputs<'static> {
    let mut g = lib();
    f(&mut g.limits);
    g
}

const HEAD: &str = "pif 1\nlogic 1\nprofile native \"t\"\nfeatures\n";

// ---- section 7, row by row ----------------------------------------------------------------

#[test]
fn file_size_is_checked_before_anything_is_read() {
    big(|| {
        let max = 64usize << 20;
        let over = vec![b'a'; max + 1];
        assert_eq!(err(&over, &lib()), E::LimitExceeded("file size"));
        // a non-ASCII file over the limit reports the size, proving no scan ran first
        let over = vec![0xffu8; max + 1];
        assert_eq!(err(&over, &lib()), E::LimitExceeded("file size"));
        // exactly at the limit it is a lexical error, not a limit error
        let at = b"a\n".repeat(max / 2);
        assert_eq!(at.len(), max);
        assert_eq!(err(&at, &lib()), E::BadLexicalForm);
    });
}

#[test]
fn line_length_is_checked_before_a_line_is_buffered() {
    big(|| {
        let line = |n: usize| format!("{HEAD}{}\nend 1\n", "a".repeat(n));
        assert_eq!(
            err(line((1 << 20) + 1).as_bytes(), &lib()),
            E::LimitExceeded("line length")
        );
        // exactly 1 MiB is within the limit; it is then just not a record
        assert_eq!(err(line(1 << 20).as_bytes(), &lib()), E::BadLexicalForm);
        // a 10 MB line with no LF at all
        let long = vec![b'a'; 10_000_000];
        assert_eq!(err(&long, &lib()), E::LimitExceeded("line length"));
    });
}

#[test]
fn records_per_file_limit_and_a_header_that_claims_a_huge_count() {
    let g = limited(|l| l.records = 3);
    let text = |n: usize| {
        let mut b = B::new();
        for _ in 0..n {
            b.sort(0);
        }
        b.text()
    };
    ok(text(3).as_bytes(), &g);
    assert_eq!(
        err(text(4).as_bytes(), &g),
        E::LimitExceeded("records per file")
    );
    // an `end` that claims 4 billion lines is just a count that does not match
    let huge = text(2).replace("end 2", "end 4294967295");
    assert_eq!(err(huge.as_bytes(), &lib()), E::TrailerMismatch);
    let huger = text(2).replace("end 2", "end 99999999999999999999");
    assert_eq!(err(huger.as_bytes(), &lib()), E::NumberOutOfRange);
}

#[test]
fn string_length_limit_applies_to_names_and_to_the_profile() {
    let g = limited(|l| l.string_bytes = 8);
    let mut b = B::new();
    b.nstr(0, "12345678");
    ok(&b.bytes(), &g);
    let mut b = B::new();
    b.nstr(0, "123456789");
    assert_eq!(err(&b.bytes(), &g), E::LimitExceeded("string length"));
    // the limit counts decoded bytes: a 3-byte character is 3
    let mut b = B::new();
    b.nstr(0, "\\u{20ac}\\u{20ac}\\u{20ac}");
    assert_eq!(err(&b.bytes(), &g), E::LimitExceeded("string length"));
    let long_profile = format!(
        "pif 1\nlogic 1\nprofile native \"{}\"\nfeatures\nend 0\n",
        "p".repeat(9)
    );
    assert_eq!(
        err(long_profile.as_bytes(), &g),
        E::LimitExceeded("string length")
    );
}

#[test]
fn expression_depth_is_one_thousand_and_a_million_deep_chain_stops_early() {
    big(|| {
        let chain = |n: usize| {
            let mut b = B::new();
            let first = b.sort(0);
            (1..n).fold(first, |prev, _| b.app(prev, first));
            b
        };
        ok(&chain(1000).bytes(), &lib());
        assert_eq!(
            err(&chain(1001).bytes(), &lib()),
            E::LimitExceeded("expression depth")
        );
        // a million records written with backward references only: rejected at record 1001
        let mut text = String::from(HEAD);
        text.push_str("esort 1 0\n");
        for i in 2..=1_000_000 {
            text.push_str(&format!("eapp {i} {} 1\n", i - 1));
        }
        text.push_str("end 1000000\n");
        let t = Instant::now();
        assert_eq!(
            err(text.as_bytes(), &lib()),
            E::LimitExceeded("expression depth")
        );
        assert!(t.elapsed() < Duration::from_secs(60));
    });
}

#[test]
fn expression_depth_counts_the_deepest_child_in_every_position() {
    // a chain whose deep child sits in each possible operand position of each node kind
    let shapes: [(&str, &str); 9] = [
        ("eapp {i} {p} 1", "app, deep function"),
        ("eapp {i} 1 {p}", "app, deep argument"),
        ("elam {i} default 2 {p} 1", "lambda, deep domain"),
        ("elam {i} default 2 1 {p}", "lambda, deep body"),
        ("epi {i} default 2 {p} 1", "pi, deep domain"),
        ("epi {i} default 2 1 {p}", "pi, deep body"),
        ("elet {i} 2 {p} 1 1", "let, deep type"),
        ("elet {i} 2 1 {p} 1", "let, deep value"),
        ("elet {i} 2 1 1 {p}", "let, deep body"),
    ];
    for (shape, what) in shapes {
        let build = |n: usize| {
            let mut b = B::new();
            b.nstr(0, "x");
            b.nstr(0, "y");
            let mut text = b.text();
            // the header, two names, then expression 1 (a Sort) and a chain over it
            text = text.replace("end 2\n", "esort 1 0\n");
            for i in 2..=n {
                let line = shape
                    .replace("{i}", &i.to_string())
                    .replace("{p}", &(i - 1).to_string());
                text.push_str(&line);
                text.push('\n');
            }
            text.push_str(&format!("end {}\n", n + 2));
            text
        };
        ok(build(1000).as_bytes(), &lib());
        let e = err(build(1001).as_bytes(), &lib());
        assert_eq!(e, E::LimitExceeded("expression depth"), "{what}");
    }
}

#[test]
fn level_depth_and_size_are_bounded_at_insert() {
    big(|| {
        let g = lib();
        // a successor chain: 127 deep is fine, deeper is refused as it is built
        let mut b = B::new();
        (0..127).fold(0, |p, _| b.lsucc(p));
        ok(&b.bytes(), &g);
        let mut b = B::new();
        (0..140).fold(0, |p, _| b.lsucc(p));
        assert_eq!(err(&b.bytes(), &g), E::LimitExceeded("level depth or size"));
        // a doubling DAG: depth 100 would denote 2^100 paths; the tree size stops it at once
        let mut text = String::from(HEAD);
        text.push_str("lsucc 1 0\n");
        for i in 2..=100 {
            text.push_str(&format!("lmax {i} {} {}\n", i - 1, i - 1));
        }
        text.push_str("end 100\n");
        let t = Instant::now();
        assert_eq!(
            err(text.as_bytes(), &g),
            E::LimitExceeded("level depth or size")
        );
        assert!(t.elapsed() < Duration::from_secs(60));
        // two separately built copies of the same big level would be an exponential equality
        // check inside the interner; neither copy is ever built
        let mut text = String::from(HEAD);
        text.push_str("lsucc 1 0\nlsucc 2 0\n");
        for i in 0..100 {
            text.push_str(&format!(
                "limax {} {} {}\n",
                3 + 2 * i,
                1 + 2 * i,
                1 + 2 * i
            ));
            text.push_str(&format!(
                "limax {} {} {}\n",
                4 + 2 * i,
                2 + 2 * i,
                2 + 2 * i
            ));
        }
        text.push_str(&format!("esort 1 {}\nesort 2 {}\nend 204\n", 202, 203));
        assert_eq!(
            err(text.as_bytes(), &g),
            E::LimitExceeded("level depth or size")
        );
    });
}

#[test]
fn universe_parameter_and_constant_level_limits() {
    let mut b = B::new();
    let a = b.name("a");
    let names: Vec<usize> = (0..65).map(|i| b.name(&format!("u{i}"))).collect();
    let prop = b.sort(0);
    b.axiom(a, &names[..64], prop);
    ok(&b.bytes(), &lib());
    let mut b = B::new();
    let a = b.name("a");
    let names: Vec<usize> = (0..65).map(|i| b.name(&format!("u{i}"))).collect();
    let prop = b.sort(0);
    b.axiom(a, &names, prop);
    assert_eq!(
        err(&b.bytes(), &lib()),
        E::LimitExceeded("universe parameters")
    );
    // levels per constant: 64 are fine for the reader, 65 are not
    let mut b = B::new();
    let c = b.name("c");
    let ls = vec![0; 65];
    b.cnst(c, &ls[..64]);
    ok(&b.bytes(), &lib());
    let mut b = B::new();
    let c = b.name("c");
    b.cnst(c, &ls);
    assert_eq!(
        err(&b.bytes(), &lib()),
        E::LimitExceeded("levels per constant")
    );
}

#[test]
fn declaration_and_import_limits() {
    let g = limited(|l| l.decls = 2);
    let mut b = B::new();
    b.prop_axiom("A");
    b.prop_axiom("B");
    ok(&b.bytes(), &g);
    b.prop_axiom("C");
    assert_eq!(
        err(&b.bytes(), &g),
        E::LimitExceeded("declarations per file")
    );
    // an inductive block counts as a declaration too
    let mut b = B::new();
    b.prop_axiom("A");
    b.prop_axiom("B");
    b.unit_type("T");
    assert_eq!(
        err(&b.bytes(), &g),
        E::LimitExceeded("declarations per file")
    );
    // 65 imports
    let mut b = B::new();
    let digests: Vec<String> = (0..65).map(|i| format!("{i:064x}")).collect();
    for d in &digests[..64] {
        b.header.push(format!("import sha256:{d}"));
    }
    let g = GateInputs {
        expected_imports: digests[..64].to_vec(),
        ..lib()
    };
    ok(&b.bytes(), &g);
    b.header.push(format!("import sha256:{}", digests[64]));
    assert_eq!(err(&b.bytes(), &g), E::LimitExceeded("imports"));
}

#[test]
fn inductive_block_limits_are_checked_before_counts_or_features() {
    let head = |ps: &str, types: &str| {
        let mut b = B::new();
        b.line(&format!("inductive [] {ps} {types}"));
        b
    };
    assert_eq!(
        err(&head("257", "1").bytes(), &lib()),
        E::LimitExceeded("inductive block size")
    );
    // 65 types is over the limit; 2 is merely a mutual block that needs its feature
    assert_eq!(
        err(&head("0", "65").bytes(), &lib()),
        E::LimitExceeded("inductive block size")
    );
    assert_eq!(err(&head("0", "2").bytes(), &lib()), E::FeatureNotDeclared);
    // a huge declared constructor count is refused before anything is allocated for it
    let mut b = B::new();
    let t = b.name("T");
    let ty = b.sort(0);
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 4097"));
    assert_eq!(
        err(&b.bytes(), &lib()),
        E::LimitExceeded("constructors per type")
    );
    let mut b = B::new();
    let t = b.name("T");
    let ty = b.sort(0);
    b.line("inductive [] 0 1");
    b.line(&format!("itype {t} {ty} 4294967295"));
    assert_eq!(
        err(&b.bytes(), &lib()),
        E::LimitExceeded("constructors per type")
    );
    // more itype lines than declared stops at the first extra one
    let mut b = B::new();
    b.unit_type("T");
    for _ in 0..1000 {
        b.line("itype 1 1 0");
    }
    assert_eq!(err(&b.bytes(), &lib()), E::CountMismatch);
}

// ---- the 2^1000 DAG -----------------------------------------------------------------------

/// `axiom T : Prop; axiom P : Prop -> Prop -> Prop; p_0 = T; p_i = P p_{i-1} p_{i-1}`, then
/// `axiom big : p_n`: a well-typed proposition denoting a tree of 2^n nodes in about 2n records.
fn doubling(n: usize) -> B {
    let mut b = B::new();
    let (_, t) = b.prop_axiom("T");
    let pn = b.name("P");
    let prop = b.sort(0);
    let x = b.name("x");
    let arrow = b.pi(x, prop, prop);
    let arrow2 = b.pi(x, prop, arrow);
    b.axiom(pn, &[], arrow2);
    let p = b.cnst(pn, &[]);
    let p_i = (0..n).fold(t, |prev, _| {
        let f = b.app(p, prev);
        b.app(f, prev)
    });
    let big = b.name("big");
    b.axiom(big, &[], p_i);
    b
}

#[test]
fn a_dag_denoting_two_to_the_ninety_nodes_is_checked_in_linear_time() {
    big(|| {
        let start = Instant::now();
        let r = ok(&doubling(90).bytes(), &lib());
        assert_eq!(r.decls.len(), 3);
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{:?}",
            start.elapsed()
        );
    });
}

#[test]
fn a_dag_denoting_two_to_the_five_hundred_nodes_reaches_the_kernels_depth_limit() {
    // the reader builds all 997 records (depth 999 is under its limit of 1000); the
    // kernel's own call-depth limit then stops the check, quickly and with an error
    big(|| {
        let start = Instant::now();
        let e = err(&doubling(499).bytes(), &lib());
        assert_eq!(e, E::Kernel(KernelError::ResourceLimit("recursion depth")));
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{:?}",
            start.elapsed()
        );
    });
}

#[test]
fn the_literal_two_to_the_thousand_shape_is_rejected_quickly_by_the_kernel() {
    big(|| {
        // eapp i (i-1) (i-1), repeated: ill-typed, so the kernel must stop, not enumerate paths
        let mut b = B::new();
        let t = b.name("T");
        let c = b.cnst(t, &[]);
        let last = (0..990).fold(c, |prev, _| b.app(prev, prev));
        let a = b.name("a");
        b.axiom(a, &[], last);
        let start = Instant::now();
        let e = err(&b.bytes(), &lib());
        assert!(matches!(e, E::Kernel(_)), "{e:?}");
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{:?}",
            start.elapsed()
        );
    });
}

#[test]
fn many_records_are_read_in_linear_time() {
    big(|| {
        let mut text = String::from(HEAD);
        for i in 1..=200_000 {
            text.push_str(&format!("nstr {i} 0 \"n{i}\"\n"));
        }
        text.push_str("end 200000\n");
        let start = Instant::now();
        ok(text.as_bytes(), &lib());
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{:?}",
            start.elapsed()
        );
    });
}

// ---- mutation: no panic, ever --------------------------------------------------------------

const EXAMPLE: &str = include_str!("fixtures/worked-example.pif");

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Whatever the reader accepts, its report must agree with the records that were written.
fn check_accepted(text: &[u8], g: &GateInputs) {
    let Ok(r) = read(Environment::new(), text, g) else {
        return;
    };
    let src = String::from_utf8_lossy(text);
    let count = |kw: &str| {
        src.lines()
            .filter(|l| l.starts_with(&format!("{kw} ")))
            .count()
    };
    let kinds = |k| r.decls.iter().filter(|d| d.kind == k).count();
    use lean_kernel::pif::Kind::*;
    assert_eq!(kinds(Theorem), count("theorem"));
    assert_eq!(kinds(Axiom), count("axiom"));
    assert_eq!(kinds(Definition), count("def"));
    assert_eq!(kinds(Opaque), count("opaque"));
    assert_eq!(kinds(Inductive), count("itype"));
}

#[test]
fn every_truncation_of_the_example_is_rejected() {
    big(|| {
        let g = GateInputs::default();
        for cut in 0..EXAMPLE.len() {
            let r = read(Environment::new(), &EXAMPLE.as_bytes()[..cut], &g);
            assert!(r.is_err(), "a {cut}-byte prefix was accepted");
        }
    });
}

#[test]
fn single_byte_mutations_never_panic_and_kinds_stay_faithful() {
    big(|| {
        let g = GateInputs::default();
        let bytes = EXAMPLE.as_bytes();
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for _ in 0..3000 {
            let mut m = bytes.to_vec();
            let i = rng.below(m.len());
            match rng.below(4) {
                0 => m[i] = (rng.next() & 0xff) as u8,
                1 => m[i] = b"0123456789 \n\"\\[]ax"[rng.below(18)],
                2 => {
                    m.remove(i);
                }
                _ => m.insert(i, b"0123456789 \n\"\\[]ax"[rng.below(18)]),
            }
            check_accepted(&m, &g);
        }
    });
}

#[test]
fn line_level_mutations_never_panic_and_kinds_stay_faithful() {
    big(|| {
        let g = GateInputs::default();
        let lines: Vec<&str> = EXAMPLE.lines().collect();
        let mut rng = Rng(0xdead_beef_cafe_f00d);
        for _ in 0..2000 {
            let mut v: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            for _ in 0..1 + rng.below(3) {
                let i = 4 + rng.below(v.len() - 4);
                match rng.below(5) {
                    0 => {
                        v.remove(i);
                    }
                    1 => v.insert(i, v[rng.below(v.len())].clone()),
                    2 => {
                        let j = 4 + rng.below(v.len() - 4);
                        v.swap(i, j);
                    }
                    3 => {
                        // bump the first number on the line
                        let l = &v[i];
                        v[i] = l.replacen(
                            |c: char| c.is_ascii_digit(),
                            &(rng.below(120)).to_string(),
                            1,
                        );
                    }
                    _ => v[i] = v[i].replace("theorem", "axiom"),
                }
            }
            let text = v.join("\n") + "\n";
            check_accepted(text.as_bytes(), &g);
        }
    });
}

#[test]
fn random_bytes_never_panic() {
    big(|| {
        let mut rng = Rng(0x1234_5678_9abc_def1);
        let g = lib();
        for _ in 0..2000 {
            let n = rng.below(300);
            let mut m: Vec<u8> = b"pif 1\nlogic 1\nprofile native \"t\"\nfeatures\n".to_vec();
            m.extend((0..n).map(|_| b"abc 0123\n\"[]\\u{}esortnsr"[rng.below(24)]));
            let _ = read(Environment::new(), &m, &g);
        }
    });
}

#[test]
fn an_accepted_theorem_is_never_unchecked() {
    // flip the worked example's theorem record into an axiom record: it is no longer a
    // theorem, the goal rule then refuses to bind it (it is not a theorem declared here),
    // and under claim rules the axiom itself is refused
    let flipped = EXAMPLE.replace("\ntheorem 14 [] 38 71\n", "\naxiom 14 [] 38\n");
    assert_ne!(flipped, EXAMPLE);
    assert_eq!(
        err(flipped.as_bytes(), &GateInputs::default()),
        E::AxiomNotAllowed
    );
    let g = GateInputs {
        class: FileClass::Library,
        ..GateInputs::default()
    };
    assert_eq!(err(flipped.as_bytes(), &g), E::GoalTargetNotTheorem);
    // and a theorem that the kernel refuses stays refused whatever the class
    let wrong = EXAMPLE.replace("theorem 14 [] 38 71", "theorem 14 [] 38 39");
    let e = err(wrong.as_bytes(), &g);
    assert!(
        matches!(e, E::Kernel(KernelError::TypeMismatch { .. })),
        "{e:?}"
    );
}

// ---- security review: one huge legal name, referenced millions of times ---------------------

/// `chain` components of `len` characters each, then `refs` constants that all name the deepest.
fn long_name_file(chain: usize, len: usize, refs: usize) -> String {
    let mut text = String::from(HEAD);
    let comp = "a".repeat(len);
    for i in 1..=chain {
        text.push_str(&format!("nstr {i} {} \"{comp}\"\n", i - 1));
    }
    for i in 1..=refs {
        text.push_str(&format!("econst {i} {chain} []\n"));
    }
    text.push_str(&format!("end {}\n", chain + refs));
    text
}

#[test]
fn a_name_longer_than_the_total_byte_limit_is_refused_at_insert() {
    big(|| {
        // 32 components of 64 KiB: the legal-per-component 2 MiB name of the review
        let text = long_name_file(32, 65536, 1000);
        let t = Instant::now();
        assert_eq!(err(text.as_bytes(), &lib()), E::LimitExceeded("name bytes"));
        assert!(t.elapsed() < Duration::from_secs(60));
        // many short components add up the same way: 5 x 1000 > 4096, 4 x 1000 is fine
        ok(long_name_file(4, 1000, 1).as_bytes(), &lib());
        let five = long_name_file(5, 1000, 1);
        assert_eq!(err(five.as_bytes(), &lib()), E::LimitExceeded("name bytes"));
        // numeric components count too (8 bytes each): two fit in 16, three do not
        let g = limited(|l| l.name_bytes = 16);
        let nums = |n: usize| {
            let body: String = (1..=n).map(|i| format!("nnum {i} {} 1\n", i - 1)).collect();
            format!("{HEAD}{body}end {n}\n")
        };
        ok(nums(2).as_bytes(), &g);
        assert_eq!(err(nums(3).as_bytes(), &g), E::LimitExceeded("name bytes"));
    });
}

#[test]
fn a_huge_name_referenced_very_often_costs_time_proportional_to_the_bytes_read() {
    big(|| {
        // with the reader limit raised, the kernel-side cached name hash must keep 100,000
        // references to a 2 MiB name from costing 100,000 x 2 MiB of hashing
        let g = limited(|l| l.name_bytes = 4 << 20);
        let text = long_name_file(32, 65536, 100_000);
        let t = Instant::now();
        ok(text.as_bytes(), &g);
        let long = t.elapsed();
        assert!(long < Duration::from_secs(60), "{long:?}");
        // the same file shape over a short name costs about the same
        let short = long_name_file(1, 4, 100_000);
        let t = Instant::now();
        ok(short.as_bytes(), &lib());
        let s = t.elapsed();
        assert!(
            long < s * 50 + Duration::from_secs(5),
            "long {long:?} vs short {s:?}"
        );
    });
}
