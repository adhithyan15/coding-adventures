//! Emit-shape tests — assert the *text* of the generated C.  These run with no
//! C compiler present (unlike `compile_and_run.rs`), so they always execute in
//! CI and locally.

use semantic_ir::BackendErrorKind;
use semantic_ir_to_c::{compile, sanitize_ident};

fn lower_twig(src: &str) -> semantic_ir::Module {
    twig_to_semantic_ir::compile_source(src, "prog").expect("twig lowering")
}

fn lower_ruby(src: &str) -> semantic_ir::Module {
    ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering")
}

#[test]
fn arithmetic_emits_variadic_helpers() {
    let m = lower_twig("(print (+ 2 (* 3 4)))");
    let src = compile(&m).unwrap().source;
    assert!(src.contains("int main(void)"), "has a C entry point");
    assert!(src.contains("SirValue user_main(void)"), "renames SIR main");
    // Variadic-shaped builtin calls carry an argument count.
    assert!(
        src.contains("_sir_times(2, _sir_int(3LL), _sir_int(4LL))"),
        "\n{src}"
    );
    assert!(src.contains("_sir_print(1,"), "print is variadic");
}

#[test]
fn if_in_tail_position_is_a_returning_if_else() {
    // A trailing `if` becomes the function's implicit return.
    let m = lower_ruby("def f(x)\n  if x > 0\n    10\n  else\n    0\n  end\nend\nputs f(3)");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("if (_sir_truthy("),
        "guards on SIR truthiness\n{src}"
    );
    assert!(
        src.contains("return _sir_int(10LL);"),
        "returns the then-branch\n{src}"
    );
    assert!(
        src.contains("return _sir_int(0LL);"),
        "returns the else-branch\n{src}"
    );
}

#[test]
fn closures_emit_make_closure_and_a_thunk() {
    let m =
        lower_twig("(define (adder n) (lambda (x) (+ x n))) (define a (adder 5)) (print (a 3))");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_make_closure(_sir_thunk_"),
        "builds a closure\n{src}"
    );
    assert!(
        src.contains("static SirValue _sir_thunk_"),
        "declares a thunk\n{src}"
    );
    assert!(
        src.contains("_sir_apply("),
        "an indirect call applies the closure\n{src}"
    );
}

#[test]
fn display_convention_follows_source_language() {
    // Twig tags its source language, so it selects the Lisp `#t`/`#f` rendering.
    let twig = compile(&lower_twig("(print #t)")).unwrap().source;
    assert!(
        twig.contains("#define SIR_DISPLAY_RUBY 0"),
        "twig → #t/#f display"
    );

    // A module tagged `ruby` selects the `true`/`false` rendering.  (The Ruby
    // frontend does not yet set `source_language`, a pre-existing gap shared by
    // every backend; the backend substitution itself is what we verify here.)
    let mut m = lower_twig("(print #t)");
    m.metadata.source_language = Some("ruby".to_string());
    let ruby = compile(&m).unwrap().source;
    assert!(
        ruby.contains("#define SIR_DISPLAY_RUBY 1"),
        "ruby → true/false display"
    );
}

#[test]
fn output_is_deterministic() {
    let m =
        lower_twig("(define (adder n) (lambda (x) (+ x n))) (define a (adder 5)) (print (a 3))");
    assert_eq!(compile(&m).unwrap().source, compile(&m).unwrap().source);
}

#[test]
fn self_contained_no_external_headers() {
    let src = compile(&lower_twig("(print 1)")).unwrap().source;
    // Only the C standard library — no project or third-party include.
    for line in src
        .lines()
        .filter(|l| l.trim_start().starts_with("#include"))
    {
        assert!(
            line.contains('<') && line.contains('>'),
            "only <system> headers: {line}"
        );
    }
    assert!(
        src.contains("#define _CRT_SECURE_NO_WARNINGS"),
        "silences MSVC CRT deprecations"
    );
}

#[test]
fn unaccepted_feature_is_rejected_cleanly() {
    // An array literal declares Feature::Sequences, which this backend does not
    // accept (loops and mutation are now accepted, so a `while` no longer
    // exercises the rejection path — an array still does).
    let m = lower_ruby("puts [1, 2, 3]");
    let err = compile(&m).expect_err("backend rejects Sequences");
    assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
}

#[test]
fn string_literals_are_trigraph_safe() {
    // A source string containing `??/` must not let the trigraph expansion
    // (`??/` → `\` in C translation phase 1, which GCC enables under
    // `-std=c99`) break out of the emitted C literal.  Each `?` is escaped as
    // `\?`, so no `??` pair survives.
    let m = lower_ruby("puts \"a??/b?\"");
    let src = compile(&m).unwrap().source;
    assert!(
        !src.contains("??"),
        "no `??` pair (a trigraph prefix) survives in the emitted C:\n{src}"
    );
    assert!(src.contains("\\?"), "question marks are escaped:\n{src}");
}

#[test]
fn method_dispatch_is_rejected_in_v0() {
    // `"hi".length` lowers to the `__method__` collection-dispatch builtin.
    // It is not gated by an unaccepted *feature* (its receiver is a String,
    // which v0 accepts), so the structural builtin gate must reject it rather
    // than emit a call that fails at runtime.
    let m = lower_ruby("puts \"hi\".length");
    let err = compile(&m).expect_err("v0 rejects __method__ dispatch");
    assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    assert!(
        err.message.contains("__method__"),
        "names the builtin: {}",
        err.message
    );
}

#[test]
fn sanitize_ident_maps_into_c() {
    assert_eq!(sanitize_ident("foo"), "foo");
    assert_eq!(sanitize_ident("foo_bar1"), "foo_bar1");
    // C keywords get a trailing underscore.
    assert_eq!(sanitize_ident("int"), "int_");
    assert_eq!(sanitize_ident("return"), "return_");
    // Non-identifier characters are escaped, never passed through.
    assert!(sanitize_ident("a-b").starts_with('a'));
    assert!(sanitize_ident("a-b").contains("_u"));
    // The runtime namespace is kept clear.
    assert!(sanitize_ident("_sir_plus").ends_with('_'));
    // A leading digit is escaped so the result is a valid C identifier.
    let s = sanitize_ident("1x");
    assert!(!s.starts_with(|c: char| c.is_ascii_digit()));
}
