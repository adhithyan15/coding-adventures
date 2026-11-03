//! A **variable never shadows a function** of the same name, on every
//! backend (#18485).
//!
//! SIR keeps two namespaces: a [`Function`](semantic_ir::Function) is named
//! by `DirectCall`/`MakeClosure`, a variable by `VarRef`/`Assign`/`LetStar`.
//! A source language may rely on that.  Ruby does:
//!
//! ```ruby
//! def f; 40; end
//! f = 2
//! puts f() + f      # `f()` calls the method, bare `f` reads the local: 42
//! ```
//!
//! Most target languages have only *one* namespace for both.  A backend
//! that emits the function as `f` and the local as `f` lets the local
//! shadow the function, so the call reads the integer:
//!
//! | backend    | what the shadowed call did                      |
//! |------------|-------------------------------------------------|
//! | JavaScript | `TypeError: f is not a function`                 |
//! | Python     | `TypeError: 'int' object is not callable`        |
//! | Go         | `invalid operation: cannot call non-function f`  |
//! | Rust       | `error[E0618]: expected function, found i64`     |
//!
//! C and Ruby were already safe: they emit every user function as
//! `sir_fn_<name>`, a prefix no variable can spell.  The other backends
//! cannot rename their functions (callers append code that calls an
//! emitted program's functions by name), so they rename the *variable*
//! instead, and only when it collides: see each backend's `var_ident`.
//!
//! Each program's expected stdout is what real Ruby prints, and the run
//! test re-checks it against `ruby` when one is on `PATH`.
//!
//! The Ruby frontend cannot yet spell every kind of variable (it has no
//! closure literal this harness can run everywhere), so the second half of
//! this file builds SIR **by hand** for the rest: a `for` range variable, a
//! `for each` variable, a closure capture, and a keyword parameter, each
//! named like the module's function `f`.  (Since #18778 the frontend has a
//! `for` loop, so a Ruby `for` variable named `f` is a program above too.)

use std::io::Write;
use std::process::{Command, Stdio};

use semantic_ir::{
    Block, Capture, CaptureValue, Effect, EffectSet, Expr, Feature, FeatureManifest, Function,
    Global, Metadata, Module, Param, ParamKind, Scope, Span, Stmt,
};
use sir_conformance::{lower_source_via, run_module, run_source_via, Frontend, RunOutcome, Target};

/// `(name, Ruby source, Ruby's stdout)`.
const PROGRAMS: &[(&str, &str, &str)] = &[
    // The issue's program: a top-level local named like a method.
    (
        "rb_main_local_named_like_a_method",
        "def f\n  40\nend\nf = 2\nputs f() + f\n",
        "42",
    ),
    // A local inside a method body.
    (
        "rb_method_local_named_like_a_method",
        "def f\n  40\nend\ndef g\n  f = 2\n  n = f() + f\n  n\nend\nputs g()\n",
        "42",
    ),
    // A parameter named like a method.
    (
        "rb_param_named_like_a_method",
        "def f\n  40\nend\ndef g(f)\n  n = f() + f\n  n\nend\nputs g(2)\n",
        "42",
    ),
    // A `rescue` binding named like a method.
    (
        "rb_rescue_binding_named_like_a_method",
        "def f\n  40\nend\nbegin\n  raise \"boom\"\nrescue StandardError => f\n  puts f()\nend\n",
        "40",
    ),
    // A `for` loop variable named like a method (#18778): the loop iterates
    // a fresh variable and stores it into the local `f`, which keeps the
    // last item.  (40 + 1) + (40 + 2) = 83.
    (
        "rb_for_variable_named_like_a_method",
        "def f\n  40\nend\nt = 0\nfor f in [1, 2]\n  t = t + f() + f\nend\nputs t\nputs f\n",
        "83\n2",
    ),
    // The method is also called *recursively* from a body whose parameter
    // spells its name: the call inside must still reach the method.
    (
        "rb_recursive_method_with_param_named_like_itself",
        "def f(f)\n  if f > 0\n    n = f(f - 1)\n    n + 1\n  else\n    40\n  end\nend\nputs f(2)\n",
        "42",
    ),
];

