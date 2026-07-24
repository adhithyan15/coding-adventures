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

#[test]
fn sanitize_ident_flags_encoding_magic_constant() {
    // `__ENCODING__` is Ruby's third magic-constant keyword, alongside
    // `__FILE__`/`__LINE__` (which were already covered here) —
    // `__ENCODING__ = 5` is a SyntaxError under MRI, confirmed against
    // Ruby 3.4. It was missing from `is_ruby_keyword`'s list even though
    // its two siblings were already present.
    assert_eq!(sanitize_ident("__ENCODING__"), "__ENCODING___");

    // Ordinary identifiers — including close look-alikes — are
    // unaffected by the addition.
    assert_eq!(sanitize_ident("encoding"), "encoding");
    assert_eq!(sanitize_ident("__encoding__"), "__encoding__");
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

// ── SIR19 default parameters ────────────────────────────────────────────────
// `Feature::DefaultParams` — a positional parameter carrying a default
// expression renders as Ruby's native `def f(a, b = <default>)`. Ruby evaluates
// the default at call time when the argument is omitted (= the SIR semantics).
// A hand-built module (function with a defaulted param + a `main` that calls it
// with and without the trailing argument) proves the behaviour end to end.

use semantic_ir::{Param, ParamKind};

fn param(name: &str, default: Option<Expr>) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind: ParamKind::Required,
        default: default.map(Box::new),
        span: s2(),
    }
}
fn pref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s2() }
}
fn directcall(fn_name: &str, args: Vec<Expr>) -> Expr {
    Expr::DirectCall {
        fn_name: fn_name.into(),
        args,
        effects: EffectSet::PURE,
        span: s2(),
    }
}
/// A module with a helper function `f(<params>) = <body_value>` and a `main`
/// running `main_stmts`, declaring `DefaultParams`.
fn defparam_module(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Module {
    let f = Function {
        name: "f".into(),
        params,
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body_value, span: s2() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s2(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: main_stmts,
            value: Expr::NilLit { span: s2() },
            span: s2(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s2(),
    };
    Module {
        name: "defprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::DefaultParams,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![f, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(CURRENT_SIR_VERSION),
        span: s2(),
    }
}
fn run_defparam(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Option<String> {
    let m = defparam_module(params, body_value, main_stmts);
    run_ruby(&compile(&m).expect("default-param module must compile, not panic").source)
}

#[test]
fn default_param_is_used_when_the_argument_is_omitted() {
    // `def f(a, b = 5); a + b; end` — `f(1)` uses the default (`1 + 5 = 6`),
    // `f(1, 2)` overrides it (`1 + 2 = 3`).
    let body = bin("+", pref("a"), pref("b"));
    let out = run_defparam(
        vec![param("a", None), param("b", Some(ilit(5)))],
        body,
        vec![
            puts(directcall("f", vec![ilit(1)])),          // 6
            puts(directcall("f", vec![ilit(1), ilit(2)])), // 3
        ],
    );
    match out {
        Some(o) => assert_eq!(o, "6\n3"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn default_may_reference_an_earlier_parameter() {
    // `def f(a, b = a); b; end` — a default sees the parameters declared before
    // it (Ruby evaluates left to right), so `f(7)` yields `7`.
    let out = run_defparam(
        vec![param("a", None), param("b", Some(pref("a")))],
        pref("b"),
        vec![puts(directcall("f", vec![ilit(7)]))], // 7
    );
    match out {
        Some(o) => assert_eq!(o, "7"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn default_param_emits_native_ruby_syntax() {
    // Emit-shape: the signature carries the native `= <default>`.
    let m = defparam_module(
        vec![param("a", None), param("b", Some(ilit(5)))],
        bin("+", pref("a"), pref("b")),
        vec![],
    );
    let rb = compile(&m).expect("compile").source;
    assert!(rb.contains("def f(a, b = 5)"), "native default syntax:\n{rb}");
}

#[test]
fn unsupported_builtin_in_a_default_is_rejected_not_panicked() {
    // A default is an expression evaluated at call time, so an unsupported
    // builtin hidden in it must be caught by the pre-check (like a body), never
    // reach the emitter's `unreachable!`. `compile` must return an Err.
    let bad = Expr::BuiltinCall {
        name: "totally_unsupported_xyz".into(),
        args: vec![],
        effects: EffectSet::PURE,
        span: s2(),
    };
    let m = defparam_module(
        vec![param("a", None), param("b", Some(bad))],
        pref("a"),
        vec![],
    );
    let err = compile(&m).expect_err("an unsupported builtin in a default is rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn unsupported_builtin_in_an_indirect_call_target_is_rejected_cleanly() {
    // Defense in depth (security review): `scan_expr` now also scans an
    // `IndirectCall`'s TARGET, not just its args — `sir_apply(<target>, …)`
    // renders the target, so a deferred builtin in the callee position must not
    // reach the emitter's `unreachable!`. The invariant that matters is that
    // such a module (here, the bad builtin nested in a param default) is
    // rejected CLEANLY — whether by validation or the builtin gate — and never
    // panics `compile`.
    let bad_target = Expr::IndirectCall {
        target: Box::new(Expr::BuiltinCall {
            name: "totally_unsupported_xyz".into(),
            args: vec![],
            effects: EffectSet::PURE,
            span: s2(),
        }),
        args: vec![],
        effects: EffectSet::PURE,
        span: s2(),
    };
    let m = defparam_module(
        vec![param("a", None), param("b", Some(bad_target))],
        pref("a"),
        vec![],
    );
    assert!(
        compile(&m).is_err(),
        "a bad builtin in an IndirectCall target must be rejected, not panic"
    );
}

// ── SIR19 keyword parameters ────────────────────────────────────────────────
// `Feature::KeywordParams` — a keyword parameter (`def f(x:)` / `def f(x: 1)`)
// and a keyword argument (`f(x: 5)`) render as Ruby's NATIVE keyword forms;
// Ruby matches by name, so no positional resolution is needed (unlike Go/C).

fn kwparam(name: &str, default: Option<Expr>) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind: ParamKind::Keyword,
        default: default.map(Box::new),
        span: s2(),
    }
}
fn kwarg(name: &str, value: Expr) -> Expr {
    Expr::KeywordArg { name: name.into(), value: Box::new(value), span: s2() }
}
/// Like `defparam_module` but declaring `KeywordParams` (+ `DynamicTyping`).
fn kwparam_module(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Module {
    let mut m = defparam_module(params, body_value, main_stmts);
    m.manifest = FeatureManifest::from_features(&[
        Feature::KeywordParams,
        Feature::DynamicTyping,
    ]);
    m
}
fn run_kwparam(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Option<String> {
    let m = kwparam_module(params, body_value, main_stmts);
    run_ruby(&compile(&m).expect("keyword-param module must compile, not panic").source)
}

#[test]
fn keyword_argument_binds_to_the_keyword_parameter() {
    // `def f(x:); x; end` called `f(x: 5)` → `5`.
    let out = run_kwparam(
        vec![kwparam("x", None)],
        pref("x"),
        vec![puts(directcall("f", vec![kwarg("x", ilit(5))]))],
    );
    match out {
        Some(o) => assert_eq!(o, "5"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn keyword_arguments_resolve_by_name_regardless_of_order() {
    // `def f(a:, b:); a - b; end` called `f(b: 2, a: 10)` → `8` — a keyword
    // argument binds by NAME, so the call order does not matter (Ruby native).
    let out = run_kwparam(
        vec![kwparam("a", None), kwparam("b", None)],
        bin("-", pref("a"), pref("b")),
        vec![puts(directcall("f", vec![kwarg("b", ilit(2)), kwarg("a", ilit(10))]))],
    );
    match out {
        Some(o) => assert_eq!(o, "8"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn optional_keyword_uses_its_default_when_omitted() {
    // `def f(x: 7); x; end` — `f()` uses the default (`7`), `f(x: 9)` overrides
    // it (`9`). A keyword default is an OPTIONAL keyword (rides on
    // `KeywordParams`, not `DefaultParams`).
    let out = run_kwparam(
        vec![kwparam("x", Some(ilit(7)))],
        pref("x"),
        vec![
            puts(directcall("f", vec![])),                 // 7
            puts(directcall("f", vec![kwarg("x", ilit(9))])), // 9
        ],
    );
    match out {
        Some(o) => assert_eq!(o, "7\n9"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn keyword_param_and_arg_emit_native_ruby_syntax() {
    // Emit-shape: the signature carries `x:` and the call carries `x: 5`.
    let rb = compile(&kwparam_module(
        vec![kwparam("x", None)],
        pref("x"),
        vec![puts(directcall("f", vec![kwarg("x", ilit(5))]))],
    ))
    .expect("compile")
    .source;
    assert!(rb.contains("def f(x:)"), "keyword parameter syntax:\n{rb}");
    assert!(rb.contains("x: 5"), "keyword argument syntax:\n{rb}");
}

#[test]
fn rest_and_keyword_rest_params_emit_splat_syntax() {
    // Making the parameter emitter TOTAL: a `Rest` parameter renders `*rest`
    // and a `KwRest` renders `**opts` — native Ruby.  A `**opts` in particular
    // co-occurs with keyword parameters, so accepting `KeywordParams` must not
    // leave it mis-emitted as a bare name. (These kinds carry no feature of
    // their own, so the untyped-param `DynamicTyping` is the only manifest
    // requirement.)
    let splat = |kind: ParamKind, name: &str| Param {
        name: name.into(),
        sir_type: None,
        kind,
        default: None,
        span: s2(),
    };
    // `def f(a, *rest, x:, **opts)` — canonical order the validator enforces.
    let rb = compile(&kwparam_module(
        vec![
            param("a", None),
            splat(ParamKind::Rest, "rest"),
            kwparam("x", None),
            splat(ParamKind::KwRest, "opts"),
        ],
        pref("a"),
        vec![],
    ))
    .expect("compile")
    .source;
    assert!(rb.contains("def f(a, *rest, x:, **opts)"), "splat syntax:\n{rb}");
}

// ── SIR17 exceptions ────────────────────────────────────────────────────────
// `Feature::Exceptions` — `begin … rescue … ensure … end` (`Stmt::TryCatch`)
// and the `raise` / `retry` builtins render as Ruby's native exception
// handling. Hand-built modules prove the catch/ensure/binding behaviour through
// a real `ruby`.

use semantic_ir::RescueClause;

fn raise_(arg: Option<Expr>) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".into(),
            args: arg.into_iter().collect(),
            effects: EffectSet::PURE,
            span: s2(),
        },
        span: s2(),
    }
}
fn rescue_clause(types: Vec<&str>, binding: Option<&str>, body: Vec<Stmt>) -> RescueClause {
    RescueClause {
        exception_types: types.into_iter().map(String::from).collect(),
        binding: binding.map(String::from),
        body,
        span: s2(),
    }
}
fn trycatch(body: Vec<Stmt>, rescues: Vec<RescueClause>, ensure_body: Option<Vec<Stmt>>) -> Stmt {
    Stmt::TryCatch { body, rescues, ensure_body, span: s2() }
}
fn exc_module(stmts: Vec<Stmt>) -> Module {
    let mut m = seq_module(stmts);
    m.manifest = FeatureManifest::from_features(&[Feature::Exceptions, Feature::Strings]);
    m
}
fn run_exc(stmts: Vec<Stmt>) -> Option<String> {
    run_ruby(&compile(&exc_module(stmts)).expect("exception module must compile, not panic").source)
}

#[test]
fn bare_rescue_catches_a_raised_message() {
    // `begin; raise "boom"; rescue; puts "caught"; end` — the raised
    // `RuntimeError` is caught by the bare (catch-all) rescue.
    let out = run_exc(vec![trycatch(
        vec![raise_(Some(strlit("boom")))],
        vec![rescue_clause(vec![], None, vec![puts(strlit("caught"))])],
        None,
    )]);
    match out {
        Some(o) => assert_eq!(o, "caught"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn ensure_body_always_runs() {
    // `begin; puts "body"; ensure; puts "cleanup"; end` — no exception, but the
    // ensure still runs after the body.
    let out = run_exc(vec![trycatch(
        vec![puts(strlit("body"))],
        vec![],
        Some(vec![puts(strlit("cleanup"))]),
    )]);
    match out {
        Some(o) => assert_eq!(o, "body\ncleanup"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn rescue_binds_the_caught_exception() {
    // `begin; raise "x"; rescue => e; puts "got"; end` — the binding `e` is in
    // scope in the clause body (a `Scope::Local`); the clause runs.
    let out = run_exc(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec![], Some("e"), vec![puts(strlit("got"))])],
        None,
    )]);
    match out {
        Some(o) => assert_eq!(o, "got"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn rescue_matches_a_standard_exception_class() {
    // `begin; raise "x"; rescue StandardError; puts "std"; end` — a
    // `RuntimeError` is a `StandardError`, so the typed clause matches.
    let out = run_exc(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec!["StandardError"], None, vec![puts(strlit("std"))])],
        None,
    )]);
    match out {
        Some(o) => assert_eq!(o, "std"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn exception_emits_native_begin_rescue_ensure() {
    // Emit-shape: the native keywords are present.
    let rb = compile(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec!["StandardError"], Some("e"), vec![puts(strlit("c"))])],
        Some(vec![puts(strlit("f"))]),
    )]))
    .expect("compile")
    .source;
    assert!(rb.contains("begin"), "begin:\n{rb}");
    assert!(rb.contains("rescue StandardError => e"), "typed rescue + binding:\n{rb}");
    assert!(rb.contains("ensure"), "ensure:\n{rb}");
}

#[test]
fn injectable_rescue_type_is_rejected_cleanly() {
    // A rescue exception-type name is emitted verbatim as a constant reference,
    // so a name carrying source (a hand-built module could) must be rejected —
    // never emitted. `compile` returns an Err, not injectable Ruby.
    let m = exc_module(vec![trycatch(
        vec![puts(strlit("x"))],
        vec![rescue_clause(vec!["Foo; system('rm -rf /')"], None, vec![puts(strlit("y"))])],
        None,
    )]);
    let err = compile(&m).expect_err("an injectable rescue type must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn injectable_rescue_type_nested_in_a_call_argument_is_rejected() {
    // Regression (security review): a `TryCatch` hidden in an EXPRESSION position
    // (here a `Block` used as a `puts` argument) is still reached by the emitter,
    // so its rescue types must be validated too. The pre-emit scan is co-total
    // with the emitter — a hand-picked subset walk had missed this position.
    let payload = "StandardError\nSTDERR.puts('INJECTED')\nrescue";
    let bad_try = trycatch(
        vec![puts(strlit("x"))],
        vec![rescue_clause(vec![payload], None, vec![puts(strlit("y"))])],
        None,
    );
    let block_arg = Expr::Block(Box::new(Block {
        stmts: vec![bad_try],
        value: Expr::NilLit { span: s2() },
        span: s2(),
    }));
    let m = exc_module(vec![puts(block_arg)]);
    assert!(
        compile(&m).is_err(),
        "an injectable rescue type nested in a call argument must be rejected"
    );
}

#[test]
fn injectable_rescue_type_in_the_function_value_is_rejected() {
    // Regression (security review): a `TryCatch` in the function's trailing
    // VALUE (not its statement list) is emitted too, so the scan must visit
    // `f.body.value` — not only `f.body.stmts`.
    let bad_try = trycatch(
        vec![puts(strlit("x"))],
        vec![rescue_clause(vec!["Foo; system('boom')"], None, vec![puts(strlit("y"))])],
        None,
    );
    let mut m = exc_module(vec![]);
    if let Some(main) = m.functions.iter_mut().find(|f| f.name == "main") {
        main.body.value = Expr::Block(Box::new(Block {
            stmts: vec![bad_try],
            value: Expr::NilLit { span: s2() },
            span: s2(),
        }));
    }
    assert!(
        compile(&m).is_err(),
        "an injectable rescue type in the function value must be rejected"
    );
}

// ── OOP classes slice 1 + constants (Feature::Classes, Feature::Constants) ──
//
// The first OOP slice: an EMPTY base class (`class Foo; end`) plus construction
// (`Foo.new`), and the entangled `Constants` prerequisite (a class name IS a
// Ruby constant, so any `Foo.new` makes the frontend observe `Constants`).
//
// Neither a native `class Foo; end` block nor a bare `PI = 3` can be emitted:
// the frontend wraps a program's top-level code in `main`, and Ruby forbids
// BOTH a class definition and a constant assignment inside a method body. So a
// class / constant is defined REFLECTIVELY with `Object.const_set` — legal
// anywhere, executing in place — which still names the class (`Foo.name`).
//
// Positive cases go through the real Ruby frontend + interpreter (skipping when
// `ruby` is absent); rejection / injection cases are hand-built modules, since
// the frontend never PRODUCES an out-of-slice or injectable shape.

/// A `main` module carrying `stmts`, declaring Classes + Constants + Strings.
fn class_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "clsprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Classes,
            Feature::Constants,
            Feature::Strings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s2() }, span: s2() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new().with_source_language("test"),
            span: s2(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s2(),
    }
}
fn classdef(name: &str, superclass: Option<&str>, body: Vec<Stmt>) -> Stmt {
    Stmt::ClassDef {
        name: name.into(),
        superclass: superclass.map(Into::into),
        body,
        span: s2(),
    }
}
fn new_expr(class: &str, args: Vec<Expr>) -> Expr {
    let mut a = vec![strlit(class)];
    a.extend(args);
    Expr::BuiltinCall { name: "__new__".into(), args: a, effects: EffectSet::PURE, span: s2() }
}
fn const_assign(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Const, value, span: s2() }
}
fn const_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Const, span: s2() }
}
fn let_bind(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s2() }
}

// ---- positive, through the real frontend + interpreter --------------------

#[test]
fn e2e_empty_class_declaration_and_construction() {
    // `class Foo; end; x = Foo.new; puts "ok"` runs cleanly and prints "ok".
    // Proves the empty class emits, is instantiable, and the program is valid
    // Ruby (a `class`/`const` inside `main` would otherwise be a SyntaxError).
    let rb = ruby_to_ruby("class Foo\nend\nx = Foo.new\nputs \"ok\"\n");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "ok"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn empty_class_emits_reflective_const_set() {
    // The empty class becomes `Object.const_set(:Foo, Class.new)` and the
    // construction a native `Foo.new(...)` — NOT a native `class Foo` block
    // (illegal inside the `main` method the frontend wraps top-level code in).
    let rb = ruby_to_ruby("class Foo\nend\nx = Foo.new\n");
    assert!(
        rb.contains("Object.const_set(:Foo, Class.new)"),
        "empty class → reflective const_set:\n{rb}"
    );
    assert!(rb.contains("Foo.new"), "construction → native .new:\n{rb}");
    assert!(!rb.contains("class Foo\n"), "must NOT emit a native class block:\n{rb}");
}

#[test]
fn e2e_constant_definition_and_reference() {
    // `PI = 3; puts PI` prints "3" — the constant is defined reflectively and
    // the bare reference resolves at runtime.  (Constants rides in with Classes.)
    let rb = ruby_to_ruby("PI = 3\nputs PI\n");
    assert!(
        rb.contains("Object.const_set(:PI, 3)"),
        "constant → reflective const_set:\n{rb}"
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "3"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_constant_used_in_an_expression() {
    // A constant reference participates in arithmetic: `N = 5; puts N + 1` → 6.
    let rb = ruby_to_ruby("N = 5\nputs N + 1\n");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "6"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn raise_of_a_constant_exception_class_now_compiles() {
    // Accepting `Constants` also unblocks `raise SomeClass` (a specific
    // exception class is a `Const` reference) — a form the exceptions slice
    // deferred precisely because `Constants` was then unaccepted.  A hand-built
    // module `raise(ArgumentError)` (producer-agnostic — the bundled frontend
    // lowers a bare `raise Foo` as a call, not a const ref) now compiles, and
    // the const-referenced class is emitted as a bare Ruby constant.
    let raise_const = Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".into(),
            args: vec![const_ref("ArgumentError")],
            effects: EffectSet::PURE,
            span: s2(),
        },
        span: s2(),
    };
    let rb = compile(&class_module(vec![raise_const]))
        .expect("raise of a const exception class must compile")
        .source;
    assert!(rb.contains("raise(ArgumentError)"), "raise a bare constant class:\n{rb}");
}

// ---- hand-built: totality (deferred shapes rejected, never panicked) ------

#[test]
fn a_class_with_a_superclass_is_rejected_cleanly() {
    // Inheritance is a later slice; a `class Foo < Bar` is rejected, not emitted.
    let m = class_module(vec![classdef("Foo", Some("Bar"), vec![])]);
    let err = compile(&m).expect_err("a superclass must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn a_non_empty_class_body_is_rejected_cleanly() {
    // Class-level code / constants are a later slice; a non-empty body rejects.
    let m = class_module(vec![classdef("Foo", None, vec![puts(strlit("hi"))])]);
    assert!(compile(&m).is_err(), "a non-empty class body must be rejected");
}

#[test]
fn a_singleton_class_is_rejected_cleanly() {
    // Regression (security review): `Stmt::SingletonClassDef` (`class << self`)
    // ALSO observes `Feature::Classes` in the validator, so accepting `Classes`
    // obligates handling it — a hand-built module carrying one must be rejected
    // cleanly, NOT reach the emitter's `unreachable!` (a DoS on a
    // producer-agnostic module).
    let singleton = Stmt::SingletonClassDef {
        target: "self".into(),
        body: vec![],
        span: s2(),
    };
    let err = compile(&class_module(vec![singleton]))
        .expect_err("a singleton class must be rejected, not panic");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn a_namespaced_class_name_is_rejected_cleanly() {
    // `const_set` names a constant in one namespace; a `Foo::Bar` class name is
    // deferred (not injectable — a valid path — but not yet supported).
    let m = class_module(vec![classdef("Foo::Bar", None, vec![])]);
    assert!(compile(&m).is_err(), "a namespaced class name must be rejected");
}

#[test]
fn a_malformed_def_method_missing_its_closure_is_rejected_cleanly() {
    // Totality: `__def_method__` is now supported (slice 2), but the emitter
    // renders its closure argument (`args[2]`); a malformed registration missing
    // it must be rejected in the scan, so the emitter never indexes past the end.
    let def_method = Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__def_method__".into(),
            args: vec![strlit("Foo"), strlit("greet")], // no closure
            effects: EffectSet::PURE,
            span: s2(),
        },
        span: s2(),
    };
    let m = class_module(vec![classdef("Foo", None, vec![]), def_method]);
    let err = compile(&m).expect_err("a __def_method__ with no closure must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);

    // Likewise a NON-closure third argument (which would emit `&(5)` and fail at
    // Ruby runtime) is rejected at compile time, not mis-emitted.
    let non_closure = Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__def_method__".into(),
            args: vec![strlit("Foo"), strlit("greet"), ilit(5)],
            effects: EffectSet::PURE,
            span: s2(),
        },
        span: s2(),
    };
    assert!(
        compile(&class_module(vec![classdef("Foo", None, vec![]), non_closure])).is_err(),
        "a __def_method__ whose third arg is not a closure must be rejected"
    );
}

// ---- hand-built: injection (a crafted constant name cannot inject) --------

#[test]
fn an_injectable_class_name_is_rejected() {
    // A `ClassDef` name is emitted into a `const_set` symbol / would name a
    // class; a metacharacter-bearing name must be rejected, never emitted.
    let m = class_module(vec![classdef("Foo\n  system('boom')", None, vec![])]);
    let err = compile(&m).expect_err("an injectable class name must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn an_injectable_new_class_name_is_rejected() {
    // `__new__`'s class-name argument is emitted VERBATIM as the `.new` receiver
    // — a crafted name (even with no matching ClassDef) must be rejected.
    let m = class_module(vec![let_bind("x", new_expr("Foo.new; system('boom')", vec![]))]);
    assert!(compile(&m).is_err(), "an injectable __new__ class name must be rejected");
}

#[test]
fn an_injectable_constant_reference_is_rejected() {
    // A `Scope::Const` reference is emitted verbatim as a Ruby constant, so a
    // crafted name must be rejected before it can inject source.
    let m = class_module(vec![puts(const_ref("PI; system('boom')"))]);
    assert!(compile(&m).is_err(), "an injectable const reference must be rejected");
}

#[test]
fn an_injectable_constant_assignment_target_is_rejected() {
    // A `Scope::Const` assignment target is emitted into a `const_set` symbol; a
    // crafted target must be rejected.
    let m = class_module(vec![const_assign("PI\n=1; system('x')", ilit(1))]);
    assert!(compile(&m).is_err(), "an injectable const assignment must be rejected");
}

// ── OOP classes slice 2 — instance methods (define_method / public_send) ────
//
// A method-bearing class lowers to a HOISTED top-level function `Class__method`,
// a `__def_method__("Class", "method", MakeClosure(fn))` registration, and a
// `__method__(recv, "method", args…)` dispatch. This slice renders the
// registration as `Class.define_method(:sir_um_method, &closure)` and the
// dispatch as `(recv).public_send(:sir_um_method, args…)`.
//
// The reserved `sir_um_` method-name PREFIX is the anti-RCE guarantee: no
// reflection/eval built-in is named `sir_um_*`, so `public_send` with a crafted
// name can NEVER reach `instance_eval` / `send` / etc. — it can only reach a
// method installed by `__def_method__`.  A `__method__` to an UNregistered name
// (a built-in method call like `.upcase`) is rejected cleanly (Collections batch).

fn method_module(stmts: Vec<Stmt>) -> Module {
    method_module_fns(stmts, vec![])
}
/// A `main` module carrying `stmts`, plus `extra` hoisted method functions (a
/// `MakeClosure` in a `__def_method__` references one by name, and the validator
/// requires the target to exist).
fn method_module_fns(stmts: Vec<Stmt>, extra: Vec<Function>) -> Module {
    let mut functions = vec![Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s2() }, span: s2() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new().with_source_language("test"),
        span: s2(),
    }];
    functions.extend(extra);
    Module {
        name: "methprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Classes,
            Feature::Constants,
            Feature::Strings,
            Feature::Closures,
        ]),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s2(),
    }
}
/// A minimal hoisted method function (no params, `nil` body) — enough for the
/// validator to resolve a `MakeClosure` that names it.
fn hoisted(name: &str) -> Function {
    Function {
        name: name.into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: Expr::NilLit { span: s2() }, span: s2() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s2(),
    }
}
fn make_closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s2() }
}
fn def_method(class: &str, method: &str, fn_name: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__def_method__".into(),
            args: vec![strlit(class), strlit(method), make_closure(fn_name)],
            effects: EffectSet::PURE,
            span: s2(),
        },
        span: s2(),
    }
}
fn method_call(recv: Expr, method: &str, args: Vec<Expr>) -> Expr {
    let mut a = vec![recv, strlit(method)];
    a.extend(args);
    Expr::BuiltinCall { name: "__method__".into(), args: a, effects: EffectSet::PURE, span: s2() }
}

// ---- positive, through the real frontend + interpreter --------------------

#[test]
fn e2e_instance_method_call() {
    // `class Greeter; def greet; puts "hi"; end; end; Greeter.new.greet` → "hi".
    let rb = ruby_to_ruby("class Greeter\n  def greet\n    puts \"hi\"\n  end\nend\ng = Greeter.new\ng.greet\n");
    assert!(rb.contains("define_method(:sir_um_greet"), "prefixed registration:\n{rb}");
    assert!(rb.contains("public_send(:sir_um_greet"), "prefixed dispatch:\n{rb}");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "hi"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_instance_method_with_args_and_return() {
    // A method with parameters and a return value: `add(a, b) = a + b`.
    let rb = ruby_to_ruby("class Adder\n  def add(a, b)\n    a + b\n  end\nend\nputs Adder.new.add(2, 3)\n");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "5"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ---- hand-built: anti-RCE (the sir_um_ prefix closes reflection dispatch) --

#[test]
fn dispatch_is_prefixed_so_reflection_is_unreachable() {
    // Even when the method name coincides with a dangerous reflection built-in
    // (`instance_eval`), the `sir_um_` prefix means the emitted `public_send`
    // targets `:sir_um_instance_eval` — a name no built-in has — so Ruby's real
    // `instance_eval` is UNREACHABLE.  (The method is registered so it passes the
    // allowlist; the prefix is what neutralises the RCE.)
    let recv = new_expr("Foo", vec![]);
    let m = method_module_fns(
        vec![
            classdef("Foo", None, vec![]),
            def_method("Foo", "instance_eval", "Foo__x"),
            puts(method_call(recv, "instance_eval", vec![strlit("system('boom')")])),
        ],
        vec![hoisted("Foo__x")],
    );
    let rb = compile(&m).expect("registered dispatch compiles").source;
    assert!(rb.contains(":sir_um_instance_eval"), "prefixed symbol:\n{rb}");
    assert!(!rb.contains(".instance_eval("), "must NOT emit a bare instance_eval call:\n{rb}");
    assert!(
        !rb.contains("public_send(:instance_eval"),
        "must NOT dispatch to the unprefixed reflection name:\n{rb}"
    );
}

// ---- hand-built: totality / clean rejection -------------------------------

#[test]
fn dispatch_to_an_unregistered_method_is_rejected_cleanly() {
    // `__method__(recv, "upcase")` with no `__def_method__` registering `upcase`
    // is a BUILT-IN method call (Collections batch) — rejected cleanly, so it
    // does NOT compile-then-`NoMethodError` at runtime (`sir_um_upcase` is unbound).
    let recv = new_expr("Foo", vec![]);
    let m = method_module(vec![
        classdef("Foo", None, vec![]),
        puts(method_call(recv, "upcase", vec![])),
    ]);
    let err = compile(&m).expect_err("a built-in method dispatch must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn a_registered_method_dispatches_across_the_whole_module() {
    // The allowlist is module-wide: a method registered anywhere lets its
    // dispatch compile (the receiver need not be statically the defining class).
    let m = method_module_fns(
        vec![
            classdef("Foo", None, vec![]),
            def_method("Foo", "greet", "Foo__greet"),
            puts(method_call(new_expr("Foo", vec![]), "greet", vec![])),
        ],
        vec![hoisted("Foo__greet")],
    );
    assert!(compile(&m).is_ok(), "a registered-method dispatch must compile");
}

#[test]
fn an_injectable_def_method_class_name_is_rejected() {
    // `__def_method__`'s class name is emitted as the bare `const_set`/receiver
    // constant, so a crafted class name must be rejected (injection guard).
    let m = method_module_fns(
        vec![def_method("Foo\n  system('x')", "greet", "Foo__greet")],
        vec![hoisted("Foo__greet")],
    );
    assert!(compile(&m).is_err(), "an injectable def_method class name must be rejected");
}

#[test]
fn still_rejected_super_self_classmethod_builtins() {
    // The remaining OOP builtins are later slices — a hand-built module carrying
    // any is rejected cleanly (never `unreachable!`).  (`__self__` landed in
    // slice 3, so it is no longer in this list.)
    for bad in ["__super__", "__class_method__", "__def_class_method__"] {
        let call = Expr::BuiltinCall {
            name: bad.into(),
            args: vec![strlit("Foo"), strlit("m")],
            effects: EffectSet::PURE,
            span: s2(),
        };
        let m = method_module(vec![puts(call)]);
        assert!(compile(&m).is_err(), "`{bad}` must still be rejected");
    }
}

// ── OOP classes slice 3 — instance variables (@ivars) + self ────────────────
//
// `@v = x` / `@v` lower to a `Scope::Instance` `Assign` / `VarRef` whose `name`
// already includes the `@`. They render as native `@v = x` / `@v` (the name
// emitted verbatim, validated as `@<identifier>` by the scan — no injection).
// Instance-method bodies are installed with `define_method` (slice 2), which
// binds `self` to the receiver, so `@v` inside a method reads/writes the
// instance's own variable with no runtime plumbing. A bare `self` (`__self__`)
// renders the native `self`.

/// A `main` module carrying `stmts`, declaring InstanceVars (+ Strings) — for
/// hand-built ivar shapes (the frontend only produces `@v` inside a method body).
fn ivar_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "ivprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::InstanceVars,
            Feature::MutableBindings,
            Feature::Strings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s2() }, span: s2() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new().with_source_language("test"),
            span: s2(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s2(),
    }
}
fn ivar_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Instance, span: s2() }
}
fn ivar_assign(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Instance, value, span: s2() }
}

// ---- positive, through the real frontend + interpreter --------------------

#[test]
fn e2e_instance_variable_set_and_get() {
    // `class Box; def set(v); @v=v; end; def get; @v; end; end` — a set writes
    // the instance's `@v`, a later get reads it back through `self`.
    let rb = ruby_to_ruby(
        "class Box\n  def set(v)\n    @v = v\n  end\n  def get\n    @v\n  end\nend\n\
         b = Box.new\nb.set(7)\nputs b.get\n",
    );
    assert!(rb.contains("@v = "), "native ivar write:\n{rb}");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "7"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_instance_variable_mutation_across_calls() {
    // A counter mutating `@n` across method calls — proves the ivar persists on
    // the instance between dispatches, and `@n = @n + 1` reads then writes it.
    let rb = ruby_to_ruby(
        "class Counter\n  def start\n    @n = 0\n  end\n  def inc\n    @n = @n + 1\n  end\n  \
         def value\n    @n\n  end\nend\n\
         c = Counter.new\nc.start\nc.inc\nc.inc\nputs c.value\n",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "2"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ---- hand-built: emit shape + self ----------------------------------------

#[test]
fn ivar_read_and_write_emit_verbatim() {
    // `@v = 1` then a read of `@v` render as native `@v` — the leading `@` is
    // preserved (NOT routed through `sanitize_ident`, which would mangle it).
    let m = ivar_module(vec![ivar_assign("@v", ilit(1)), puts(ivar_ref("@v"))]);
    let rb = compile(&m).expect("ivar module compiles").source;
    assert!(rb.contains("@v = 1"), "ivar write verbatim:\n{rb}");
    assert!(rb.contains("sir_puts(@v)"), "ivar read verbatim:\n{rb}");
}

#[test]
fn self_builtin_emits_native_self() {
    // `__self__` (a bare `self`) renders the native `self` keyword.
    let self_call = Expr::BuiltinCall {
        name: "__self__".into(),
        args: vec![],
        effects: EffectSet::PURE,
        span: s2(),
    };
    let m = ivar_module(vec![puts(self_call)]);
    let rb = compile(&m).expect("self module compiles").source;
    assert!(rb.contains("sir_puts(self)"), "native self:\n{rb}");
}

// ---- hand-built: injection (a crafted ivar name cannot inject) ------------

#[test]
fn an_injectable_ivar_write_name_is_rejected() {
    // A `Scope::Instance` assignment target is emitted verbatim, so a crafted
    // name must be rejected before it can inject source.
    let m = ivar_module(vec![ivar_assign("@v = 1; system('boom')", ilit(1))]);
    let err = compile(&m).expect_err("an injectable ivar write must be rejected");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}

#[test]
fn an_injectable_ivar_read_name_is_rejected() {
    // Likewise an ivar REFERENCE name.
    let m = ivar_module(vec![puts(ivar_ref("@v; system('boom')"))]);
    assert!(compile(&m).is_err(), "an injectable ivar read must be rejected");
}

#[test]
fn a_non_at_ivar_name_is_rejected() {
    // A `Scope::Instance` name that does not start with `@` (or is just `@`) is
    // malformed — rejected, never emitted as a bare identifier.
    assert!(compile(&ivar_module(vec![ivar_assign("v", ilit(1))])).is_err(), "no @");
    assert!(compile(&ivar_module(vec![ivar_assign("@", ilit(1))])).is_err(), "bare @");
    assert!(compile(&ivar_module(vec![ivar_assign("@1x", ilit(1))])).is_err(), "@ + digit");
}
