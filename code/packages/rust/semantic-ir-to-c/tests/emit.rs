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
    // The whole OOP surface — classes, instance/class methods, `@`/`@@` vars,
    // inheritance, `super`, and now modules (`include`/`extend`) — is accepted, so
    // those no longer exercise the rejection path.  A `class << self` singleton
    // (`Stmt::SingletonClassDef`) is the nearest still-deferred OOP construct: it
    // observes the accepted `Feature::Classes`, so the capability check passes and
    // the STRUCTURAL scan rejects it cleanly (rather than reaching an emitter
    // `unreachable!`).
    let m = lower_ruby("class << self\n  def x\n    1\n  end\nend");
    let err = compile(&m).expect_err("backend rejects a singleton class");
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
fn builtin_method_dispatch_is_rejected() {
    // A built-in method NOT in the Collections slice yet (`strip`) lowers to a
    // `__method__` dispatch the module never registers via `__def_method__`, so
    // the allowlist rejects it cleanly (rather than emitting a call that
    // `NoMethodError`s at runtime).  (`length`/`upcase`/… ARE lowered now — see
    // `collection_string_methods_route_to_the_builtin_dispatcher` and the
    // `compile_and_run_string_methods` execution proof.)
    let m = lower_ruby("puts \"hi\".strip");
    let err = compile(&m).expect_err("rejects a not-yet-lowered built-in method dispatch");
    assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    assert!(
        err.message.contains("strip"),
        "names the built-in method: {}",
        err.message
    );
}

#[test]
fn instance_vars_route_through_ivar_helpers() {
    // OOP slice 3: `@v = v` is a `Scope::Instance` Assign → `_sir_ivar_set`, and
    // `@v` a `Scope::Instance` VarRef → `_sir_ivar_get`.  The `@`-name is a QUOTED
    // C string literal (no name injection), and the runtime carries the helpers.
    let m = lower_ruby("class Box\n  def set(v)\n    @v = v\n  end\n  def get\n    @v\n  end\nend");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_ivar_set(\"@v\","),
        "an `@v =` write calls the setter with the quoted `@`-name\n{src}"
    );
    assert!(
        src.contains("_sir_ivar_get(\"@v\")"),
        "an `@v` read calls the getter with the quoted `@`-name\n{src}"
    );
    // The runtime defines the helpers + the current-self machinery they read.
    assert!(src.contains("_sir_ivar_get"), "runtime declares the getter");
    assert!(src.contains("_sir_ivar_set"), "runtime declares the setter");
    assert!(
        src.contains("_sir_current_self"),
        "dispatch binds the receiver into `_sir_current_self`\n{src}"
    );
}

#[test]
fn explicit_self_renders_as_sir_self() {
    // A bare `self` (`__self__`) renders as the `_sir_self()` accessor, so a method
    // can return the receiver for chaining.
    let m = lower_ruby("class Widget\n  def me\n    self\n  end\nend");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_self()"),
        "`self` lowers to the `_sir_self()` accessor\n{src}"
    );
}

#[test]
fn subclass_registers_its_superclass_edge() {
    // OOP slice 4: `class Dog < Animal` emits a `_sir_register_super` edge (both
    // names QUOTED C string literals — no injection), and the runtime carries the
    // ancestry-walking resolver + `super` dispatcher.
    let m = lower_ruby("class Animal\n  def legs\n    4\n  end\nend\nclass Dog < Animal\nend\nputs Dog.new.legs");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_register_super(\"Dog\", \"Animal\")"),
        "a subclass registers its `sub -> super` edge\n{src}"
    );
    assert!(
        src.contains("_sir_resolve_method"),
        "the runtime resolves methods up the ancestry\n{src}"
    );
}

#[test]
fn super_lowers_to_call_super() {
    // OOP slice 4: `super` (`__super__`) renders as `_sir_call_super(method,
    // definingClass, argc, …)` — both names quoted; dispatch is an ancestry walk.
    let m = lower_ruby(
        "class Base\n  def val\n    10\n  end\nend\n\
         class Derived < Base\n  def val\n    super + 5\n  end\nend\nputs Derived.new.val",
    );
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_call_super(\"val\", \"Derived\""),
        "`super` resolves from the defining class's superclass\n{src}"
    );
}

#[test]
fn class_methods_route_through_the_singleton_table() {
    // OOP slice 5: `def self.m` → `_sir_def_class_method`; `Class.m(x)` →
    // `_sir_call_class_method` (receiver is the class NAME, a quoted C literal).
    let m = lower_ruby(
        "class Math2\n  def self.double(x)\n    x + x\n  end\nend\nputs Math2.double(21)",
    );
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_def_class_method(\"Math2\", \"double\""),
        "a class method registers in the class-method table\n{src}"
    );
    assert!(
        src.contains("_sir_call_class_method(\"Math2\", \"double\""),
        "a `Class.m` call dispatches through the class-method table\n{src}"
    );
}