/// Ruby's stdout for `source` (trailing newlines trimmed), or `None` when
/// no `ruby` is on `PATH`.
fn real_ruby(source: &str) -> Option<String> {
    let mut child = Command::new("ruby")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(source.as_bytes())
        .expect("write the program to ruby");
    let out = child.wait_with_output().expect("ruby runs");
    assert!(
        out.status.success(),
        "ruby rejected the program: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(
        String::from_utf8_lossy(&out.stdout)
            .replace("\r\n", "\n")
            .trim_end_matches('\n')
            .to_string(),
    )
}

/// Every program lowers to SIR that passes the validator.
#[test]
fn programs_lower_and_validate() {
    for (name, source, _) in PROGRAMS {
        let module = lower_source_via(name, source, Frontend::Ruby)
            .unwrap_or_else(|e| panic!("`{name}` failed to lower: {e}"));
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "`{name}` failed validation: {:?}",
            report.issues
        );
    }
}

/// Every program prints Ruby's output on every available backend.
///
/// Every `(program, backend)` cell runs before the test reports, so one
/// failure lists all the cells that disagree.
#[test]
fn programs_print_rubys_output_on_every_backend() {
    let mut ran = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (name, source, expected) in PROGRAMS {
        if let Some(actual) = real_ruby(source) {
            assert_eq!(actual, *expected, "`{name}`: Ruby prints {actual:?}");
        }
        for &target in Target::all() {
            match run_source_via(name, source, Frontend::Ruby, target) {
                RunOutcome::Ran(stdout) => {
                    if stdout != *expected {
                        wrong.push(format!(
                            "`{name}` on {}: printed {stdout:?}, expected {expected:?}",
                            target.tag()
                        ));
                    }
                    ran += 1;
                }
                RunOutcome::Skipped(why) if why.contains("does not yet accept") => {
                    wrong.push(format!("`{name}` on {}: refused: {why}", target.tag()))
                }
                RunOutcome::Skipped(why) => eprintln!("skip {name}/{}: {why}", target.tag()),
                RunOutcome::Failed(msg) => {
                    let head: Vec<&str> = msg.lines().take(6).collect();
                    wrong.push(format!(
                        "`{name}` on {}: failed:\n    {}",
                        target.tag(),
                        head.join("\n    ")
                    ));
                }
            }
        }
    }
    eprintln!("local shadows function: {ran} program/backend runs");
    assert!(
        wrong.is_empty(),
        "{} program/backend cells disagree with Ruby:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Hand-built SIR: the variable kinds no frontend here can spell yet
// ---------------------------------------------------------------------------

fn sp() -> Span {
    Span::synthetic()
}

fn int(value: i64) -> Expr {
    Expr::IntLit { value, span: sp() }
}

fn var(name: &str, scope: Scope) -> Expr {
    Expr::VarRef {
        name: name.to_string(),
        scope,
        span: sp(),
    }
}

fn call(fn_name: &str, args: Vec<Expr>) -> Expr {
    Expr::DirectCall {
        fn_name: fn_name.to_string(),
        args,
        effects: EffectSet::PURE,
        span: sp(),
    }
}

fn add(a: Expr, b: Expr) -> Expr {
    Expr::BuiltinCall {
        name: "+".to_string(),
        args: vec![a, b],
        effects: EffectSet::PURE,
        span: sp(),
    }
}

/// `f() + f`, where the bare `f` is the variable of `scope`: the sum is
/// right only if the call reaches the function *and* the read reaches the
/// variable.
fn call_plus_var(scope: Scope) -> Expr {
    add(call("f", vec![]), var("f", scope))
}

fn block(stmts: Vec<Stmt>, value: Expr) -> Block {
    Block {
        stmts,
        value,
        span: sp(),
    }
}

fn func(name: &str, params: Vec<Param>, captures: Vec<&str>, body: Block) -> Function {
    Function {
        name: name.to_string(),
        params,
        return_type: None,
        captures: captures
            .into_iter()
            .map(|c| Capture {
                name: c.to_string(),
                sir_type: None,
            })
            .collect(),
        body,
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    }
}

fn param(name: &str, kind: ParamKind) -> Param {
    Param {
        name: name.to_string(),
        sir_type: None,
        kind,
        default: None,
        span: sp(),
    }
}

/// `t = t + <value>` for the local accumulator `t`.
fn accumulate(value: Expr) -> Stmt {
    Stmt::Assign {
        name: "t".to_string(),
        scope: Scope::Local,
        value: add(var("t", Scope::Local), value),
        span: sp(),
    }
}

/// A module of `def f; 40; end`, the function `g` under test, and a `main`
/// that prints `g()`.
fn module(name: &str, g: Function, extra: Vec<Function>, features: &[Feature]) -> Module {
    let print_g = Expr::BuiltinCall {
        name: "__sys_write__".to_string(),
        args: vec![
            Expr::StrLit {
                value: "stdout".to_string(),
                span: sp(),
            },
            Expr::StrLit {
                value: "per_value".to_string(),
                span: sp(),
            },
            Expr::BoolLit {
                value: true,
                span: sp(),
            },
            call("g", vec![]),
        ],
        effects: EffectSet::from_effects(&[Effect::MayPrint]),
        span: sp(),
    };
    let mut functions = vec![func("f", vec![], vec![], block(vec![], int(40))), g];
    functions.extend(extra);
    functions.push(func("main", vec![], vec![], block(vec![], print_g)));
    let mut all = vec![Feature::ConsoleIO, Feature::Strings];
    all.extend_from_slice(features);
    Module {
        name: name.to_string(),
        manifest: FeatureManifest::from_features(&all),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new(),
        span: sp(),
    }
}

/// [`module`], plus a module global `global` that the synthesised `_init`
/// sets to 2 -- the `global_set` shape the Twig frontend emits for
/// `(define x 2)`.
fn module_with_global(name: &str, global: &str, g: Function) -> Module {
    let init = func(
        "_init",
        vec![],
        vec![],
        block(
            vec![Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "global_set".to_string(),
                    args: vec![
                        Expr::SymLit {
                            name: global.to_string(),
                            span: sp(),
                        },
                        int(2),
                    ],
                    effects: EffectSet::PURE,
                    span: sp(),
                },
                span: sp(),
            }],
            Expr::NilLit { span: sp() },
        ),
    );
    let mut m = module(name, g, vec![init], &[Feature::Globals, Feature::Symbols]);
    m.globals.push(Global {
        name: global.to_string(),
        sir_type: None,
        init_function: "_init".to_string(),
        span: sp(),
    });
    m
}

