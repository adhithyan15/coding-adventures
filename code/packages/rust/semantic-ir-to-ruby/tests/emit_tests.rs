//! Behavioural tests for the Ruby backend.
//!
//! Emit-shape assertions run everywhere (no `ruby` needed).  The end-to-end
//! tests lower real source, emit Ruby, and — when a `ruby` interpreter is on
//! `PATH` — run it and check stdout, skipping gracefully otherwise (the
//! toolchain-gated convention the conformance harness uses).

use semantic_ir_to_ruby::{compile, sanitize_ident};

/// Lower Ruby source → SIR → Ruby text.
fn ruby_to_ruby(src: &str) -> String {
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    compile(&module).expect("ruby emit").source
}

/// Lower Twig source → SIR → Ruby text.
fn twig_to_ruby(src: &str) -> String {
    let module = twig_to_semantic_ir::compile_source(src, "prog").expect("twig lowering");
    compile(&module).expect("ruby emit").source
}

/// Run emitted Ruby with a `ruby` interpreter if one is available; return its
/// stdout, or `None` to signal a skip.
fn run_ruby(source: &str) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir();
    // Unique per (process, call) so parallel tests never share a temp file.
    let path = dir.join(format!(
        "sir_ruby_test_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .ok()?
        .write_all(source.as_bytes())
        .ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok();
    let _ = std::fs::remove_file(&path);
    let out = out?;
    if !out.status.success() {
        panic!(
            "emitted ruby exited non-zero:\n{}\n--- source ---\n{source}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .replace("\r\n", "\n")
            .trim_end()
            .to_string(),
    )
}

// ── emit-shape (no interpreter needed) ──────────────────────────────────────

#[test]
fn arithmetic_precedence_shape() {
    let rb = ruby_to_ruby("puts 2 + 3 * 4");
    // Native operators, precedence preserved by parenthesisation.
    assert!(rb.contains("sir_puts((2 + (3 * 4)))"), "got:\n{rb}");
    assert!(rb.contains("def sir_user_main"), "renames main");
    assert!(rb.ends_with("sir_user_main\n"), "calls the entry:\n{rb}");
}

#[test]
fn if_is_a_native_expression() {
    let rb = ruby_to_ruby("def f(x)\n  if x > 0\n    10\n  else\n    0\n  end\nend\nputs f(3)");
    assert!(
        rb.contains("(if sir_truthy((x > 0)) then 10 else 0 end)"),
        "got:\n{rb}"
    );
}

#[test]
fn display_convention_follows_source_language() {
    // The mechanism: a module tagged source_language="ruby" renders booleans
    // the Ruby way (true/false); anything else keeps the Lisp #t/#f.  (The Ruby
    // frontend does not yet set the tag — a known cross-backend gap — so we set
    // it directly to exercise the backend's substitution.)
    let mut module = twig_to_semantic_ir::compile_source("(print 1)", "prog").unwrap();
    module.metadata.source_language = Some("ruby".into());
    assert!(compile(&module)
        .unwrap()
        .source
        .contains("SIR_DISPLAY_RUBY = true"));

    module.metadata.source_language = Some("twig".into());
    assert!(compile(&module)
        .unwrap()
        .source
        .contains("SIR_DISPLAY_RUBY = false"));
}

#[test]
fn deterministic_output() {
    let a = ruby_to_ruby("puts 2 + 3 * 4");
    let b = ruby_to_ruby("puts 2 + 3 * 4");
    assert_eq!(a, b, "emission must be byte-stable");
}

#[test]
fn string_hash_is_escaped_so_no_interpolation_can_fire() {
    // A literal `#` is escaped to `\#` so a crafted `#{...}` in source data can
    // never become a Ruby interpolation in the emitted literal.
    let rb = ruby_to_ruby(r##"puts "a#b""##);
    assert!(rb.contains("\"a\\#b\""), "the # should be escaped:\n{rb}");
    if let Some(out) = run_ruby(&rb) {
        assert_eq!(out, "a#b"); // and it still prints the literal text
    }
}

#[test]
fn sanitize_ident_handles_keywords_and_namespace() {
    assert_eq!(sanitize_ident("foo"), "foo");
    assert_eq!(sanitize_ident("end"), "end_"); // ruby keyword
    assert_eq!(sanitize_ident("class"), "class_");
    assert!(
        sanitize_ident("sir_x").starts_with("sir_x"),
        "runtime namespace guarded"
    );
    assert_eq!(sanitize_ident("Foo"), "_Foo"); // locals may not start uppercase
}

// ── end-to-end (skips when `ruby` is absent) ────────────────────────────────

#[test]
fn e2e_arithmetic() {
    let rb = ruby_to_ruby("puts 2 + 3 * 4");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "14"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_recursion_via_method() {
    let rb = ruby_to_ruby(
        "def add(a, b)\n  a + b\nend\ndef triple(n)\n  add(add(n, n), n)\nend\nputs triple(7)",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "21"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_twig_closure_and_globals() {
    let rb = twig_to_ruby(
        "(define (adder n) (lambda (x) (+ x n))) (define add5 (adder 5)) (print (add5 3))",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "8"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_tail_if_both_branches() {
    let rb = ruby_to_ruby(
        "def classify(n)\n  if n == 0\n    \"zero\"\n  elsif n < 0\n    \"neg\"\n  else\n    \"pos\"\n  end\nend\nputs classify(0)\nputs classify(-5)\nputs classify(7)",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "zero\nneg\npos"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR26 integer conversions (Expr::Convert) ───────────────────────────────

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, IntSpec, IntWidth, Metadata,
    Module, Overflow, Span, CURRENT_SIR_VERSION,
};

/// Build a module whose `main` prints `convert(to, IntLit(v))`, emit Ruby, run
/// it, and return stdout (or `None` when `ruby` is absent).
fn run_convert(v: i64, to: IntSpec) -> Option<String> {
    let inner = Expr::IntLit {
        value: v,
        span: Span::synthetic(),
    };
    let conv = Expr::Convert {
        value: Box::new(inner),
        to,
        span: Span::synthetic(),
    };
    let puts = Expr::BuiltinCall {
        name: "puts".into(),
        args: vec![conv],
        effects: EffectSet::PURE,
        span: Span::synthetic(),
    };
    let module = Module {
        name: "prog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Conversions,
            Feature::SizedIntegers,
            Feature::Unsigned,
            Feature::WrappingArithmetic,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: puts,
                span: Span::synthetic(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: Span::synthetic(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: Span::synthetic(),
    };
    let rb = compile(&module).expect("ruby emit").source;
    run_ruby(&rb)
}

fn sized(w: IntWidth, signed: bool) -> IntSpec {
    IntSpec::sized(w, signed, Overflow::Wrap)
}

#[test]
fn convert_emit_shape_picks_the_right_helper() {
    let inner = Expr::IntLit {
        value: 300,
        span: Span::synthetic(),
    };
    let m = Module {
        name: "prog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Conversions,
            Feature::SizedIntegers,
            Feature::Unsigned,
            Feature::WrappingArithmetic,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::Convert {
                    value: Box::new(inner),
                    to: sized(IntWidth::W8, false),
                    span: Span::synthetic(),
                },
                span: Span::synthetic(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: Span::synthetic(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: Span::synthetic(),
    };
    assert!(compile(&m).unwrap().source.contains("sir_u8(300)"));
}

#[test]
fn e2e_convert_u8_wraps() {
    // 300 mod 256 == 44 (the canonical uint8 overflow).
    match run_convert(300, sized(IntWidth::W8, false)) {
        Some(out) => assert_eq!(out, "44"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_convert_i32_overflow() {
    // (int32_t)4_000_000_000 == -294_967_296 under two's complement.
    match run_convert(4_000_000_000, sized(IntWidth::W32, true)) {
        Some(out) => assert_eq!(out, "-294967296"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_convert_u32_of_negative_one() {
    // (uint32_t)(-1) == 4_294_967_295.
    match run_convert(-1, sized(IntWidth::W32, false)) {
        Some(out) => assert_eq!(out, "4294967295"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_convert_i8_of_200() {
    // (int8_t)200 == -56.
    match run_convert(200, sized(IntWidth::W8, true)) {
        Some(out) => assert_eq!(out, "-56"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_convert_arbitrary_is_identity() {
    // A widen into the unbounded integer keeps the value exactly.
    match run_convert(4_000_000_000, IntSpec::arbitrary()) {
        Some(out) => assert_eq!(out, "4000000000"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR16 sequences (Expr::SeqLit) ──────────────────────────────────────────

#[test]
fn seq_literal_emits_a_native_ruby_array() {
    // `[1, 2, 3]` maps to a native array literal — no runtime helper, since
    // Ruby arrays ARE the value (unlike the Go/Rust tagged-value backends).
    let rb = ruby_to_ruby("puts([1, 2, 3])");
    assert!(rb.contains("[1, 2, 3]"), "SeqLit should be a native array:\n{rb}");
}

#[test]
fn e2e_seq_literal_displays_as_an_array() {
    // Ruby's `puts` of an array prints the array (`[1, 2, 3]`) — a
    // convention-independent check (no boolean display involved).
    let rb = ruby_to_ruby("puts([1, 2, 3])");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "[1, 2, 3]"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_seq_structural_equality_drives_control_flow() {
    // The point of sequences on this backend: `Array#==` is STRUCTURAL, so
    // `[1, 2] == [1, 2]` is true even for distinct array objects — matching
    // the Python/Go/Rust backends that carry sequences. Driven through an
    // `if` (printing a string) so the assertion does not depend on the
    // boolean display convention. Also exercises a NEGATIVE and a NESTED case.
    let rb = ruby_to_ruby(
        "if [1, 2] == [1, 2]\n  puts \"same\"\nelse\n  puts \"diff\"\nend\n\
         if [1, 2] == [1, 3]\n  puts \"same\"\nelse\n  puts \"diff\"\nend\n\
         if [[1, 2], [3]] == [[1, 2], [3]]\n  puts \"same\"\nelse\n  puts \"diff\"\nend",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "same\ndiff\nsame"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR16 sequences: HAND-BUILT modules (producer-agnostic) ─────────────────
//
// The bundled Ruby frontend cannot yet PRODUCE `SeqIndex`/`SeqLen`/`SeqSet`/
// `ForEach` (its parser drops `a[i]`, `.length` needs a deferred builtin, etc.),
// so a source-driven test would MASK them. But SIR is producer-agnostic — a
// C→SIR or Twig→SIR module can carry these nodes with a `{Sequences, Loops}`
// manifest that this backend now accepts. These tests build such modules
// directly and prove the emitter is TOTAL for the feature (no `unreachable!`)
// and matches the reference semantics.

use semantic_ir::{Effect, Scope, Stmt};

fn s2() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s2() }
}
fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s2() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s2() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args: vec![arg],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s2(),
        },
        span: s2(),
    }
}
/// A `main` module carrying the given statements, declaring Sequences + Loops.
fn seq_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "seqprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Loops]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s2() }, span: s2() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s2(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(CURRENT_SIR_VERSION),
        span: s2(),
    }
}
fn emit_seq_module(stmts: Vec<Stmt>) -> String {
    compile(&seq_module(stmts)).expect("seq module must compile, not panic").source
}

#[test]
fn seq_index_matches_ruby_bracket_semantics() {
    // `a[i]`: in-range, negative-from-end, and OOB→nil (never raises).
    let rb = emit_seq_module(vec![
        puts(Expr::SeqIndex { seq: Box::new(seq(vec![ilit(10), ilit(20), ilit(30)])), index: Box::new(ilit(1)), span: s2() }),
        puts(Expr::SeqIndex { seq: Box::new(seq(vec![ilit(10), ilit(20), ilit(30)])), index: Box::new(ilit(-1)), span: s2() }),
        puts(Expr::SeqIndex { seq: Box::new(seq(vec![ilit(10), ilit(20), ilit(30)])), index: Box::new(ilit(9)), span: s2() }),
    ]);
    assert!(rb.contains("[10, 20, 30])[1]"), "SeqIndex should be native []:\n{rb}");
    match run_ruby(&rb) {
        // `a[1]`→20, `a[-1]`→30 (from the end), `a[9]`→nil (OOB never raises).
        // `sir_puts(nil)` renders nil as "nil" in this backend's convention.
        Some(out) => assert_eq!(out, "20\n30\nnil"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn seq_len_is_native_length() {
    let rb = emit_seq_module(vec![puts(Expr::SeqLen {
        seq: Box::new(seq(vec![ilit(1), ilit(2), ilit(3)])),
        span: s2(),
    })]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "3"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn seq_set_writes_in_bounds_and_returns_the_value() {
    // `a = [1,2,3]; a[1] = 99; puts a` → `[1, 99, 3]`.
    let rb = emit_seq_module(vec![
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: seq(vec![ilit(1), ilit(2), ilit(3)]),
            span: s2(),
        },
        Stmt::SeqSet { seq: local("a"), index: ilit(1), value: ilit(99), span: s2() },
        puts(local("a")),
    ]);
    assert!(rb.contains("sir_seq_set(a, 1, 99)"), "SeqSet should use the guarded helper:\n{rb}");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "[1, 99, 3]"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn for_each_iterates_via_each_block() {
    // `for x in [1,2,3]: puts x` → `1\n2\n3`, emitted as `(…).each { |x| … }`
    // so the loop var is block-scoped (see `for_each_var_is_block_scoped…`).
    let rb = emit_seq_module(vec![Stmt::ForEach {
        var: "x".into(),
        iter: seq(vec![ilit(1), ilit(2), ilit(3)]),
        body: Block { stmts: vec![puts(local("x"))], value: Expr::NilLit { span: s2() }, span: s2() },
        span: s2(),
    }]);
    assert!(rb.contains("([1, 2, 3]).each do |x|"), "ForEach should be a .each block:\n{rb}");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "1\n2\n3"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn for_each_var_is_block_scoped_not_leaking() {
    // A loop var that SHADOWS an enclosing local must NOT clobber it — the
    // validator rewinds the loop var (body-scoped), and the Go backend
    // block-scopes it too. `x = 99; for x in [1,2,3]; end; puts x` → 99.
    let rb = emit_seq_module(vec![
        Stmt::LetBinding { name: "x".into(), sir_type: None, value: ilit(99), span: s2() },
        Stmt::ForEach {
            var: "x".into(),
            iter: seq(vec![ilit(1), ilit(2), ilit(3)]),
            body: Block { stmts: vec![], value: Expr::NilLit { span: s2() }, span: s2() },
            span: s2(),
        },
        puts(local("x")),
    ]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "99"), // block-scoped: enclosing x survives
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR16 ForRange (numeric `for` loop, gated by Loops) ─────────────────────
//
// ForRange requires ONLY `Feature::Loops` (accepted since 0.3.0), so it is
// reachable from the Ruby frontend (`for i in 0...3`) — yet the emitter used to
// send it to `unreachable!`, a pre-existing panic. Now desugared to a `while`
// matching the Go/Rust backends: evaluate-once bounds, direction-aware
// exclusive stop, the loop var leaking to the enclosing scope.

fn forrange(var: &str, start: i64, stop: i64, step: i64, body: Vec<Stmt>) -> Stmt {
    Stmt::ForRange {
        var: var.into(),
        start: ilit(start),
        stop: ilit(stop),
        step: ilit(step),
        body: Block { stmts: body, value: Expr::NilLit { span: s2() }, span: s2() },
        span: s2(),
    }
}

/// A ForRange module needs only Loops; build one so the manifest is minimal.
fn loops_module(stmts: Vec<Stmt>) -> Module {
    let mut m = seq_module(stmts);
    m.manifest = FeatureManifest::from_features(&[Feature::Loops]);
    m
}

#[test]
fn for_range_counts_up_exclusive() {
    // `for i in 0, 3, 1: puts i` → 0,1,2 (stop is exclusive).
    let rb = compile(&loops_module(vec![forrange("i", 0, 3, 1, vec![puts(local("i"))])]))
        .expect("ForRange must compile, not panic")
        .source;
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "0\n1\n2"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn for_range_counts_down_with_negative_step() {
    // `for i in 3, 0, -1: puts i` → 3,2,1 (descending, exclusive of 0).
    let rb = compile(&loops_module(vec![forrange("i", 3, 0, -1, vec![puts(local("i"))])]))
        .expect("ForRange must compile")
        .source;
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "3\n2\n1"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn for_range_nested_uses_distinct_temporaries() {
    // Nested range loops must not clobber each other's temporaries.
    let inner = forrange("j", 0, 2, 1, vec![puts(local("j"))]);
    let rb = compile(&loops_module(vec![forrange("i", 0, 2, 1, vec![inner])]))
        .expect("nested ForRange must compile")
        .source;
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "0\n1\n0\n1"), // i=0:{j0,j1}, i=1:{j0,j1}
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn unsupported_builtin_in_while_body_is_rejected_not_panicked() {
    // Gap B (security review): a `While` body was not scanned by the
    // unsupported-builtin pre-check, so an unknown builtin hidden there
    // survived validation and hit the emitter's `unreachable!`. It must now be
    // rejected CLEANLY (a `BackendError`), never panic.
    let bad_call = Expr::BuiltinCall {
        name: "totally_unsupported_xyz".into(),
        args: vec![],
        effects: EffectSet::PURE,
        span: s2(),
    };
    let while_stmt = Stmt::While {
        cond: Expr::BoolLit { value: false, span: s2() },
        body: Block {
            stmts: vec![Stmt::ExprStmt { expr: bad_call, span: s2() }],
            value: Expr::NilLit { span: s2() },
            span: s2(),
        },
        span: s2(),
    };
    // `compile` must return an Err (clean rejection), not unwind/panic.
    let result = compile(&loops_module(vec![while_stmt]));
    assert!(result.is_err(), "an unsupported builtin in a while body must be rejected cleanly");
}

// ── SIR16 maps: HAND-BUILT modules (producer-agnostic) ──────────────────────
//
// The Ruby frontend does not yet PRODUCE MapLit/MapGet/MapSet, so a source
// test would mask them. SIR is producer-agnostic — a C→SIR / Twig→SIR module
// can carry these with a `{Maps}` manifest this backend now accepts. Built
// directly; each proves the emitter is total and matches native Hash.

use semantic_ir::MapEntry;

fn strlit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s2() }
}
fn maplit(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries.into_iter().map(|(k, v)| MapEntry { key: k, value: v }).collect(),
        span: s2(),
    }
}
fn mapget(map: Expr, key: Expr) -> Expr {
    Expr::MapGet { map: Box::new(map), key: Box::new(key), span: s2() }
}
/// A `main` module declaring Maps + Sequences + Strings + Loops.
fn map_module(stmts: Vec<Stmt>) -> Module {
    let mut m = seq_module(stmts);
    m.manifest = FeatureManifest::from_features(&[
        Feature::Maps,
        Feature::Sequences,
        Feature::Strings,
        Feature::Loops,
    ]);
    m
}
fn run_map(stmts: Vec<Stmt>) -> Option<String> {
    run_ruby(&compile(&map_module(stmts)).expect("map module must compile, not panic").source)
}
fn let_(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s2() }
}

#[test]
fn map_get_reads_present_and_missing_keys() {
    // `h = {1 => 10, 2 => 20}; puts h[2]; puts h[9]` → 20, then nil (a missing
    // key yields nil, no raise — matching `_sir_map_get`).
    let out = run_map(vec![
        let_("h", maplit(vec![(ilit(1), ilit(10)), (ilit(2), ilit(20))])),
        puts(mapget(local("h"), ilit(2))),
        puts(mapget(local("h"), ilit(9))),
    ]);
    match out {
        Some(o) => assert_eq!(o, "20\nnil"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn map_set_inserts_and_updates_the_shared_hash() {
    // `h = {1 => 10}; h[2] = 20; h[1] = 99; puts h[1]; puts h[2]` → 99, 20.
    let out = run_map(vec![
        let_("h", maplit(vec![(ilit(1), ilit(10))])),
        Stmt::MapSet { map: local("h"), key: ilit(2), value: ilit(20), span: s2() },
        Stmt::MapSet { map: local("h"), key: ilit(1), value: ilit(99), span: s2() },
        puts(mapget(local("h"), ilit(1))),
        puts(mapget(local("h"), ilit(2))),
    ]);
    match out {
        Some(o) => assert_eq!(o, "99\n20"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn map_composite_key_is_structural() {
    // A composite key `[1, 2]` looks up by VALUE (Ruby Array#eql?/#hash are
    // structural), so a DISTINCT equal array finds the entry.
    let out = run_map(vec![
        let_("h", maplit(vec![(seq(vec![ilit(1), ilit(2)]), strlit("found"))])),
        puts(mapget(local("h"), seq(vec![ilit(1), ilit(2)]))),
    ]);
    match out {
        Some(o) => assert_eq!(o, "found"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn map_literal_emits_a_native_hash() {
    let rb = compile(&map_module(vec![puts(maplit(vec![(ilit(1), ilit(2))]))]))
        .expect("compile")
        .source;
    assert!(rb.contains("{1 => 2}"), "MapLit should be a native Hash literal:\n{rb}");
}

#[test]
fn for_each_over_a_map_does_not_panic() {
    // Accepting Maps makes `ForEach` over a Hash reachable (Loops + Maps). The
    // emitted `(h).each { |kv| … }` works on a Hash (yielding [k, v]) as well
    // as an Array — so the backend must not panic and the program must run.
    let m = map_module(vec![Stmt::ForEach {
        var: "kv".into(),
        iter: maplit(vec![(ilit(1), ilit(10))]),
        body: Block { stmts: vec![puts(local("kv"))], value: Expr::NilLit { span: s2() }, span: s2() },
        span: s2(),
    }]);
    // Must compile without an `unreachable!` panic.
    let rb = compile(&m).expect("ForEach over a map must compile, not panic").source;
    assert!(rb.contains(").each do |kv|"), "map ForEach is a .each block:\n{rb}");
    if let Some(out) = run_ruby(&rb) {
        assert!(!out.is_empty(), "the loop ran and printed the entry");
    }
}

// ── SIR16 floats ───────────────────────────────────────────────────────────
// `Feature::Floats` gates ONLY `Expr::FloatLit`. Ruby has a native `Float`, so
// the literal renders directly and arithmetic reuses the native operators. The
// Ruby FRONTEND masks this node (its parser would emit a `FloatLit` only from
// float SOURCE, which these tests don't go through), so — like the sequence and
// map tests above — we hand-build producer-agnostic modules and prove the
// emitter is total and the numbers/display match a real `ruby`.

fn flit(value: f64) -> Expr {
    Expr::FloatLit { value, span: s2() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args: vec![a, b],
        effects: EffectSet::PURE,
        span: s2(),
    }
}
/// A `main` module declaring only `Floats` (arithmetic/`puts` are builtins,
/// which are gated by the builtin allowlist, not by a feature).
fn float_module(stmts: Vec<Stmt>) -> Module {
    let mut m = seq_module(stmts);
    m.manifest = FeatureManifest::from_features(&[Feature::Floats]);
    m
}
fn run_float(stmts: Vec<Stmt>) -> Option<String> {
    run_ruby(&compile(&float_module(stmts)).expect("float module must compile, not panic").source)
}

#[test]
fn float_literal_emits_a_float_not_an_integer() {
    // THE core hazard: a naive `f64::to_string()` renders `7.0` as `"7"`, which
    // Ruby parses as an Integer (wrong type, wrong `/`, wrong display). The
    // emitted literal must carry a decimal point.
    let rb = compile(&float_module(vec![puts(flit(7.0))]))
        .expect("compile")
        .source;
    assert!(
        rb.contains("7.0"),
        "an integral FloatLit must emit `7.0`, not the Integer `7`:\n{rb}"
    );
}

#[test]
fn non_finite_literal_emits_a_named_constant() {
    // Ruby has no `inf`/`nan` numeric token — the values are `Float::INFINITY` /
    // `Float::NAN`. A `FloatLit` carrying one must still emit a parseable form.
    let rb = compile(&float_module(vec![
        puts(flit(f64::INFINITY)),
        puts(flit(f64::NEG_INFINITY)),
        puts(flit(f64::NAN)),
    ]))
    .expect("compile")
    .source;
    assert!(rb.contains("Float::INFINITY"), "positive infinity:\n{rb}");
    assert!(rb.contains("-Float::INFINITY"), "negative infinity:\n{rb}");
    assert!(rb.contains("Float::NAN"), "NaN:\n{rb}");
}

#[test]
fn float_literal_displays_with_a_trailing_point() {
    // `puts 7.0` → `7.0` (integral float keeps its `.0`), `puts 3.25` → `3.25`,
    // `puts(-0.0)` → `-0.0` (the sign of zero survives) — all via the runtime's
    // `sir_fmt_float`, matching a real Ruby.
    match run_float(vec![puts(flit(7.0)), puts(flit(3.25)), puts(flit(-0.0))]) {
        Some(out) => assert_eq!(out, "7.0\n3.25\n-0.0"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn float_arithmetic_is_native_and_exact() {
    // Float `+`/`-`/`*` reuse the native chained operators, so an integral
    // result stays a Float (`4.0`, not `4`) — matching every other backend.
    match run_float(vec![
        puts(bin("+", flit(1.5), flit(2.5))), // 4.0
        puts(bin("*", flit(2.0), flit(3.0))), // 6.0
        puts(bin("-", flit(7.0), flit(0.5))), // 6.5
    ]) {
        Some(out) => assert_eq!(out, "4.0\n6.0\n6.5"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn float_and_integer_division_follow_ruby() {
    // `7.0 / 2` true-divides to `3.5` (Float#/); `7 / 2` floors to `3`
    // (Integer#/) — the division frontier, preserved: a Float operand promotes,
    // two Integers floor. A regression guard that adding floats did not disturb
    // integer division.
    match run_float(vec![
        puts(bin("/", flit(7.0), ilit(2))), // 3.5
        puts(bin("/", flit(6.0), flit(2.0))), // 3.0
        puts(bin("/", ilit(7), ilit(2))),   // 3  (Integer floor)
    ]) {
        Some(out) => assert_eq!(out, "3.5\n3.0\n3"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn non_finite_arithmetic_displays_named() {
    // Float division by zero yields `Infinity`/`-Infinity` (NOT a
    // `ZeroDivisionError` — that is Integer-only), and `0.0/0.0` is `NaN` —
    // rendered by `sir_fmt_float`, matching a real Ruby.
    match run_float(vec![
        puts(bin("/", flit(1.0), flit(0.0))),  // Infinity
        puts(bin("/", flit(-1.0), flit(0.0))), // -Infinity
        puts(bin("/", flit(0.0), flit(0.0))),  // NaN
    ]) {
        Some(out) => assert_eq!(out, "Infinity\n-Infinity\nNaN"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn float_equality_is_value_based() {
    // `7.0 == 7.0` is true; a Float equals a numerically-equal Integer under
    // Ruby `==` (`7.0 == 7`), routed through `sir_eq`.
    match run_float(vec![
        puts(bin("=", flit(7.0), flit(7.0))), // #t
        puts(bin("=", flit(7.0), ilit(7))),   // #t (Ruby ==: 7.0 == 7)
        puts(bin("=", flit(7.0), flit(7.5))), // #f
    ]) {
        Some(out) => assert_eq!(out, "#t\n#t\n#f"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR16 short-circuit ─────────────────────────────────────────────────────
// `Feature::ShortCircuit` gates `Expr::LogicalAnd` / `Expr::LogicalOr`. Ruby's
// native `&&`/`||` ARE the SIR semantics (yield the deciding operand, skip the
// rhs when the lhs decides). The frontend CONSTANT-FOLDS `true && false`, so a
// `LogicalAnd` node only survives from a non-constant source — these tests
// hand-build the node directly to prove the emitter is total and short-circuits.

fn blit(value: bool) -> Expr {
    Expr::BoolLit { value, span: s2() }
}
fn nil_lit() -> Expr {
    Expr::NilLit { span: s2() }
}
fn land(lhs: Expr, rhs: Expr) -> Expr {
    Expr::LogicalAnd { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s2() }
}
fn lor(lhs: Expr, rhs: Expr) -> Expr {
    Expr::LogicalOr { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s2() }
}
/// A `main` module declaring only `ShortCircuit`.
fn sc_module(stmts: Vec<Stmt>) -> Module {
    let mut m = seq_module(stmts);
    m.manifest = FeatureManifest::from_features(&[Feature::ShortCircuit]);
    m
}
fn run_sc(stmts: Vec<Stmt>) -> Option<String> {
    run_ruby(&compile(&sc_module(stmts)).expect("short-circuit module must compile, not panic").source)
}

#[test]
fn logical_and_returns_the_deciding_operand() {
    // `a && b` is the OPERAND, not a bool: `1 && 2` → `2` (lhs truthy → rhs);
    // `false && 2` → `false` (lhs falsy → lhs); `nil && 2` → `nil`.
    match run_sc(vec![
        puts(land(ilit(1), ilit(2))),     // 2
        puts(land(blit(false), ilit(2))), // #f
        puts(land(nil_lit(), ilit(2))),   // nil
    ]) {
        Some(out) => assert_eq!(out, "2\n#f\nnil"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn logical_or_returns_the_deciding_operand() {
    // `a || b`: `1 || 2` → `1` (lhs truthy → lhs); `false || 5` → `5`
    // (lhs falsy → rhs); `nil || 7` → `7`.
    match run_sc(vec![
        puts(lor(ilit(1), ilit(2))),     // 1
        puts(lor(blit(false), ilit(5))), // 5
        puts(lor(nil_lit(), ilit(7))),   // 7
    ]) {
        Some(out) => assert_eq!(out, "1\n5\n7"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn short_circuit_does_not_evaluate_the_dead_operand() {
    // The RHS must NOT run when the LHS already decides. The dead operand here
    // is `1 / 0`, which RAISES `ZeroDivisionError` if evaluated — so a broken
    // (eager) lowering makes the emitted Ruby exit non-zero and `run_ruby`
    // panics. A correct short-circuit skips it: `false && (1/0)` → `false`,
    // `true || (1/0)` → `true`, both exit clean.
    let div_by_zero = || bin("/", ilit(1), ilit(0));
    match run_sc(vec![
        puts(land(blit(false), div_by_zero())), // #f, no raise
        puts(lor(blit(true), div_by_zero())),   // #t, no raise
    ]) {
        Some(out) => assert_eq!(out, "#f\n#t"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn short_circuit_emits_native_operators() {
    // Emit-shape: the nodes render as Ruby `&&` / `||` (native short-circuit).
    let rb = compile(&sc_module(vec![
        puts(land(ilit(1), ilit(2))),
        puts(lor(ilit(1), ilit(2))),
    ]))
    .expect("compile")
    .source;
    assert!(rb.contains("(1 && 2)"), "LogicalAnd → `&&`:\n{rb}");
    assert!(rb.contains("(1 || 2)"), "LogicalOr → `||`:\n{rb}");
}