#[test]
fn class_vars_route_through_the_cvar_table() {
    // OOP slice 6: a class-body `@@x = 0` seeds `_sir_cvar_set_in("Class", "@@x")`;
    // a method-body read/write uses `_sir_cvar_get`/`_sir_cvar_set` (resolved via
    // the current class).  All names are quoted C string literals (no injection).
    let m = lower_ruby(
        "class Counter\n  @@count = 0\n  def self.bump\n    @@count = @@count + 1\n  end\n  \
         def peek\n    @@count\n  end\nend\nCounter.bump\nputs Counter.new.peek",
    );
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_cvar_set_in(\"Counter\", \"@@count\""),
        "the class-body initializer seeds the named class's storage\n{src}"
    );
    assert!(
        src.contains("_sir_cvar_set(\"@@count\","),
        "a method-body `@@x =` writes via the current class\n{src}"
    );
    assert!(
        src.contains("_sir_cvar_get(\"@@count\")"),
        "a method-body `@@x` read resolves via the current class\n{src}"
    );
}

#[test]
fn modules_register_include_and_extend() {
    // OOP slice 7: `include`/`extend` render as `_sir_register_include` /
    // `_sir_register_extend` (both names quoted); a `module` declaration is a
    // comment.  Module methods reuse `__def_method__` (keyed on the module name).
    let m = lower_ruby(
        "module Greet\n  def hi\n    42\n  end\nend\n\
         class Person\n  include Greet\nend\n\
         module Cls\n  def tag\n    7\n  end\nend\n\
         class Widget\n  extend Cls\nend\nputs Person.new.hi",
    );
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_register_include(\"Person\", \"Greet\")"),
        "`include` records the mixin\n{src}"
    );
    assert!(
        src.contains("_sir_register_extend(\"Widget\", \"Cls\")"),
        "`extend` records the class-method mixin\n{src}"
    );
    assert!(
        src.contains("_sir_def_method(\"Greet\", \"hi\""),
        "a module method is registered like a class method, keyed on the module\n{src}"
    );
}

#[test]
fn collection_string_methods_route_to_the_builtin_dispatcher() {
    // Collections slice 1: a `__method__` dispatch to a built-in method name
    // (`upcase`) that is NOT a user-defined method routes to
    // `_sir_builtin_method` (the runtime dispatcher), not the user method table.
    let m = lower_ruby("puts \"hi\".upcase");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_builtin_method("),
        "a built-in method dispatches through the runtime dispatcher\n{src}"
    );
    assert!(
        src.contains("\"upcase\""),
        "the method name is a quoted C string literal\n{src}"
    );
}

#[test]
fn collection_string_query_passes_its_argument() {
    // Collections slice 2: a 1-arg built-in query (`include?`) routes to the
    // dispatcher AND carries its argument (the substring) through.
    let m = lower_ruby("puts \"hello\".include?(\"ell\")");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_builtin_method("),
        "the query routes to the runtime dispatcher\n{src}"
    );
    // The `?` in `include?` is trigraph-escaped (`\?`) by `quote_c_string`; what
    // matters is that argc=1 and the substring argument are passed through.
    assert!(
        src.contains(", 1, _sir_str(\"ell\"))"),
        "the dispatcher is passed argc=1 and the quoted substring argument\n{src}"
    );
}

#[test]
fn collection_array_methods_route_to_the_builtin_dispatcher() {
    // Collections slice 3: a `__method__` dispatch to a 0-arg Array method
    // (`sort`) that is NOT a user-defined method routes to
    // `_sir_builtin_method`, same as the slice-1 String methods.
    let m = lower_ruby("puts [3, 1, 2].sort");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_builtin_method("),
        "a built-in Array method dispatches through the runtime dispatcher\n{src}"
    );
    assert!(
        src.contains("\"sort\""),
        "the method name is a quoted C string literal\n{src}"
    );
}

#[test]
fn collection_array_reverse_shares_the_slice1_string_name() {
    // `reverse` is allowlisted once (slice 1, for String) and its runtime arm
    // widens to accept `SIR_SEQ` in slice 3 — the allowlist itself does not
    // change, so this just confirms an Array receiver still dispatches (not
    // rejected as an unknown-for-this-receiver-type name at compile time; the
    // receiver-type check happens at RUNTIME, matching every other polymorphic
    // built-in here).
    let m = lower_ruby("puts [1, 2, 3].reverse");
    let src = compile(&m).unwrap().source;
    assert!(
        src.contains("_sir_builtin_method("),
        "an Array `reverse` dispatches through the runtime dispatcher, not rejected at compile time\n{src}"
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