/// `(name, module, the stdout every backend must print)`.
fn hand_built() -> Vec<(&'static str, Module, &'static str)> {
    let t0 = || Stmt::LetStarBinding {
        name: "t".to_string(),
        sir_type: None,
        value: int(0),
        span: sp(),
    };
    vec![
        // for f in 0...2: t = t + f() + f      → (40 + 0) + (40 + 1) = 81
        (
            "sir_for_range_var_named_like_a_function",
            module(
                "sir_for_range_var_named_like_a_function",
                func(
                    "g",
                    vec![],
                    vec![],
                    block(
                        vec![
                            t0(),
                            Stmt::ForRange {
                                var: "f".to_string(),
                                start: int(0),
                                stop: int(2),
                                step: int(1),
                                body: block(vec![accumulate(call_plus_var(Scope::Local))], int(0)),
                                span: sp(),
                            },
                        ],
                        var("t", Scope::Local),
                    ),
                ),
                vec![],
                &[Feature::Loops, Feature::MutableBindings],
            ),
            "81",
        ),
        // for f in [1, 2]: t = t + f() + f      → (40 + 1) + (40 + 2) = 83
        (
            "sir_for_each_var_named_like_a_function",
            module(
                "sir_for_each_var_named_like_a_function",
                func(
                    "g",
                    vec![],
                    vec![],
                    block(
                        vec![
                            t0(),
                            Stmt::ForEach {
                                var: "f".to_string(),
                                iter: Expr::SeqLit {
                                    items: vec![int(1), int(2)],
                                    span: sp(),
                                },
                                body: block(vec![accumulate(call_plus_var(Scope::Local))], int(0)),
                                span: sp(),
                            },
                        ],
                        var("t", Scope::Local),
                    ),
                ),
                vec![],
                &[Feature::Loops, Feature::MutableBindings, Feature::Sequences],
            ),
            "83",
        ),
        // A closure body whose capture is named `f`, and a caller whose
        // local is named `f`:  let f = 2; (closure capturing f)()  → 40 + 2
        (
            "sir_capture_named_like_a_function",
            module(
                "sir_capture_named_like_a_function",
                func(
                    "g",
                    vec![],
                    vec![],
                    block(
                        vec![Stmt::LetStarBinding {
                            name: "f".to_string(),
                            sir_type: None,
                            value: int(2),
                            span: sp(),
                        }],
                        Expr::IndirectCall {
                            target: Box::new(Expr::MakeClosure {
                                fn_name: "body".to_string(),
                                captures: vec![CaptureValue {
                                    name: "f".to_string(),
                                    value: var("f", Scope::Local),
                                }],
                                span: sp(),
                            }),
                            args: vec![],
                            effects: EffectSet::PURE,
                            span: sp(),
                        },
                    ),
                ),
                vec![func(
                    "body",
                    vec![],
                    vec!["f"],
                    block(vec![], call_plus_var(Scope::Capture)),
                )],
                &[Feature::Closures],
            ),
            "42",
        ),
        // A keyword parameter named `f`, passed by keyword:
        // def k(f:) = f() + f; k(f: 2)  → 42.  The keyword at the call site
        // and the parameter must be renamed alike.
        (
            "sir_keyword_param_named_like_a_function",
            module(
                "sir_keyword_param_named_like_a_function",
                func(
                    "g",
                    vec![],
                    vec![],
                    block(
                        vec![],
                        call(
                            "k",
                            vec![Expr::KeywordArg {
                                name: "f".to_string(),
                                value: Box::new(int(2)),
                                span: sp(),
                            }],
                        ),
                    ),
                ),
                vec![func(
                    "k",
                    vec![param("f", ParamKind::Keyword)],
                    vec![],
                    block(vec![], call_plus_var(Scope::Param)),
                )],
                &[Feature::KeywordParams, Feature::DynamicTyping],
            ),
            "42",
        ),
        // A module *global* named `f` beside the function `f`:
        // f() + <global f> → 40 + 2.  JavaScript declared both at the top
        // level (`let f = null;` and `function f`), a SyntaxError.
        (
            "sir_global_named_like_a_function",
            module_with_global(
                "sir_global_named_like_a_function",
                "f",
                func("g", vec![], vec![], block(vec![], call_plus_var(Scope::Global))),
            ),
            "42",
        ),
        // A *local* `x` beside the global `x`, read in the same function:
        // let x = 40; <local x> + <global x> → 42.  In JavaScript the local
        // hid the global, so the sum was 80.
        (
            "sir_local_named_like_a_global",
            module_with_global(
                "sir_local_named_like_a_global",
                "x",
                func(
                    "g",
                    vec![],
                    vec![],
                    block(
                        vec![Stmt::LetStarBinding {
                            name: "x".to_string(),
                            sir_type: None,
                            value: int(40),
                            span: sp(),
                        }],
                        add(var("x", Scope::Local), var("x", Scope::Global)),
                    ),
                ),
            ),
            "42",
        ),
    ]
}

/// The hand-built modules are valid SIR, so a backend failure is the
/// backend's, not the module's.
#[test]
fn hand_built_modules_validate() {
    for (name, module, _) in hand_built() {
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "`{name}` failed validation: {:?}",
            report.issues
        );
    }
}

/// Each hand-built module prints its expected value on every backend that
/// accepts it.  A backend that does not yet accept a module's features (the
/// C and Ruby backends' declared gaps) is skipped, not failed: what is under
/// test is the spelling of the variable, not the feature.
#[test]
fn hand_built_modules_print_the_same_on_every_backend() {
    let mut ran = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (name, module, expected) in hand_built() {
        for &target in Target::all() {
            match run_module(name, &module, target) {
                RunOutcome::Ran(stdout) => {
                    if stdout != expected {
                        wrong.push(format!(
                            "`{name}` on {}: printed {stdout:?}, expected {expected:?}",
                            target.tag()
                        ));
                    }
                    ran += 1;
                }
                RunOutcome::Skipped(why) => eprintln!("skip {name}/{}: {why}", target.tag()),
                RunOutcome::Failed(msg) => {
                    let head: Vec<&str> = msg.lines().take(6).collect();
                    wrong.push(format!(
                        "`{name}` on {}: failed:\n    {}",
                        target.tag(),
                        head.join("\n    ")
                    ));
                }
            }
        }
    }
    eprintln!("hand-built shadowing modules: {ran} module/backend runs");
    assert!(
        wrong.is_empty(),
        "{} module/backend cells are wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Constants: a namespace of their own too
// ---------------------------------------------------------------------------

/// `(name, Ruby source, Ruby's stdout)` for a *constant* spelled like a
/// method.  Ruby keeps the two apart -- `F()` calls the method, bare `F`
/// reads the constant.  The Python and TypeScript backends bound the
/// constant as an ordinary identifier, which shadowed the function
/// (`TypeError: 'int' object is not callable`).
const CONSTANT_PROGRAMS: &[(&str, &str, &str)] = &[
    (
        "rb_constant_named_like_a_method",
        "def F\n  40\nend\nF = 2\nputs F() + F\n",
        "42",
    ),
    // The class operand of `raise`/`rescue` is a constant too.  Every
    // backend accepts this one.
    (
        "rb_raised_class_named_like_a_method",
        "class Boom < StandardError\nend\ndef Boom\n  40\nend\nbegin\n  raise Boom, \"x\"\nrescue Boom => e\n  puts Boom()\nend\n",
        "40",
    ),
];

/// Each constant program prints Ruby's output on every backend that accepts
/// it.  The JavaScript, Go and Rust backends refuse a constant *assignment*
/// (`UnsupportedFeature`: they accept constants only as class names), and
/// that refusal is a skip; every other failure counts.
#[test]
fn constants_named_like_methods_print_rubys_output() {
    let mut ran = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (name, source, expected) in CONSTANT_PROGRAMS {
        if let Some(actual) = real_ruby(source) {
            assert_eq!(actual, *expected, "`{name}`: Ruby prints {actual:?}");
        }
        for &target in Target::all() {
            match run_source_via(name, source, Frontend::Ruby, target) {
                RunOutcome::Ran(stdout) => {
                    if stdout != *expected {
                        wrong.push(format!(
                            "`{name}` on {}: printed {stdout:?}, expected {expected:?}",
                            target.tag()
                        ));
                    }
                    ran += 1;
                }
                RunOutcome::Failed(msg) if msg.contains("UnsupportedFeature") => {
                    eprintln!("skip {name}/{}: refuses a constant assignment", target.tag())
                }
                RunOutcome::Skipped(why) => eprintln!("skip {name}/{}: {why}", target.tag()),
                RunOutcome::Failed(msg) => {
                    let head: Vec<&str> = msg.lines().take(6).collect();
                    wrong.push(format!(
                        "`{name}` on {}: failed:\n    {}",
                        target.tag(),
                        head.join("\n    ")
                    ));
                }
            }
        }
    }
    eprintln!("constants named like methods: {ran} program/backend runs");
    assert!(
        wrong.is_empty(),
        "{} program/backend cells disagree with Ruby:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
