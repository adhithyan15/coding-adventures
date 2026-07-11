//! Ruby `GrammarASTNode` → `semantic_ir::Module` lowering.
//!
//! See [the crate README](../README.md) for the v0 scope.  The
//! lowering is deliberately tiny because the v0 ruby-parser grammar
//! itself is tiny (six rules: `program`, `statement`, `assignment`,
//! `method_call`, `expression_stmt`, `expression`, `term`, `factor`).
//! Anything Ruby can write that isn't covered by those rules either
//! fails to parse or reaches us as a more general shape that we
//! still pattern-match against.

use std::collections::HashSet;

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, Capture, CaptureValue, Effect, EffectSet, ExportName, Expr, Feature, FeatureManifest,
    Function, IndexArg, Metadata, Module, Param, ParamKind, RescueClause, Scope, Span, Stmt,
};

/// A failure encountered during Ruby → SIR lowering.
///
/// Carries 1-based line/column so callers can produce IDE-friendly
/// diagnostics.  When the position is unknown (e.g. the AST node
/// had no recorded span), the fields are zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for RubyLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RubyLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for RubyLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Ruby program into a `semantic_ir::Module`.
///
/// The root node must be a `program` rule node — that's what
/// [`coding_adventures_ruby_parser::parse_ruby`] always emits.  The
/// `module_name` becomes the SIR module identifier (typically the
/// source file's stem).
pub fn compile(program: &GrammarASTNode, module_name: &str) -> Result<Module, RubyLowerError> {
    if program.rule_name != "program" {
        return Err(RubyLowerError {
            message: format!("expected root rule `program`, got `{}`", program.rule_name),
            line: program.start_line.unwrap_or(0),
            column: program.start_column.unwrap_or(0),
        });
    }

    let mut lw = Lowerer {
        file_name: module_name.to_string(),
        declared_locals: HashSet::new(),
        current_params: HashSet::new(),
        user_functions: Vec::new(),
        features_used: HashSet::new(),
        block_counter: 0,
        multi_assign_counter: 0,
        interp_depth: 0,
        block_param_methods: HashSet::new(),
        in_def_body: false,
        block_captures_enclosing: false,
        in_block_body: false,
        current_class: None,
        current_method: None,
    };
    // Phase 6a: hoist `def name(params) … end` declarations to
    // top-level Functions BEFORE walking the rest of the program so
    // the main-body lowerer knows which names resolve as
    // `DirectCall` targets vs. unknown builtins.
    lw.collect_def_statements(program)?;
    let block = lw.lower_program(program)?;

    let main = Function {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: None,
        captures: Vec::new(),
        body: block,
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: lw.span_of(program),
    };

    // User-defined functions come first, then `main`.  The SIR
    // validator doesn't care about ordering — backends that emit
    // forward declarations will still see `main` exported.
    let mut functions = std::mem::take(&mut lw.user_functions);
    functions.push(main);

    // Phase Q9f (FC) — explicit block-param ABI, part 2: call-site
    // normalization.  Now that every `def` has been lowered,
    // `lw.block_param_methods` holds the full set of methods that gained
    // a trailing `__sir_block__` parameter (Q9e).  Walk *all* function
    // bodies (user functions + `main`) and thread the matching block
    // argument at every `DirectCall` to one of those methods, so call
    // arity matches the threaded def regardless of call-before-def or
    // mutual recursion.  Running here — after the whole program is
    // lowered — is what makes the pass order-independent.
    if !lw.block_param_methods.is_empty() {
        for f in &mut functions {
            // Phase Q10c — names bound as a param/local *within this
            // function*.  A bare reference to one of these is a variable,
            // not a parenless call, so it must be excluded from the
            // call-rewrite below (a local can legitimately shadow a
            // method name).
            let mut bound: HashSet<String> = f.params.iter().map(|p| p.name.clone()).collect();
            Lowerer::collect_bound_names_block(&f.body, &mut bound);
            let ctx = BlockNormCtx {
                methods: &lw.block_param_methods,
                bound: &bound,
            };
            Lowerer::normalize_block_call_args(&mut f.body, &ctx);
        }
    }

    // SIR's validator requires the manifest to *exactly* match
    // usage (declared-but-unused is a warning, used-but-undeclared
    // is an error).  We've been tallying features as we lowered;
    // here we materialise them into the manifest in a stable
    // chronological order.
    let mut manifest = FeatureManifest::new();
    for f in [
        Feature::DynamicTyping,
        Feature::MutableBindings,
        Feature::Loops,
        Feature::Sequences,
        Feature::Maps,
        Feature::Symbols,
        Feature::Closures,
        // Phase 6l — method-call chains synthesise a `StrLit` for the
        // method name when packing into the `__method__` envelope.
        // StrLit usage triggers the `Strings` feature.
        Feature::Strings,
        // Phase 6z — float literals (`1.5`, `1e10`, `1.5e-3`) lower
        // to `Expr::FloatLit` and trigger the `Floats` feature.
        Feature::Floats,
        // Phase 14a (FC) — `class Foo; end` lowers to
        // `Stmt::ClassDef` and triggers the `Classes` feature.
        Feature::Classes,
        // Phase 14d (FC) — `module M; end` lowers to
        // `Stmt::ModuleDef` and triggers the `Modules` feature.
        Feature::Modules,
        // Phase 15a (FC) — an instance-var ref (`@x`, `Scope::Instance`)
        // triggers the `InstanceVars` feature.
        Feature::InstanceVars,
        // Phase 15b (FC) — a class-var ref (`@@x`, `Scope::ClassVar`)
        // triggers the `ClassVars` feature.
        Feature::ClassVars,
        // Phase 15c (FC) — a constant ref/assign (`FOO`, `Scope::Const`)
        // triggers the `Constants` feature.
        Feature::Constants,
        // Phase 16a (FC) — `begin/rescue/ensure/end` (`Stmt::TryCatch`)
        // triggers the `Exceptions` feature.
        Feature::Exceptions,
        // Phase 20b (FC) — a multi-segment interpolation/concat
        // (`"a#{x}b"`) lowers to `Expr::StrConcat` and triggers the
        // `StringInterpolation` feature.
        Feature::StringInterpolation,
        // Phase 13a/13b (FC) — a structural array pattern with a literal
        // or nested element ANDs its checks via `Expr::LogicalAnd`, which
        // triggers the `ShortCircuit` feature.
        Feature::ShortCircuit,
        // Phase P7 (Ruby 1.0) — a parameter with a `name = expr` default
        // lowers to `Param.default = Some(_)` and triggers the
        // `DefaultParams` feature (`extract_params`).
        Feature::DefaultParams,
        // Phase KW7 (Ruby 1.0 unblock) — a keyword parameter (`a:` / `a: 1`,
        // `ParamKind::Keyword`) or a keyword argument (`f(a: 1)`,
        // `Expr::KeywordArg`) triggers the `KeywordParams` feature. Set by
        // `extract_params` (def side) and `lower_call_arg` (call side).
        Feature::KeywordParams,
    ] {
        if lw.features_used.contains(&f) {
            manifest.add(f);
        }
    }

    Ok(Module {
        name: module_name.to_string(),
        manifest,
        imports: Vec::new(),
        // `main` is the conventional entry point — exporting it lets
        // SIR backends recognise it as such.
        exports: vec![ExportName {
            name: "main".to_string(),
            span: Span::synthetic(),
        }],
        functions,
        globals: Vec::new(),
        metadata: Metadata::new(),
        span: lw.span_of(program),
    })
}

// ---------------------------------------------------------------------------
// Phase 9a (FC) — helper: detect VarRefs that name LHS targets
// ---------------------------------------------------------------------------
//
// Used by `lower_multi_assignment` to decide whether the simple
// sequential lowering is safe (`a, b = 1, 2`) or whether the swap-safe
// temp-pass is needed (`a, b = b, a`).  Walks an `Expr` tree
// recursively and returns true iff any `VarRef` it contains has a name
// listed in `names`.
//
// This is a structural recursion over every `Expr` variant the SIR
// defines.  Adding a new `Expr` variant to `semantic-ir` will cause
// this match to fail-stop at compile time (Rust's exhaustiveness
// check), prompting an update here — important because a missed
// variant would silently re-introduce the swap mis-lowering.
fn expr_references_any_name(expr: &Expr, names: &HashSet<String>) -> bool {
    match expr {
        Expr::VarRef { name, .. } => names.contains(name),
        // SIR26 conversion (not currently emitted by this frontend) — recurse.
        Expr::Convert { value, .. } => expr_references_any_name(value, names),

        // Leaves: no children, no references possible.
        Expr::IntLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::FloatLit { .. } => false,

        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_references_any_name(cond, names)
                || block_references_any_name(then_branch, names)
                || block_references_any_name(else_branch, names)
        }

        Expr::Block(b) => block_references_any_name(b, names),

        Expr::DirectCall { args, .. }
        | Expr::BuiltinCall { args, .. }
        | Expr::Intrinsic { args, .. } => args.iter().any(|a| expr_references_any_name(a, names)),

        Expr::IndirectCall { target, args, .. } => {
            expr_references_any_name(target, names)
                || args.iter().any(|a| expr_references_any_name(a, names))
        }

        Expr::MakeClosure { captures, .. } => captures
            .iter()
            .any(|c| expr_references_any_name(&c.value, names)),

        Expr::SeqLit { items, .. } => items.iter().any(|i| expr_references_any_name(i, names)),
        Expr::SeqIndex { seq, index, .. } => {
            expr_references_any_name(seq, names) || expr_references_any_name(index, names)
        }
        Expr::SeqLen { seq, .. } => expr_references_any_name(seq, names),

        Expr::MapLit { entries, .. } => entries.iter().any(|e| {
            expr_references_any_name(&e.key, names) || expr_references_any_name(&e.value, names)
        }),
        Expr::MapGet { map, key, .. } => {
            expr_references_any_name(map, names) || expr_references_any_name(key, names)
        }

        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            expr_references_any_name(lhs, names) || expr_references_any_name(rhs, names)
        }

        Expr::StrConcat { parts, .. } => parts.iter().any(|p| expr_references_any_name(p, names)),

        // KW1 compile-compat stub: a `KeywordArg` is a single-child wrapper
        // whose runtime meaning is its inner `value` (the `name` is a static
        // label carrying no `VarRef`).  Recurse into `value` so the swap-safety
        // reference check stays faithful.  Real support pending KW2–KW8.
        Expr::KeywordArg { value, .. } => expr_references_any_name(value, names),

        // SIR22 compile-compat stubs: the Ruby frontend never *produces* any
        // of these array/matrix nodes today (no lowering path emits them), so
        // these arms are unreachable in practice.  They are still walked
        // structurally — like every other arm above — so that if a future
        // lowering path does start emitting them, the swap-safety check
        // keeps scanning every child `Expr` for `VarRef`s instead of silently
        // going blind on a subtree.
        Expr::ArrayLit { rows, .. } => rows
            .iter()
            .any(|row| row.iter().any(|e| expr_references_any_name(e, names))),
        Expr::Range {
            start, step, stop, ..
        } => {
            expr_references_any_name(start, names)
                || step
                    .as_ref()
                    .is_some_and(|s| expr_references_any_name(s, names))
                || expr_references_any_name(stop, names)
        }
        Expr::MatMul { lhs, rhs, .. } => {
            expr_references_any_name(lhs, names) || expr_references_any_name(rhs, names)
        }
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            expr_references_any_name(lhs, names) || expr_references_any_name(rhs, names)
        }
        Expr::Transpose { target, .. } => expr_references_any_name(target, names),
        Expr::IndexGet {
            target, indices, ..
        } => {
            expr_references_any_name(target, names)
                || indices
                    .iter()
                    .any(|idx| index_arg_references_any_name(idx, names))
        }

        // SIR23 compile-compat stubs: same rationale as the SIR22 stubs
        // above — the Ruby frontend never produces any symbolic-expression
        // or pattern/rewrite node today, but every arm still recurses
        // structurally so the swap-safety check keeps scanning every child
        // `Expr` for `VarRef`s if a future lowering path starts emitting
        // them.
        Expr::SymSymbol { .. } | Expr::SymRational { .. } => false,
        Expr::SymApply { head, args, .. } => {
            expr_references_any_name(head, names)
                || args.iter().any(|a| expr_references_any_name(a, names))
        }
        Expr::SymPatternBlank { head, .. } => head
            .as_ref()
            .is_some_and(|h| expr_references_any_name(h, names)),
        Expr::SymPatternNamed { pattern, .. } => expr_references_any_name(pattern, names),
        Expr::SymRule { lhs, rhs, .. } => {
            expr_references_any_name(lhs, names) || expr_references_any_name(rhs, names)
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            expr_references_any_name(expr, names)
                || rules.iter().any(|r| expr_references_any_name(r, names))
        }
    }
}

// SIR22 helper: `IndexArg` (used by `Expr::IndexGet` / `Stmt::IndexSet`) is
// not an `Expr` itself but a small wrapper enum around one — mirroring how
// this file already unwraps other non-`Expr` wrapper shapes (e.g. `MapLit`'s
// entries) before recursing into their contained expressions.
fn index_arg_references_any_name(idx: &IndexArg, names: &HashSet<String>) -> bool {
    match idx {
        IndexArg::Scalar(e) | IndexArg::Range(e) => expr_references_any_name(e, names),
        IndexArg::Whole => false,
    }
}

fn block_references_any_name(block: &Block, names: &HashSet<String>) -> bool {
    // Block stmts: scan each stmt's contained Expr(s).  We only need a
    // shallow walk over the tree shapes that `lower_expression` itself
    // can emit — and `lower_expression`'s output is by definition a
    // single `Expr` so any sub-blocks were built by the same lowerer
    // (and are subject to the same parent's scope rules).  We still
    // scan them defensively in case future lowering paths nest blocks.
    if expr_references_any_name(&block.value, names) {
        return true;
    }
    for stmt in &block.stmts {
        match stmt {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::ExprStmt { expr: value, .. }
            | Stmt::Assign { value, .. } => {
                if expr_references_any_name(value, names) {
                    return true;
                }
            }
            Stmt::While { cond, body, .. } => {
                if expr_references_any_name(cond, names) || block_references_any_name(body, names) {
                    return true;
                }
            }
            Stmt::ForRange {
                start,
                stop,
                step,
                body,
                ..
            } => {
                if expr_references_any_name(start, names)
                    || expr_references_any_name(stop, names)
                    || expr_references_any_name(step, names)
                    || block_references_any_name(body, names)
                {
                    return true;
                }
            }
            Stmt::ForEach { iter, body, .. } => {
                if expr_references_any_name(iter, names) || block_references_any_name(body, names) {
                    return true;
                }
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                if expr_references_any_name(seq, names)
                    || expr_references_any_name(index, names)
                    || expr_references_any_name(value, names)
                {
                    return true;
                }
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                if expr_references_any_name(map, names)
                    || expr_references_any_name(key, names)
                    || expr_references_any_name(value, names)
                {
                    return true;
                }
            }
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                // SIR22 compile-compat stub (never emitted by this frontend
                // today — see the `Expr::ArrayLit`/etc. arms above for the
                // same rationale): mirrors the `SeqSet`/`MapSet` arms by
                // recursing into every child `Expr`, including each
                // `IndexArg`'s embedded expression.
                if expr_references_any_name(target, names)
                    || indices
                        .iter()
                        .any(|idx| index_arg_references_any_name(idx, names))
                    || expr_references_any_name(value, names)
                {
                    return true;
                }
            }
            Stmt::ClassDef { body, .. }
            | Stmt::ModuleDef { body, .. }
            | Stmt::SingletonClassDef { body, .. } => {
                // A class/module/singleton declaration body is itself a
                // `Vec<Stmt>`.  Recurse over each contained statement
                // using a synthetic wrapper Block, mirroring the loop /
                // pattern-stmt arms above.
                for inner in body {
                    let synthetic = Block {
                        stmts: vec![inner.clone()],
                        value: Expr::NilLit {
                            span: inner.span().clone(),
                        },
                        span: inner.span().clone(),
                    };
                    if block_references_any_name(&synthetic, names) {
                        return true;
                    }
                }
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                // Exception handling (Phase 16a): the try body, each
                // rescue body, and the optional ensure body are all
                // `Vec<Stmt>`.  Recurse over every contained statement via
                // a synthetic wrapper Block, mirroring the class arm.
                let scan = |stmts: &[Stmt]| -> bool {
                    stmts.iter().any(|inner| {
                        let synthetic = Block {
                            stmts: vec![inner.clone()],
                            value: Expr::NilLit {
                                span: inner.span().clone(),
                            },
                            span: inner.span().clone(),
                        };
                        block_references_any_name(&synthetic, names)
                    })
                };
                if scan(body) || rescues.iter().any(|r| scan(&r.body)) {
                    return true;
                }
                if let Some(ens) = ensure_body {
                    if scan(ens) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Phase FC — restore **sequential** (`let*`) semantics for Ruby local
/// assignments whose RHS reads an earlier local in the same block.
///
/// Ruby evaluates statements top-to-bottom, so `a = 5; x = a` binds `x` to
/// `a`'s value — assignments are sequential.  The frontend, however, lowers
/// each first-sighting `name = value` to a **parallel** [`Stmt::LetBinding`],
/// and the SIR validator treats a *run* of consecutive `LetBinding`s as one
/// parallel-`let` group: every RHS is evaluated in the scope BEFORE any of the
/// run's names are bound (see `validator::check_stmt_seq`).  That makes
/// `[LetBinding(a); LetBinding(x = a)]` reject the read of `a`
/// (`var-ref ... unknown name 'a'`) — a real bug: `x = a`, `y = h["k"]`, and
/// any `newvar = <expr using an earlier local>` fail to compile.
///
/// This post-pass walks a block's statements and rewrites a `LetBinding` to a
/// [`Stmt::LetStarBinding`] (identical fields) exactly when its `value` reads a
/// name already bound by an EARLIER statement in the block.  `LetStarBinding`
/// is sequential — the validator adds its name immediately and it *breaks the
/// parallel run* — so the reference resolves.  Independent bindings (the common
/// case, e.g. `i = 0; sum = 0`) keep their `LetBinding` form, so nothing else
/// changes and existing shape-tests still hold.  Both variants lower to the
/// same target code (a sequential variable declaration) on every backend, so
/// the rewrite is behaviour-preserving.
fn sequentialize_let_bindings(stmts: &mut [Stmt]) {
    let mut bound: HashSet<String> = HashSet::new();
    for s in stmts.iter_mut() {
        // Decide before mutating: does THIS `LetBinding` read an earlier local?
        let convert = matches!(
            s,
            Stmt::LetBinding { value, .. } if expr_references_any_name(value, &bound)
        );
        if convert {
            // Swap the variant in place, preserving every field.  The dummy is
            // overwritten on the very next line, so it is never observed.
            let taken = std::mem::replace(
                s,
                Stmt::ExprStmt {
                    expr: Expr::NilLit {
                        span: Span::synthetic(),
                    },
                    span: Span::synthetic(),
                },
            );
            if let Stmt::LetBinding {
                name,
                sir_type,
                value,
                span,
            } = taken
            {
                *s = Stmt::LetStarBinding {
                    name,
                    sir_type,
                    value,
                    span,
                };
            }
        }
        // Record the name this statement binds, so later statements see it.
        match s {
            Stmt::LetBinding { name, .. }
            | Stmt::LetStarBinding { name, .. }
            | Stmt::Assign { name, .. } => {
                bound.insert(name.clone());
            }
            _ => {}
        }
    }
}

/// Phase 19a (FC) — if `value` is a verbatim regex-literal lexeme
/// (`/pattern/flags`), split it into `(pattern, flags)`; otherwise
/// `None`.
///
/// A Ruby regex literal lexes (Phase 2 `regex_body`) to a
/// `TokenType::String` token whose value is the verbatim source WITH the
/// surrounding slashes — `/foo/`, `/foo/i`, `//`.  A double-quoted string
/// has its delimiters stripped by the lexer, so a real string like
/// `"foo"` yields `foo` (no leading slash) and is never mistaken here.
///
/// The split: drop the opening `/`, take everything up to the LAST `/`
/// as the pattern, and the remainder as the flags.  To avoid mistaking a
/// path-shaped string such as `"/usr/bin"` (lexed value `/usr/bin`) for a
/// regex, the trailing segment must be made up ENTIRELY of valid Ruby
/// regex flag letters (`imxounes`); `/usr/bin` fails because `b` is not a
/// flag, so it stays a string.  (Residual v0 ambiguity: a double-quoted
/// string whose content happens to have regex shape, e.g. `"/a/i"`, would
/// be read as a regex — the same lexeme-prefix-sentinel limitation the
/// backtick and heredoc literals already accept.)
fn regex_pattern_flags(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_prefix('/')?;
    let close = rest.rfind('/')?;
    let pattern = &rest[..close];
    let flags = &rest[close + 1..];
    if flags
        .chars()
        .all(|c| matches!(c, 'i' | 'm' | 'x' | 'o' | 'u' | 'n' | 'e' | 's'))
    {
        Some((pattern, flags))
    } else {
        None
    }
}

/// The closing delimiter that pairs with a `%r` opening delimiter.
/// Bracket pairs mirror (`{`→`}`, `[`→`]`, `(`→`)`, `<`→`>`); any other
/// delimiter (`!`, `|`, `#`, …) is its own close.
fn percent_literal_close(open: char) -> char {
    match open {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        c => c,
    }
}

/// Phase 19d (FC) — if `value` is a `%r`-delimited regex lexeme
/// (`%r{pat}flags`, `%r(pat)`, `%r!pat!i`, …), split it into
/// `(pattern, flags)`; otherwise `None`.
///
/// The lexer's `percent_r_body` state emits the literal as a
/// `TokenType::String` token whose value is the verbatim source WITH the
/// `%r`, the opening/closing delimiters, and any trailing flags — the
/// same lexeme-prefix sentinel trick `%w`/`%q`/`%i` use.  We drop the
/// `%r`, read the opening delimiter (the next char), find the matching
/// closing delimiter (the LAST occurrence — v0 does not track nested
/// brackets, matching the other percent literals' stance), take the
/// body in between as the pattern, and the remainder as the flags.  The
/// flags must be valid Ruby regex flag letters (`imxounes`); anything
/// else means this isn't a regex (kept `None` defensively).
fn percent_r_pattern_flags(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_prefix("%r")?;
    let open = rest.chars().next()?;
    let close = percent_literal_close(open);
    let after_open = &rest[open.len_utf8()..];
    let close_idx = after_open.rfind(close)?;
    let pattern = &after_open[..close_idx];
    let flags = &after_open[close_idx + close.len_utf8()..];
    if flags
        .chars()
        .all(|c| matches!(c, 'i' | 'm' | 'x' | 'o' | 'u' | 'n' | 'e' | 's'))
    {
        Some((pattern, flags))
    } else {
        None
    }
}

/// Phase 15a (FC) — is this Name-token value a Ruby *instance
/// variable* (`@x`)?  Instance vars lex as a `Name` token carrying the
/// leading sigil.  A single `@` marks an instance variable; a double
/// `@@` is a *class* variable (Phase 15b) and a leading `$` is a
/// global — both excluded here so they keep their pre-15a handling.
fn is_instance_var_name(name: &str) -> bool {
    name.starts_with('@') && !name.starts_with("@@")
}

/// Phase 15b (FC) — is this Name-token value a Ruby *class variable*
/// (`@@x`)?  Class vars lex as a `Name` token carrying the `@@` sigil.
/// (A single `@` is an *instance* variable — see `is_instance_var_name`.)
fn is_class_var_name(name: &str) -> bool {
    name.starts_with("@@")
}

/// Phase 15c (FC) — is this Name-token value a Ruby *constant*?  In
/// Ruby, any identifier whose first character is an uppercase ASCII
/// letter is a constant (`FOO`, `Pi`, `MyClass`).  Sigil names (`@x`,
/// `@@x`, `$x`) start with punctuation, so they are never constants;
/// ordinary locals/params start lowercase or `_`.  Class/module *names*
/// in `class Foo` / `module M` are consumed by their own grammar
/// productions and never reach the generic Name→VarRef factor path, so
/// this only fires for constants used as values (reads) or assignment
/// targets (`FOO = …`).
fn is_constant_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Phase 21b (FC) — is `s` a numbered block parameter `_1`..`_9`?
/// (Two chars: `_` then a digit 1-9.  `_0` and `_10` are NOT numbered
/// params in Ruby.)  Mirrors the lexer's `is_numbered_block_param`.
fn numbered_block_param_index(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    if b.len() == 2 && b[0] == b'_' && (b'1'..=b'9').contains(&b[1]) {
        Some(b[1] - b'0')
    } else {
        None
    }
}

/// Recursively walk an AST subtree, recording the highest numbered
/// block parameter (`_1`..`_9`) that appears as a `Name` token.  Does
/// NOT descend into nested `block` nodes — a `_N` inside an inner block
/// belongs to that inner block's parameter scope, not the outer one.
fn collect_max_numbered_block_param(node: &GrammarASTNode, max: &mut u8) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => {
                if let Some(idx) = numbered_block_param_index(&t.value) {
                    if idx > *max {
                        *max = idx;
                    }
                }
            }
            ASTNodeOrToken::Node(n) => {
                // Don't cross into a nested block — its `_N` refs are
                // scoped to that block's own implicit parameters.
                if n.rule_name != "block" {
                    collect_max_numbered_block_param(n, max);
                }
            }
            _ => {}
        }
    }
}

/// Phase 21c (FC) — flatten a subtree's tokens into source order,
/// pruning nested `block` nodes (their tokens belong to the inner
/// block's own implicit-parameter scope).  Used to detect the implicit
/// `it` parameter with reliable left/right adjacency.
fn flatten_block_tokens<'a>(node: &'a GrammarASTNode, out: &mut Vec<&'a Token>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => out.push(t),
            ASTNodeOrToken::Node(n) => {
                if n.rule_name != "block" {
                    flatten_block_tokens(n, out);
                }
            }
        }
    }
}

/// Phase 21c (FC) — does this block body use the implicit `it`
/// parameter (Ruby 3.4)?  A bare `it` with no explicit `|...|` header
/// is the first block argument.  We treat an `it` Name token as the
/// implicit param ONLY when it is:
///   - NOT immediately preceded by `.` (else it's a method name:
///     `obj.it`), and
///   - NOT immediately followed by `(` (else it's a call: `it(x)`).
/// `it.foo`, `it + 1`, `puts(it)` all qualify (the `.`/`(` there are
/// not adjacent in the disqualifying position).  This is a heuristic;
/// an `it` used as a local (`it = …`) or parenless callee is a rare
/// edge not handled in v0.
fn block_uses_implicit_it(inner: &GrammarASTNode) -> bool {
    let mut toks: Vec<&Token> = Vec::new();
    flatten_block_tokens(inner, &mut toks);
    for (i, t) in toks.iter().enumerate() {
        if !matches!(t.type_, TokenType::Name) || t.value != "it" {
            continue;
        }
        let prev_is_dot = i > 0 && matches!(toks[i - 1].type_, TokenType::Dot);
        let next_is_lparen = toks
            .get(i + 1)
            .is_some_and(|n| matches!(n.type_, TokenType::LParen));
        if !prev_is_dot && !next_is_lparen {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Lowerer
// ---------------------------------------------------------------------------

struct Lowerer {
    file_name: String,
    /// Names already bound by a prior `Stmt::LetBinding` in the
    /// current scope.  Drives the `LetBinding` vs `Assign` choice:
    /// first occurrence binds, subsequent occurrences re-assign.
    declared_locals: HashSet<String>,
    /// Phase 6a: parameter names visible in the *current* function
    /// scope.  Empty at the top level (main).  When a Name token is
    /// emitted as a `VarRef`, this set decides whether the `scope`
    /// is `Scope::Param` (the validator's expectation for function
    /// parameters) or `Scope::Local`.
    current_params: HashSet<String>,
    /// Phase 6a: user-defined functions collected from
    /// `def name(params) … end` declarations.  Filled by
    /// `collect_def_statements` (a top-level hoisting pass) before
    /// the main-body lowerer runs.
    user_functions: Vec<Function>,
    /// Phase 6b: SIR features actually exercised by this lowering.
    /// The SIR validator requires manifests to *exactly* match
    /// usage (declared-but-unused is a warning, used-but-undeclared
    /// is an error), so we track on-demand instead of unconditionally
    /// declaring every feature.
    features_used: HashSet<semantic_ir::Feature>,
    /// Phase 6g: monotonically-increasing counter for synthesised
    /// closure-function names.  Each `method_with_block` increments
    /// it once to mint a fresh `__block_<n>` name for the trailing
    /// block's hoisted Function.
    block_counter: usize,
    /// Phase 9a (FC): monotonically-increasing counter for synthesised
    /// multi-assignment temporary names.  Each `multi_assignment` whose
    /// LHS appears in any RHS expression (e.g. the swap `a, b = b, a`)
    /// increments it once and mints fresh `__multi_assign_t<n>_<i>`
    /// temps so the original RHS values are captured *before* any LHS
    /// is rebound — matching Ruby's parallel-binding semantics.
    multi_assign_counter: usize,
    /// Phase 20a (FC): current nesting depth of string-interpolation
    /// re-parsing.  `try_lower_interp_body` re-invokes the Ruby parser
    /// on each `#{…}` body, and a body may itself contain a nested
    /// interpolated string (`"#{ "#{x}" }"`), which recurses back into
    /// the same path.  Neither the parser nor the lowerer has its own
    /// recursion limit, so without this counter an adversarial deeply
    /// nested literal could exhaust the thread stack (an uncatchable
    /// abort — a DoS for any host that compiles untrusted source).  We
    /// stop recursing at `MAX_INTERP_DEPTH` and fall back to the safe
    /// `__interp__` marker, which preserves correctness.
    interp_depth: usize,
    /// Phase Q9e (FC) — names of methods whose body contains a direct
    /// `yield`, discovered while lowering each `def`.  The
    /// explicit-block-param ABI threads a trailing reserved block
    /// parameter (`__sir_block__`) through these methods and rewrites
    /// each in-body `yield` to an `IndirectCall` through that param.
    /// Recording the name here lets a later call-site normalization pass
    /// (Q9f) thread the matching block argument at every call to such a
    /// method.  See [`Lowerer::thread_block_param`].
    block_param_methods: HashSet<String>,
    /// Phase RB2 (FC) — true while lowering a method (`def`) body.  A
    /// `yield` inside a block literal belongs to the *enclosing method*,
    /// so [`Lowerer::hoist_block_to_function`] only threads the enclosing
    /// block-param capture when this is set (at the top level there is no
    /// enclosing method to provide a block, so the block keeps its raw
    /// `yield` — the pre-RB2 behaviour). Saved/restored across nested defs.
    in_def_body: bool,
    /// Phase RB2 (FC) — set by `hoist_block_to_function` when a block it
    /// hoisted captured the enclosing method's `__sir_block__` (its body
    /// `yield`ed).  The enclosing `def` reads this in
    /// [`Lowerer::thread_block_param`] so it gains the trailing
    /// `__sir_block__` parameter even though the `yield` is lexically
    /// inside the block, not the method body.  Reset per def body.
    block_captures_enclosing: bool,
    /// Phase RB2 (FC) — true while lowering a *block* body (set by
    /// `hoist_block_to_function` around its body lowering).  RB2 only
    /// threads the enclosing-block capture for a block lowered **directly**
    /// in a method body, not one nested inside another block: capturing
    /// across two block levels would need the intermediate block to also
    /// re-capture `__sir_block__` (capture chaining), which v0 does not do.
    /// A nested block therefore keeps its raw `yield` (valid SIR) rather
    /// than emitting an invalid cross-level `Param` reference.
    in_block_body: bool,
    /// Milestone O2 (OOP production) — the name of the class whose body /
    /// method bodies are currently being lowered, or `None` at the top
    /// level.  Threaded so `super` inside `Cat#describe` can lower to
    /// `__super__("describe", "Cat", …)`: the runtime needs the *defining*
    /// class name to know where to start the parent-method search.  Set by
    /// the `class_statement` arm around its whole-body lowering and restored
    /// after (so a class following another class does not inherit a stale
    /// name).  Modules do NOT set this — `super` in a module method has no
    /// class to anchor and is out of the v0 OOP-production scope.
    current_class: Option<String>,
    /// Milestone O2 — the name of the method (`def m`) whose body is
    /// currently being lowered, or `None` outside any method.  Threaded
    /// alongside [`Self::current_class`] so a bare/`super(args)` inside the
    /// method knows *which* method to re-dispatch on the parent
    /// (`__super__(method_name, class_name, …)`).  Set by
    /// `lower_def_statement` around its body lowering and restored after.
    current_method: Option<String>,
}

/// Phase Q9e (FC) — the reserved name of the synthesized trailing block
/// parameter threaded through every method that `yield`s.  Chosen with a
/// `__sir_`-prefix so it cannot collide with a user-written Ruby local
/// (those never begin with a double underscore in idiomatic code, and
/// the lowerer never mints another name with this exact spelling).
const BLOCK_PARAM_NAME: &str = "__sir_block__";

/// Phase Q9f/Q10c — context for the call-site block-threading walk over a
/// single function body.
struct BlockNormCtx<'a> {
    /// Methods that gained a trailing `__sir_block__` parameter (Q9e),
    /// i.e. whose calls must have a block argument threaded.
    methods: &'a std::collections::HashSet<String>,
    /// Names bound as a param/local *within the current function*.  A bare
    /// reference to one of these is a variable, not a parenless call, so
    /// Q10c must not rewrite it into a `DirectCall`.
    bound: &'a std::collections::HashSet<String>,
}

/// Phase 20a (FC): hard ceiling on nested string-interpolation
/// re-parsing depth.  Real Ruby almost never nests interpolation more
/// than a level or two (`"a#{ "b#{x}" }"` is already exotic), so 8 is
/// far beyond any legitimate use.  We deliberately keep it small: each
/// recursion level re-enters the full recursive-descent lowering stack
/// (`lower_expression` → … → `try_lower_interp_body`), whose frames are
/// large, so an over-generous cap could itself approach the thread
/// stack limit.  Past the cap we fall back to the safe `__interp__`
/// marker, bounding stack growth regardless of how deeply input nests.
const MAX_INTERP_DEPTH: usize = 8;

impl Lowerer {
    /// Build a `Span` from a node's recorded start/end positions.
    /// Missing positions fall back to a `point` at (0, 0) — fine for
    /// SIR validation purposes.
    fn span_of(&self, node: &GrammarASTNode) -> Span {
        let sl = node.start_line.unwrap_or(0);
        let sc = node.start_column.unwrap_or(0);
        let el = node.end_line.unwrap_or(sl);
        let ec = node.end_column.unwrap_or(sc);
        Span::new(&self.file_name, sl, sc, el, ec)
    }

    fn span_of_token(&self, t: &Token) -> Span {
        Span::point(&self.file_name, t.line, t.column)
    }

    // -------------------------------------------------------------------
    // program → Block
    // -------------------------------------------------------------------

    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Block, RubyLowerError> {
        // Collect the statement nodes (skip whitespace/newline
        // children that the parser may emit).
        let stmts_in: Vec<&GrammarASTNode> = program
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();

        if stmts_in.is_empty() {
            return Ok(Block {
                stmts: Vec::new(),
                value: Expr::NilLit {
                    span: self.span_of(program),
                },
                span: self.span_of(program),
            });
        }

        // The last statement node *may* be promoted to the block's
        // `value` slot — but only if it's an `expression_stmt` (a
        // bare expression with no side-effecting structure around
        // it).  Assignments always stay as statements because they
        // bind a name, and method calls stay as statements because
        // their effects (printing, raising) are observed before any
        // value is consumed.  Exception: if the method call is the
        // sole tail of the program, we still promote it so the
        // module has a meaningful return value.
        let last_idx = stmts_in.len() - 1;
        let mut stmts_out: Vec<Stmt> = Vec::with_capacity(stmts_in.len());
        let mut value: Option<Expr> = None;

        for (i, s) in stmts_in.iter().enumerate() {
            let inner = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                message: "statement node had no child rule".to_string(),
                line: s.start_line.unwrap_or(0),
                column: s.start_column.unwrap_or(0),
            })?;

            let is_tail = i == last_idx;
            let tail_kind = inner.rule_name.as_str();
            if is_tail
                && matches!(
                    tail_kind,
                    "expression_stmt" | "method_call" | "method_call_no_paren"
                )
            {
                // Promote the tail expression to the block's value.
                let v = match tail_kind {
                    "expression_stmt" => {
                        let expr_node =
                            self.first_node_child(inner).ok_or_else(|| RubyLowerError {
                                message: "expression_stmt had no expression child".to_string(),
                                line: inner.start_line.unwrap_or(0),
                                column: inner.start_column.unwrap_or(0),
                            })?;
                        self.lower_expression(expr_node)?
                    }
                    "method_call" | "method_call_no_paren" => self.lower_method_call(inner)?,
                    _ => unreachable!(),
                };
                value = Some(v);
            } else {
                // Phase 6r — use the multi-stmt dispatch wrapper so
                // `multi_assignment` nodes fan out into one SIR Stmt
                // per (lhs[i], rhs[i]) pair.
                stmts_out.extend(self.lower_statement_inner_multi(inner)?);
            }
        }

        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(program),
        });
        // Ruby assignments are sequential: rewrite any `LetBinding` whose RHS
        // reads an earlier local to a `LetStarBinding` (see the fn's doc).
        sequentialize_let_bindings(&mut stmts_out);
        Ok(Block {
            stmts: stmts_out,
            value,
            span: self.span_of(program),
        })
    }

    /// Phase 6r — multi-statement-emitting dispatch wrapper.
    ///
    /// Some Ruby source-statement forms lower to *multiple* SIR
    /// statements:
    ///
    /// - `multi_assignment` (`a, b = 1, 2`) → one `LetBinding`/`Assign`
    ///   per LHS-RHS pair.  The grammar groups them as a single
    ///   surface statement, but at the SIR level they're independent
    ///   bindings.
    ///
    /// Every other statement form produces exactly one SIR statement
    /// and is delegated to [`lower_statement_inner`].  The helper exists
    /// so callers walking a statement list (`lower_program`,
    /// `lower_clause_statements`, `lower_def_statement`, etc.) can
    /// uniformly `.extend(...)` the result instead of `.push(...)`-ing
    /// a single Stmt — keeping the single-stmt path lossless while
    /// permitting multi-stmt fan-out where the grammar warrants it.
    fn lower_statement_inner_multi(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        match node.rule_name.as_str() {
            "multi_assignment" => self.lower_multi_assignment(node),
            "begin_statement" => self.lower_begin_statement(node),
            // O2 (OOP production) — a `class Foo … end` now lowers to MORE than
            // one statement: the `ClassDef` itself PLUS a `__def_method__` /
            // `__def_class_method__` registration for every method it defines
            // (and the synthesized accessors from `attr_*`).  Those
            // registrations must run at program start, right after the
            // `ClassDef`, so `Foo.new` later finds an `initialize` and dispatch
            // finds the instance methods.  Routing `class_statement` through the
            // multi-statement path lets the arm return that whole sequence.
            "class_statement" => self.lower_class_statement_multi(node),
            // MX1 (mixins) — a `module M … end` now lowers to MORE than one
            // statement, exactly like a class: the `ModuleDef` itself PLUS a
            // `__def_method__` registration for every method it defines, and an
            // `__include__` / `__extend__` for every mixin directive in its
            // body.  Those registrations must run at program start, right after
            // the `ModuleDef`, so that a later `include M` (or a class that
            // includes `M`) finds `M`'s methods in the runtime method table.
            // Routing `module_statement` through the multi-statement path lets
            // the arm return that whole sequence (mirrors `class_statement`).
            "module_statement" => self.lower_module_statement_multi(node),
            _ => Ok(vec![self.lower_statement_inner(node)?]),
        }
    }

    /// Lower the inner rule node of a `statement` (one of
    /// `assignment` / `method_call` / `expression_stmt`) into a
    /// `Stmt`.
    fn lower_statement_inner(&mut self, node: &GrammarASTNode) -> Result<Stmt, RubyLowerError> {
        match node.rule_name.as_str() {
            "assignment" => self.lower_assignment(node),
            "rightward_assignment" => self.lower_rightward_assignment(node),
            // Phase 23b (FC) — `defined?(x)` in statement position (the
            // grammar lists `defined_expression` in the statement
            // alternation so a bare `defined?(x)` doesn't get swallowed
            // by `method_call`).  Wrap the lowered operator in ExprStmt.
            "defined_expression" => {
                let expr = self.lower_defined_expression(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "method_call" => {
                let expr = self.lower_method_call(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "method_call_no_paren" => {
                // Phase 6h: paren-less call.  Shape-compatible with
                // `method_call` (same callee + expression-arg layout
                // minus the LPAREN/RPAREN), so the existing
                // `lower_method_call` handles it transparently —
                // both shapes' `expression` children are collected
                // the same way.
                let expr = self.lower_method_call(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "expression_stmt" => {
                let expr_node = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression_stmt had no expression child".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                let expr = self.lower_expression(expr_node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "def_statement" | "endless_def_statement" => {
                // `def` declarations (both the block-bodied form and the
                // Phase-7c endless `def foo = expr` form) were hoisted
                // to top-level Functions in the pre-pass; here we drop
                // them from the main-body statement stream.  Returning
                // a no-op ExprStmt keeps the `Block.stmts` slot occupied
                // but valid SIR-wise.
                Ok(Stmt::ExprStmt {
                    expr: Expr::NilLit {
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "class_statement" => {
                // O2 — the OOP-production path returns the `ClassDef` PLUS its
                // method registrations (see `lower_class_statement_multi`).  A
                // single-`Stmt` caller (only the multi-assignment LHS path,
                // which is never a class) takes just the first statement — the
                // `ClassDef`/`SingletonClassDef` — dropping any registrations.
                // That is safe because a class never appears as a multi-assign
                // target; the real body/program paths all go through
                // `lower_statement_inner_multi`, which keeps the registrations.
                let mut stmts = self.lower_class_statement_multi(node)?;
                Ok(stmts.remove(0))
            }
            "module_statement" => {
                // MX1 (mixins) — the module-production path returns the
                // `ModuleDef` PLUS its method registrations and mixin directives
                // (see `lower_module_statement_multi`).  A single-`Stmt` caller
                // (only the multi-assignment LHS path, which is never a module)
                // takes just the first statement — the `ModuleDef` — dropping any
                // registrations.  That is safe because a module never appears as a
                // multi-assign target; the real body/program paths all go through
                // `lower_statement_inner_multi`, which keeps the registrations.
                let mut stmts = self.lower_module_statement_multi(node)?;
                Ok(stmts.remove(0))
            }
            "if_statement" | "unless_statement" => {
                // Phase 6b: SIR's `Expr::If` is an *expression* — it
                // always yields a value.  We wrap it in `Stmt::ExprStmt`
                // here so the body's value (or NilLit) propagates
                // through the SIR statement stream.
                let expr = self.lower_if_or_unless(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "case_statement" => {
                // Phase 6u — `case x; when v1[,v2,...] then body; else end`.
                //
                // Lower to a chained `Expr::If`:
                //
                //   case x
                //   when 1, 2 then a
                //   when 3    then b
                //   else c
                //   end
                //
                // becomes
                //
                //   if (x == 1 || x == 2) then a
                //   else if x == 3 then b
                //   else c
                //
                // wrapped in `Stmt::ExprStmt`.  Each `when_clause`
                // becomes a single `If` step; the else_clause (or
                // implicit `NilLit` block) terminates the chain.
                let expr = self.lower_case_statement(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "while_statement" | "until_statement" => {
                // Phase 6c: SIR's `Stmt::While` is the canonical
                // top-level loop — `until cond` lowers to
                // `while !cond` (wrap the condition in `not`).
                self.lower_while_or_until(node)
            }
            "method_with_block" => {
                // Phase 6g
                let expr = self.lower_method_with_block(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "modifier_statement" => {
                // Phase 6q: trailing-modifier conditionals/loops.
                // `lhs if cond`    → ExprStmt(If(cond, [lhs], Nil))
                // `lhs unless cond`→ ExprStmt(If(not cond, [lhs], Nil))
                // `lhs while cond` → While(cond, [lhs])
                // `lhs until cond` → While(not cond, [lhs])
                self.lower_modifier_statement(node)
            }
            "yield_statement" => {
                // Phase 6t — `yield` keyword.
                //
                // Grammar shape:
                //   yield_statement = "yield" [ yield_args ] ;
                //   yield_args      = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
                //                   | call_arg { COMMA call_arg } ;
                //
                // Lowering: `BuiltinCall("yield", lowered_args)` wrapped
                // in `Stmt::ExprStmt`.  The `yield_args` wrapper (when
                // present) holds the call_arg subnodes directly; we walk
                // either the statement node or the yield_args wrapper.
                //
                // Effects: PURE.  `yield` invokes the caller-supplied
                // block, whose effects bubble up through the call site's
                // effect set when the block is constructed.  Modelling
                // `yield` itself as PURE keeps the effect lattice from
                // double-counting block effects.
                let yield_args_node = self.find_node_child(node, "yield_args");
                let call_arg_nodes: Vec<&GrammarASTNode> = if let Some(ya) = yield_args_node {
                    ya.children
                        .iter()
                        .filter_map(|c| match c {
                            ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                            _ => None,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let args: Vec<Expr> = call_arg_nodes
                    .into_iter()
                    .map(|n| self.lower_call_arg(n))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Stmt::ExprStmt {
                    expr: Expr::BuiltinCall {
                        name: "yield".to_string(),
                        args,
                        effects: EffectSet::PURE,
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "super_expr" => {
                // Issue #59 — `super` used in EXPRESSION position but reached
                // here as a bare statement (`super`, `super()`, `super(x)`,
                // `super x` written on their own line).  The grammar routes
                // ALL `super` forms through `factor`'s `super_expr` production
                // now (the old statement-only `super_statement` is gone), so a
                // stand-alone `super` line arrives as an `expression_stmt`
                // wrapping a `super_expr`.  The value-producing lowering lives
                // in `lower_super_expr`; wrap its result in an `ExprStmt` for
                // the statement context.
                let expr = self.lower_super_expr(node)?;
                Ok(Stmt::ExprStmt {
                    span: self.span_of(node),
                    expr,
                })
            }
            "return_statement" | "break_statement" | "next_statement" => {
                // Phase 6j: control-flow keywords lower to BuiltinCall
                // with Effect::Divergent.  Optional trailing expression
                // becomes the single arg; bare `return` carries NilLit.
                let name = match node.rule_name.as_str() {
                    "return_statement" => "return",
                    "break_statement" => "break",
                    "next_statement" => "next",
                    _ => unreachable!(),
                };
                let arg_node = self.find_node_child(node, "expression");
                let arg = match arg_node {
                    Some(n) => self.lower_expression(n)?,
                    None => Expr::NilLit {
                        span: self.span_of(node),
                    },
                };
                let expr = Expr::BuiltinCall {
                    name: name.to_string(),
                    args: vec![arg],
                    effects: EffectSet::PURE.with(Effect::Divergent),
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "redo_statement" | "retry_statement" => {
                // Phase 11b/11c: `redo` restarts the current loop
                // iteration without re-checking the condition; `retry`
                // re-executes the enclosing `begin` block from the top
                // (inside a `rescue` clause).  Both are bare keywords
                // that never carry a value, so they lower to a
                // zero-argument Divergent BuiltinCall — distinct from
                // `break`/`next`, which always carry an operand (NilLit
                // when bare).  No `expression` child to inspect.
                let name = match node.rule_name.as_str() {
                    "redo_statement" => "redo",
                    "retry_statement" => "retry",
                    _ => unreachable!(),
                };
                let expr = Expr::BuiltinCall {
                    name: name.to_string(),
                    args: vec![],
                    effects: EffectSet::PURE.with(Effect::Divergent),
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "alias_statement" => {
                // Phase 24a (FC) — `alias new old` aliases the existing
                // method `old` under the name `new`.  Both operands are
                // bare method-name NAME tokens (the symbol forms
                // `alias :new :old` are a deliberate follow-up slice).
                // We lower to a zero-side-effect
                // `BuiltinCall("alias", [StrLit(new), StrLit(old)])`:
                // the method names are surfaced as string literals — they
                // are NOT local variables, so emitting `VarRef`s would be
                // wrong and the SIR validator would reject the never-bound
                // names.  Effects are `PURE`: in this model the alias
                // declaration carries no runtime data effect, mirroring
                // how the other declaration-ish keyword statements behave.
                let names: Vec<&Token> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t),
                        _ => None,
                    })
                    .collect();
                if names.len() != 2 {
                    return Err(RubyLowerError {
                        message: format!(
                            "alias_statement expects 2 name operands, found {}",
                            names.len()
                        ),
                        line: node.start_line.unwrap_or(0),
                        column: node.start_column.unwrap_or(0),
                    });
                }
                let args = names
                    .iter()
                    .map(|t| Expr::StrLit {
                        value: t.value.clone(),
                        span: self.span_of_token(t),
                    })
                    .collect();
                // The method names surface as `StrLit`s, so the module
                // uses the `strings` feature — declare it (the manifest
                // builder allowlist already permits `Feature::Strings`).
                self.features_used.insert(Feature::Strings);
                let expr = Expr::BuiltinCall {
                    name: "alias".to_string(),
                    args,
                    effects: EffectSet::PURE,
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "undef_statement" => {
                // Phase 24b (FC) — `undef name` removes the method `name`
                // from the current class/module.  The single operand is a
                // bare method-name NAME token (the symbol form `undef :name`
                // and the multi-name form `undef a, b` are deliberate
                // follow-up slices).  We lower to a zero-side-effect
                // `BuiltinCall("undef", [StrLit(name)])`, mirroring the
                // Phase 24a `alias` lowering exactly: the method name is
                // surfaced as a string literal — it is NOT a local variable,
                // so emitting a `VarRef` would be wrong and the SIR validator
                // would reject the never-bound name.  Effects are `PURE`: in
                // this model the undef declaration carries no runtime data
                // effect, like the other declaration-ish keyword statements.
                let names: Vec<&Token> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t),
                        _ => None,
                    })
                    .collect();
                if names.len() != 1 {
                    return Err(RubyLowerError {
                        message: format!(
                            "undef_statement expects 1 name operand, found {}",
                            names.len()
                        ),
                        line: node.start_line.unwrap_or(0),
                        column: node.start_column.unwrap_or(0),
                    });
                }
                let args = names
                    .iter()
                    .map(|t| Expr::StrLit {
                        value: t.value.clone(),
                        span: self.span_of_token(t),
                    })
                    .collect();
                // The method name surfaces as a `StrLit`, so the module uses
                // the `strings` feature — declare it (the manifest builder
                // allowlist already permits `Feature::Strings`).
                self.features_used.insert(Feature::Strings);
                let expr = Expr::BuiltinCall {
                    name: "undef".to_string(),
                    args,
                    effects: EffectSet::PURE,
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            other => Err(RubyLowerError {
                message: format!("unsupported statement form `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6b — `if … else … end` / `unless … else … end`
    // -------------------------------------------------------------------

    /// Lower an `if_statement` or `unless_statement` node into an
    /// `Expr::If`.  Both rules have the same shape from the AST's
    /// perspective; the only difference is that `unless`'s
    /// condition is negated.  `elsif` chains nest right — the
    /// `else_branch` of the outermost `If` is itself an `If` for
    /// the first elsif, etc.
    fn lower_if_or_unless(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let is_unless = node.rule_name == "unless_statement";
        // The first `expression` child is the condition.
        let cond_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: format!("{} missing condition expression", node.rule_name),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let mut cond = self.lower_expression(cond_node)?;
        if is_unless {
            // `unless cond` is `if !cond` — wrap in `not` builtin.
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }

        // Then-branch body: every `statement` child *until* the
        // first elsif/else/end terminator.  Since the grammar
        // already segregates elsif/else into their own subnodes,
        // direct `statement` children of `node` are the then-body.
        let then_body = self.lower_clause_statements(node)?;

        // elsif chain — right-associative nesting.  Build the
        // tail starting from `else_clause` and unwind back through
        // any `elsif_clause` nodes in reverse order.
        let elsifs: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "elsif_clause" => Some(n),
                _ => None,
            })
            .collect();
        let else_clause: Option<&GrammarASTNode> = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "else_clause" => Some(n),
            _ => None,
        });

        // Start with the `else` body (or `NilLit` if absent).
        let mut tail = if let Some(ec) = else_clause {
            self.lower_clause_statements(ec)?
        } else {
            Block {
                stmts: Vec::new(),
                value: Expr::NilLit {
                    span: self.span_of(node),
                },
                span: self.span_of(node),
            }
        };

        // Unwind elsif clauses in reverse order, each wrapping the
        // accumulated tail as its own else-branch.
        for ec in elsifs.iter().rev() {
            let ec_cond = self
                .find_node_child(ec, "expression")
                .ok_or_else(|| RubyLowerError {
                    message: "elsif_clause missing condition expression".to_string(),
                    line: ec.start_line.unwrap_or(0),
                    column: ec.start_column.unwrap_or(0),
                })?;
            let ec_cond_expr = self.lower_expression(ec_cond)?;
            let ec_body = self.lower_clause_statements(ec)?;
            tail = Block {
                stmts: Vec::new(),
                value: Expr::If {
                    cond: Box::new(ec_cond_expr),
                    then_branch: Box::new(ec_body),
                    else_branch: Box::new(tail),
                    span: self.span_of(ec),
                },
                span: self.span_of(ec),
            };
        }

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_body),
            else_branch: Box::new(tail),
            span: self.span_of(node),
        })
    }

    /// Lower the `statement` children of a clause node (`if_statement`,
    /// `elsif_clause`, `else_clause`, `unless_statement`) into a
    /// `Block`.  Tail-expression promotion follows the same rule as
    /// `lower_program` — last bare `expression_stmt` / `method_call`
    /// becomes `value`, otherwise `value = NilLit`.
    fn lower_clause_statements(&mut self, node: &GrammarASTNode) -> Result<Block, RubyLowerError> {
        // Phase 6b: each branch is an independent SIR `Block`.
        // Lock the declared-locals set to the outer-scope's snapshot
        // before lowering the body, then restore on exit.  Without
        // this, locals introduced in one `if`-branch would leak
        // into the other branch's scope and cause spurious
        // `Stmt::Assign` emissions (or vice versa).
        let saved_locals = self.declared_locals.clone();
        let stmts_in: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        if stmts_in.is_empty() {
            return Ok(Block {
                stmts: Vec::new(),
                value: Expr::NilLit {
                    span: self.span_of(node),
                },
                span: self.span_of(node),
            });
        }
        let last_idx = stmts_in.len() - 1;
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        for (i, s) in stmts_in.iter().enumerate() {
            let inner = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                message: "statement node had no child rule".to_string(),
                line: s.start_line.unwrap_or(0),
                column: s.start_column.unwrap_or(0),
            })?;
            let is_tail = i == last_idx;
            // Phase FC — the branch's last value-producing statement becomes
            // the branch `Block`'s implicit-return `value`; see
            // [`Self::lower_tail_value`] for the promotion table (which now
            // includes a trailing `if`/`unless`).
            if is_tail {
                if let Some(v) = self.lower_tail_value(inner)? {
                    value = Some(v);
                    continue;
                }
            }
            // Phase 6r — multi-stmt fan-out for `multi_assignment`.
            stmts_out.extend(self.lower_statement_inner_multi(inner)?);
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(node),
        });
        // Restore the outer scope's declared locals.
        self.declared_locals = saved_locals;
        // Sequential-assignment fix-up (see `sequentialize_let_bindings`).
        sequentialize_let_bindings(&mut stmts_out);
        Ok(Block {
            stmts: stmts_out,
            value,
            span: self.span_of(node),
        })
    }

    /// Phase FC — implicit-return tail promotion.
    ///
    /// In Ruby, the *value* of a method (or `if`/`unless` branch) body is
    /// its **last evaluated expression** — there is no explicit `return`.
    /// `def f; a + b; end` returns `a + b`; `def f; if c then a else b end;
    /// end` returns whichever branch ran.  The SIR [`Block`] carries that
    /// value in its dedicated `value` slot (kept distinct from the
    /// side-effecting `stmts`), and every backend emits `value` as the
    /// block's implicit return.  A tail statement therefore has to be
    /// *promoted* out of `stmts` and into `value` — but only if it is a
    /// form that actually produces a usable value.
    ///
    /// This helper is that decision, shared by every body-lowering site so
    /// the rule stays identical everywhere:
    ///
    /// | tail `statement`'s inner rule        | promoted value                     |
    /// |--------------------------------------|------------------------------------|
    /// | `expression_stmt`                    | the bare expression                |
    /// | `method_call` / `method_call_no_paren` | the call's result                |
    /// | `if_statement` / `unless_statement`  | an [`Expr::If`] (branches recurse) |
    /// | `case_statement` (`case/when`, `case/in`) | a chained [`Expr::If`] (arms recurse) |
    /// | anything else (assignment, `while`…) | `None` — the node stays a `Stmt`   |
    ///
    /// Returning `None` tells the caller to route the node through
    /// [`Self::lower_statement_inner_multi`] as an ordinary statement: its
    /// value (if any) is unobservable in Ruby — a trailing `while`, for
    /// instance, evaluates to `nil`, so it stays a `Stmt` and the block's
    /// `value` falls back to `NilLit`.
    ///
    /// **Recursion.** [`Self::lower_if_or_unless`] builds each branch with
    /// [`Self::lower_clause_statements`], which itself calls this helper.  So
    /// a tail `if` whose branch *also* ends in an `if` promotes all the way
    /// down — arbitrarily nested tail conditionals each carry their value,
    /// with no special-casing.
    ///
    /// Note: this is used for method bodies and branch bodies, where Ruby's
    /// implicit return is observable.  A *script's* top-level value is not
    /// language-visible, so [`Self::lower_program`] keeps a bare trailing
    /// `if` as a `Stmt` (see its own promotion list).
    fn lower_tail_value(&mut self, inner: &GrammarASTNode) -> Result<Option<Expr>, RubyLowerError> {
        let value = match inner.rule_name.as_str() {
            "expression_stmt" => {
                let expr_node = self.first_node_child(inner).ok_or_else(|| RubyLowerError {
                    message: "expression_stmt had no expression child".to_string(),
                    line: inner.start_line.unwrap_or(0),
                    column: inner.start_column.unwrap_or(0),
                })?;
                self.lower_expression(expr_node)?
            }
            "method_call" | "method_call_no_paren" => self.lower_method_call(inner)?,
            "if_statement" | "unless_statement" => self.lower_if_or_unless(inner)?,
            // A `case` (both `case/when` value-matching and `case/in` pattern
            // matching) already lowers to a chained `Expr::If` whose arms are
            // built with `lower_clause_statements` — so promoting it here makes
            // a method that ends in a `case` return the matched arm's value
            // (recursing through the same helper), instead of `nil`.
            "case_statement" => self.lower_case_statement(inner)?,
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    // -------------------------------------------------------------------
    // Phase 6c — `while cond … end` / `until cond … end`
    // -------------------------------------------------------------------

    /// Lower a `while_statement` or `until_statement` into a
    /// `Stmt::While`.  `until cond` lowers to `while !cond`
    /// (condition wrapped in `BuiltinCall("not", ...)`).
    fn lower_while_or_until(&mut self, node: &GrammarASTNode) -> Result<Stmt, RubyLowerError> {
        let is_until = node.rule_name == "until_statement";
        let cond_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: format!("{} missing condition expression", node.rule_name),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let mut cond = self.lower_expression(cond_node)?;
        if is_until {
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }
        let body = self.lower_clause_statements(node)?;
        // Phase 6c: the SIR validator requires `loops` to be
        // declared whenever the module emits a `Stmt::While` /
        // `Stmt::ForRange` / `Stmt::ForEach`.
        self.features_used.insert(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(node),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6u — `case … when … else … end`
    // -------------------------------------------------------------------

    /// Lower a `case_statement` node to a chained `Expr::If`.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// case_statement = "case" expression { when_clause } [ else_clause ] "end" ;
    /// when_clause    = "when" expression { COMMA expression }
    ///                       { !"when" !"else" !"end" statement } ;
    /// ```
    ///
    /// Lowering rule:
    ///
    /// ```text
    /// case x
    /// when v1, v2 then body_a
    /// when v3     then body_b
    /// else body_c
    /// end
    /// ```
    ///
    /// becomes
    ///
    /// ```text
    /// if ((x == v1) || (x == v2)) then body_a
    /// else if (x == v3) then body_b
    /// else body_c
    /// ```
    ///
    /// Each `when_clause` produces one nested `If` step.  Multiple values
    /// in a single `when` (`when 1, 2, 3`) chain through `BuiltinCall("or", ...)`
    /// inside that step's condition.  The else terminator (or an empty
    /// `NilLit` block when absent) caps the chain.
    ///
    /// v0 caveats (deferred):
    /// - Ruby's `when` uses `===` (case-equality, class-aware) — this
    ///   v0 lowers to `==`.  Phase 7d adds full `case/in` pattern
    ///   matching with proper match semantics.
    /// - Range/Regex/Class values in `when` lists work syntactically
    ///   (they parse as expressions) but the `==` comparison won't
    ///   match Ruby's case-equality semantics.
    fn lower_case_statement(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // 1. Scrutinee — the first `expression` direct child of the
        //    case_statement.  (subsequent `expression` children belong
        //    to when_clause descendants, but they're inside subnodes,
        //    not direct children.)
        let scrutinee_node =
            self.find_node_child(node, "expression")
                .ok_or_else(|| RubyLowerError {
                    message: "case_statement missing scrutinee expression".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
        let scrutinee = self.lower_expression(scrutinee_node)?;

        // 2. Collect every when_clause / in_clause subnode in source order.
        // Phase 7d — `in_clause` is treated alongside `when_clause` and
        // the two are unwound through the same If-chain pipeline.  The
        // dispatch on clause type happens inside the unwind loop.
        let clauses: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n)
                    if n.rule_name == "when_clause" || n.rule_name == "in_clause" =>
                {
                    Some(n)
                }
                _ => None,
            })
            .collect();

        // 3. Find the optional else_clause (reused from if_statement).
        let else_clause = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "else_clause" => Some(n),
            _ => None,
        });

        // 4. Build the tail: the else block, or an empty NilLit block
        //    if no else clause was provided.
        let mut tail: Block = if let Some(ec) = else_clause {
            self.lower_clause_statements(ec)?
        } else {
            Block {
                stmts: Vec::new(),
                value: Expr::NilLit {
                    span: self.span_of(node),
                },
                span: self.span_of(node),
            }
        };

        // 5. Unwind clauses in reverse, each wrapping the accumulated
        //    tail as its else-branch.  Dispatch on clause type:
        //    when_clause keeps the existing OR-of-`==` shape; in_clause
        //    (Phase 7d) dispatches on pattern kind via
        //    `lower_in_clause_pattern` which returns both a match
        //    condition AND zero or more body-prefix statements (used
        //    for binding patterns that introduce locals on a match).
        for wc in clauses.iter().rev() {
            let span = self.span_of(wc);
            let (cond, mut prefix_stmts) = if wc.rule_name == "when_clause" {
                let cond = self.lower_when_clause_condition(wc, &scrutinee)?;
                (cond, Vec::<Stmt>::new())
            } else {
                // in_clause — find the single `pattern` subnode and
                // dispatch on its inner shape.
                let pattern_node = wc
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Node(n) if n.rule_name == "pattern" => Some(n),
                        _ => None,
                    })
                    .ok_or_else(|| RubyLowerError {
                        message: "in_clause missing pattern".to_string(),
                        line: wc.start_line.unwrap_or(0),
                        column: wc.start_column.unwrap_or(0),
                    })?;
                self.lower_in_clause_pattern(pattern_node, &scrutinee)?
            };

            // Lower the clause body.  Binding-pattern bindings must
            // execute BEFORE the body sees the new local, so prepend
            // them to the body's stmts.
            let mut then_block = self.lower_clause_statements(wc)?;
            if !prefix_stmts.is_empty() {
                prefix_stmts.extend(std::mem::take(&mut then_block.stmts));
                then_block.stmts = prefix_stmts;
            }

            // Wrap into an If and let `tail` become the else.
            tail = Block {
                stmts: Vec::new(),
                value: Expr::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_block),
                    else_branch: Box::new(tail),
                    span: span.clone(),
                },
                span,
            };
        }

        // The case expression is the chain's outermost If — which
        // currently sits as the `tail` Block's `value`.  Peel it out.
        Ok(tail.value)
    }

    /// Phase 6u helper — extract the OR-of-`==` condition for a single
    /// `when_clause` (refactored out of `lower_case_statement` so that
    /// Phase 7d's `in_clause` pattern dispatch can stay symmetric).
    fn lower_when_clause_condition(
        &mut self,
        wc: &GrammarASTNode,
        scrutinee: &Expr,
    ) -> Result<Expr, RubyLowerError> {
        let value_nodes: Vec<&GrammarASTNode> = wc
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .collect();
        if value_nodes.is_empty() {
            return Err(RubyLowerError {
                message: "when_clause missing value expression(s)".to_string(),
                line: wc.start_line.unwrap_or(0),
                column: wc.start_column.unwrap_or(0),
            });
        }
        let span = self.span_of(wc);
        let mut cond: Option<Expr> = None;
        for vn in &value_nodes {
            let val = self.lower_expression(vn)?;
            // M5 — a `when` clause uses Ruby case-equality (`pattern === x`),
            // NOT `==`.  Three shapes need type-aware dispatch:
            //   • a bare constant (`when Integer` / `when MyClass`) → a class
            //     match, lowered to `x.is_a?(Const)` via the `__method__`
            //     dispatch envelope (the backend already passes a `Const`
            //     operand to `is_a?` as its name string, so a built-in class
            //     name needs no binding);
            //   • a range (`when 1..5`) or regex (`when /re/`) literal, plus
            //     any other value → the `case_eq(pattern, x)` runtime helper,
            //     which dispatches Range→membership, Regexp→match, else `==`.
            // The `case_eq` floor is `==`, so plain literals keep their old
            // behaviour.
            let cmp = if matches!(
                val,
                Expr::VarRef {
                    scope: Scope::Const,
                    ..
                }
            ) {
                self.features_used.insert(Feature::Classes);
                // The synthetic `"is_a?"` method-name is a string literal.
                self.features_used.insert(Feature::Strings);
                Expr::BuiltinCall {
                    name: "__method__".to_string(),
                    args: vec![
                        scrutinee.clone(),
                        Expr::StrLit {
                            value: "is_a?".to_string(),
                            span: span.clone(),
                        },
                        val,
                    ],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                }
            } else {
                Expr::BuiltinCall {
                    name: "case_eq".to_string(),
                    args: vec![val, scrutinee.clone()],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                }
            };
            cond = Some(match cond {
                None => cmp,
                Some(prev) => Expr::BuiltinCall {
                    name: "or".to_string(),
                    args: vec![prev, cmp],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                },
            });
        }
        Ok(cond.expect("at least one when value"))
    }

    // -------------------------------------------------------------------
    // Phase 7d — case/in pattern matching
    // -------------------------------------------------------------------

    /// Lower an `in_clause`'s pattern against a scrutinee expression.
    ///
    /// Returns `(cond, prefix_stmts)`:
    /// - `cond`: the boolean expression that decides if this clause
    ///   matches.  When `cond` evaluates true at runtime, the clause's
    ///   body runs; otherwise control falls through to the next clause.
    /// - `prefix_stmts`: zero or more statements (currently always
    ///   `LetBinding`s) that must execute *before* the body sees them.
    ///   Empty for literal / array / hash patterns; populated for
    ///   binding patterns that introduce a fresh local.
    ///
    /// ## Pattern dispatch (v0)
    ///
    /// | Pattern        | cond                                           | prefix_stmts            |
    /// |----------------|------------------------------------------------|-------------------------|
    /// | literal `1`    | `scrutinee == 1`                               | `[]`                    |
    /// | literal `nil`  | `scrutinee == nil`                             | `[]`                    |
    /// | binding `x`    | `BoolLit(true)`                                | `[LetBinding(x, scrut)]`|
    /// | array `[…]`    | structural (`SeqLen`/`SeqIndex` + `&&`)        | per-element bindings    |
    /// | hash `{…}`     | structural (`MapGet` + `&&`, see `lower_hash_pattern`) | per-key bindings |
    ///
    /// ## v0 deferred limitations
    ///
    /// - Array patterns with non-lowerable elements (hash sub-patterns)
    ///   fall back to a `__pattern_match__` marker builtin carrying the
    ///   verbatim raw text of the pattern — downstream emitters can
    ///   re-derive the pattern from the raw text.  Same marker-builtin
    ///   pattern as Phase 6v rescue/ensure, Phase 6y `__interp__`, and
    ///   Phase 7a `backtick`.
    /// - Pin operators (`^x`), find patterns (`[…, *, …]`), and class
    ///   patterns (`SomeClass(x)`) are not yet parsed.
    /// - Hash patterns can't enforce key *presence* (SIR has no map
    ///   has-key primitive); see `lower_hash_pattern` for the precise v0
    ///   semantics.
    fn lower_in_clause_pattern(
        &mut self,
        pattern_node: &GrammarASTNode,
        scrutinee: &Expr,
    ) -> Result<(Expr, Vec<Stmt>), RubyLowerError> {
        let inner = self
            .first_node_child(pattern_node)
            .ok_or_else(|| RubyLowerError {
                message: "pattern node had no inner rule child".to_string(),
                line: pattern_node.start_line.unwrap_or(0),
                column: pattern_node.start_column.unwrap_or(0),
            })?;
        let span = self.span_of(pattern_node);
        match inner.rule_name.as_str() {
            "literal_pattern" => {
                // Lower the literal value, then build `scrutinee == lit`.
                // The literal_pattern node carries exactly one Token
                // child (NUMBER / STRING / KEYWORD) OR a symbol_literal
                // subnode.  We delegate to lower_factor_atom by re-using
                // the factor-atom Token dispatch.
                let lit = self.lower_pattern_literal(inner)?;
                Ok((
                    Expr::BuiltinCall {
                        name: "==".to_string(),
                        args: vec![scrutinee.clone(), lit],
                        effects: EffectSet::PURE,
                        span,
                    },
                    Vec::new(),
                ))
            }
            "binding_pattern" => {
                // Bare NAME — matches any value and binds it.  The
                // condition is trivially true (BoolLit(true)); the
                // binding goes into the body's prefix stmts.
                let name_tok = inner
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                        _ => None,
                    })
                    .ok_or_else(|| RubyLowerError {
                        message: "binding_pattern missing Name token".to_string(),
                        line: inner.start_line.unwrap_or(0),
                        column: inner.start_column.unwrap_or(0),
                    })?;
                let bind_name = name_tok.value.clone();
                self.declared_locals.insert(bind_name.clone());
                let bind_span = self.span_of_token(name_tok);
                let prefix = Stmt::LetBinding {
                    name: bind_name,
                    sir_type: None,
                    value: scrutinee.clone(),
                    span: bind_span,
                };
                Ok((Expr::BoolLit { value: true, span }, vec![prefix]))
            }
            "array_pattern" => {
                // Phase 13a/13b (FC) — a fixed-arity array pattern whose
                // every element is itself lowerable (literal_pattern,
                // binding_pattern, or — Phase 13b — a nested
                // `array_pattern` of the same kind) lowers to a real
                // structural match: a length check ANDed with per-element
                // equality checks and nested sub-conditions, plus a
                // `LetBinding` for every binding element (at any depth).
                // Hash sub-patterns are still unsupported and make the
                // whole pattern fall back to the v0 `__pattern_match__`
                // marker (kept per the Tier-3 marker-replacement
                // convention).
                //
                // Phase FC — splat elements: a SINGLE splat (`[a, *mid, b]`)
                // lowers structurally (relaxed `>=` length check + front/
                // back indexing + optional `__seq_slice__` binding of the
                // middle).  TWO splats (the *find* pattern `[*, x, *]`)
                // would need a contiguous-window search the IR can't
                // express inline, so it falls back to the marker.
                let splats = self.array_pattern_splat_count(inner);
                if splats == 0 {
                    if self.array_pattern_is_lowerable(inner) {
                        self.lower_array_pattern(inner, scrutinee, span)
                    } else {
                        Ok(self.pattern_match_marker(inner, scrutinee, span))
                    }
                } else if splats == 1 && self.array_pattern_is_lowerable(inner) {
                    self.lower_array_pattern_one_splat(inner, scrutinee, span)
                } else {
                    Ok(self.pattern_match_marker(inner, scrutinee, span))
                }
            }
            "hash_pattern" => {
                // Phase FC — hash patterns now lower to a real structural
                // match keyed by symbol (`MapGet`), mirroring the array
                // path.  Every pair's sub-pattern is fed back through
                // `lower_in_clause_pattern` (against `target[:key]`), so
                // literal / binding / nested array / nested hash
                // sub-patterns all compose; the `{name:}` shorthand binds
                // `name = target[:name]`.
                self.lower_hash_pattern(inner, scrutinee, span)
            }
            "pin_pattern" => {
                // Phase FC — `^x` matches iff the scrutinee equals the
                // value of the already-bound local `x`.  Lowers to
                // `scrutinee == x` (an equality `BuiltinCall` over a
                // `VarRef`), with no new binding.  The pinned name is read
                // as a `Scope::Local` (the validator verifies it was
                // bound earlier in scope).
                // The lexer classifies the leading `^` as a `Name` token
                // (value "^"), so skip it and take the pinned identifier.
                let name_tok = inner
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Token(t)
                            if matches!(t.type_, TokenType::Name) && t.value != "^" =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                    .ok_or_else(|| RubyLowerError {
                        message: "pin_pattern missing Name token".to_string(),
                        line: inner.start_line.unwrap_or(0),
                        column: inner.start_column.unwrap_or(0),
                    })?;
                let var = Expr::VarRef {
                    name: name_tok.value.clone(),
                    scope: Scope::Local,
                    span: self.span_of_token(name_tok),
                };
                Ok((
                    Expr::BuiltinCall {
                        name: "==".to_string(),
                        args: vec![scrutinee.clone(), var],
                        effects: EffectSet::PURE,
                        span,
                    },
                    Vec::new(),
                ))
            }
            "class_pattern" => Ok(self.lower_class_pattern(inner, scrutinee, span)?),
            other => Err(RubyLowerError {
                message: format!("unknown pattern kind `{}` in in_clause", other),
                line: inner.start_line.unwrap_or(0),
                column: inner.start_column.unwrap_or(0),
            }),
        }
    }

    /// Phase FC — lower a `class_pattern` (`Foo(p, …)`) against `target`.
    ///
    /// Produces `is_a?(target, "Foo") && <positional deconstruction>`:
    ///
    /// - A class check `BuiltinCall("is_a?", [target, StrLit("Foo")])`.
    ///   The class is surfaced as a `StrLit` of its name (not a `Const`
    ///   `VarRef`) so no constant declaration is required and no
    ///   `Constants` feature is pulled in — the name round-trips for a
    ///   Ruby emitter, same convention as `alias`/`undef`.
    /// - When the pattern lists positional sub-patterns `Foo(a, b)`, they
    ///   match the target's deconstructed elements: a `len(target) == N`
    ///   check plus a recursive match of each inner pattern against
    ///   `target[i]` (reusing `lower_in_clause_pattern` via `SeqIndex`),
    ///   all ANDed after the `is_a?` guard.
    ///
    /// v0 simplification: Ruby calls `#deconstruct` to obtain the array;
    /// here we index `target` directly (assuming it is array-like), the
    /// same modelling `lower_array_pattern` uses.  Requests
    /// `Feature::Strings` (the class-name literal) and, when there are
    /// positional sub-patterns, `Feature::Sequences` + `Feature::ShortCircuit`.
    fn lower_class_pattern(
        &mut self,
        class_inner: &GrammarASTNode,
        target: &Expr,
        span: Span,
    ) -> Result<(Expr, Vec<Stmt>), RubyLowerError> {
        let class_tok = class_inner
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "class_pattern missing class Name token".to_string(),
                line: class_inner.start_line.unwrap_or(0),
                column: class_inner.start_column.unwrap_or(0),
            })?;
        self.features_used.insert(Feature::Strings);
        let mut cond = Expr::BuiltinCall {
            name: "is_a?".to_string(),
            args: vec![
                target.clone(),
                Expr::StrLit {
                    value: class_tok.value.clone(),
                    span: self.span_of_token(class_tok),
                },
            ],
            effects: EffectSet::PURE,
            span: span.clone(),
        };
        let mut prefix: Vec<Stmt> = Vec::new();

        // Positional sub-patterns (the `pattern` children).
        let inners: Vec<&GrammarASTNode> = class_inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "pattern" => Some(n),
                _ => None,
            })
            .collect();
        if !inners.is_empty() {
            self.features_used.insert(Feature::Sequences);
            self.features_used.insert(Feature::ShortCircuit);
            // `len(target) == N`.
            let len_check = Expr::BuiltinCall {
                name: "==".to_string(),
                args: vec![
                    Expr::SeqLen {
                        seq: Box::new(target.clone()),
                        span: span.clone(),
                    },
                    Expr::IntLit {
                        value: inners.len() as i64,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span: span.clone(),
            };
            cond = Expr::LogicalAnd {
                lhs: Box::new(cond),
                rhs: Box::new(len_check),
                span: span.clone(),
            };
            for (i, p) in inners.iter().enumerate() {
                let index = Expr::SeqIndex {
                    seq: Box::new(target.clone()),
                    index: Box::new(Expr::IntLit {
                        value: i as i64,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                };
                let (sub_cond, sub_prefix) = self.lower_in_clause_pattern(p, &index)?;
                cond = Expr::LogicalAnd {
                    lhs: Box::new(cond),
                    rhs: Box::new(sub_cond),
                    span: span.clone(),
                };
                prefix.extend(sub_prefix);
            }
        }
        Ok((cond, prefix))
    }

    /// Phase 13a (FC) — the v0 `__pattern_match__` marker for patterns
    /// we cannot yet lower structurally (hash patterns, array patterns
    /// with nested sub-patterns).  Emits
    /// `BuiltinCall("__pattern_match__", [scrutinee, StrLit(<raw text>)])`
    /// — the raw text round-trips the pattern back to a Ruby emitter.
    /// Returns no body-prefix statements (the marker binds nothing).
    fn pattern_match_marker(
        &mut self,
        inner: &GrammarASTNode,
        scrutinee: &Expr,
        span: Span,
    ) -> (Expr, Vec<Stmt>) {
        let raw = self.pattern_node_raw_text(inner);
        self.features_used.insert(Feature::Strings);
        (
            Expr::BuiltinCall {
                name: "__pattern_match__".to_string(),
                args: vec![
                    scrutinee.clone(),
                    Expr::StrLit {
                        value: raw,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span,
            },
            Vec::new(),
        )
    }

    /// Phase 13b (FC) — collect the element `pattern` subnodes of an
    /// `array_pattern` node, in source order.
    fn array_pattern_elements<'a>(
        &self,
        array_inner: &'a GrammarASTNode,
    ) -> Vec<&'a GrammarASTNode> {
        array_inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "pattern" => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Phase 13b (FC) — can this `array_pattern` node be lowered to a real
    /// structural match?  True when every element is a `literal_pattern`,
    /// a `binding_pattern`, or (recursively) a lowerable nested
    /// `array_pattern`.  Hash sub-patterns (and any other shape) make the
    /// whole pattern fall back to the `__pattern_match__` marker.
    fn array_pattern_is_lowerable(&self, array_inner: &GrammarASTNode) -> bool {
        self.array_pattern_elements(array_inner).iter().all(|p| {
            match self.first_node_child(p) {
                Some(elem) => match elem.rule_name.as_str() {
                    "literal_pattern" | "binding_pattern" => true,
                    "array_pattern" => self.array_pattern_is_lowerable(elem),
                    // Hash sub-patterns now lower structurally too (a
                    // hash pattern is always lowerable — non-lowerable
                    // leaves inside it marker themselves), so an array
                    // containing one no longer forces a whole-pattern
                    // marker fallback.
                    "hash_pattern" => true,
                    _ => false,
                },
                None => false,
            }
        })
    }

    /// Phase 13a/13b (FC) — lower an `array_pattern` node into a real
    /// structural match of `target` (the scrutinee, or a `SeqIndex` into
    /// it for a nested pattern).
    ///
    /// `in [1, x, [2, y]]` against `s` produces:
    ///
    /// ```text
    /// cond   = (((len(s) == 3) && (s[0] == 1))
    ///            && ((len(s[2]) == 2) && (s[2][0] == 2)))
    /// prefix = [ let x = s[1], let y = s[2][1] ]
    /// ```
    ///
    /// Every length check leads its sub-match, and all checks are joined
    /// with the short-circuiting `&&` (`Expr::LogicalAnd`) in the order
    /// outer-length → element → (nested) inner-length → …, so each
    /// `SeqIndex` is only evaluated once the enclosing length check has
    /// held — keeping every index in bounds.  Binding `LetBinding`s run
    /// only in the match arm's body (reached only when the whole
    /// condition held), so they too are in bounds.  Assumes the caller
    /// verified `array_pattern_is_lowerable`.  `Feature::Sequences` is
    /// requested for the `SeqLen` / `SeqIndex` nodes.
    fn lower_array_pattern(
        &mut self,
        array_inner: &GrammarASTNode,
        target: &Expr,
        span: Span,
    ) -> Result<(Expr, Vec<Stmt>), RubyLowerError> {
        self.features_used.insert(Feature::Sequences);
        let elems = self.array_pattern_elements(array_inner);

        // Length check: `len(target) == elems.len()`.
        let mut cond = Expr::BuiltinCall {
            name: "==".to_string(),
            args: vec![
                Expr::SeqLen {
                    seq: Box::new(target.clone()),
                    span: span.clone(),
                },
                Expr::IntLit {
                    value: elems.len() as i64,
                    span: span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span: span.clone(),
        };

        let mut prefix: Vec<Stmt> = Vec::new();
        for (i, p) in elems.iter().enumerate() {
            let elem_inner = self.first_node_child(p).ok_or_else(|| RubyLowerError {
                message: "array_pattern element missing inner pattern".to_string(),
                line: p.start_line.unwrap_or(0),
                column: p.start_column.unwrap_or(0),
            })?;
            // `target[i]` — reused for the equality check, the binding
            // value, or the nested sub-match's target.
            let index = Expr::SeqIndex {
                seq: Box::new(target.clone()),
                index: Box::new(Expr::IntLit {
                    value: i as i64,
                    span: span.clone(),
                }),
                span: span.clone(),
            };
            match elem_inner.rule_name.as_str() {
                "literal_pattern" => {
                    // Append `target[i] == lit` to the AND-chain.
                    let lit = self.lower_pattern_literal(elem_inner)?;
                    let eq = Expr::BuiltinCall {
                        name: "==".to_string(),
                        args: vec![index, lit],
                        effects: EffectSet::PURE,
                        span: span.clone(),
                    };
                    self.features_used.insert(Feature::ShortCircuit);
                    cond = Expr::LogicalAnd {
                        lhs: Box::new(cond),
                        rhs: Box::new(eq),
                        span: span.clone(),
                    };
                }
                "binding_pattern" => {
                    // Bind `name = target[i]` (runs in the match body only).
                    let name_tok = elem_inner
                        .children
                        .iter()
                        .find_map(|c| match c {
                            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => {
                                Some(t)
                            }
                            _ => None,
                        })
                        .ok_or_else(|| RubyLowerError {
                            message: "binding_pattern missing Name token".to_string(),
                            line: elem_inner.start_line.unwrap_or(0),
                            column: elem_inner.start_column.unwrap_or(0),
                        })?;
                    let bind_name = name_tok.value.clone();
                    self.declared_locals.insert(bind_name.clone());
                    prefix.push(Stmt::LetBinding {
                        name: bind_name,
                        sir_type: None,
                        value: index,
                        span: self.span_of_token(name_tok),
                    });
                }
                "array_pattern" => {
                    // Phase 13b — recurse: match `target[i]` against the
                    // nested array pattern.  AND the sub-condition into
                    // the chain (after this level's length check, so the
                    // `target[i]` index is already in bounds) and append
                    // any nested bindings to the prefix.
                    let (sub_cond, sub_prefix) =
                        self.lower_array_pattern(elem_inner, &index, span.clone())?;
                    self.features_used.insert(Feature::ShortCircuit);
                    cond = Expr::LogicalAnd {
                        lhs: Box::new(cond),
                        rhs: Box::new(sub_cond),
                        span: span.clone(),
                    };
                    prefix.extend(sub_prefix);
                }
                "hash_pattern" => {
                    // Phase FC — a hash sub-pattern at array element `i`
                    // matches `target[i]` against the hash pattern (keyed
                    // by symbol via `MapGet`).  Same AND-after-length-check
                    // discipline as the nested-array case.
                    let (sub_cond, sub_prefix) =
                        self.lower_hash_pattern(elem_inner, &index, span.clone())?;
                    self.features_used.insert(Feature::ShortCircuit);
                    cond = Expr::LogicalAnd {
                        lhs: Box::new(cond),
                        rhs: Box::new(sub_cond),
                        span: span.clone(),
                    };
                    prefix.extend(sub_prefix);
                }
                other => {
                    return Err(RubyLowerError {
                        message: format!(
                            "lower_array_pattern reached non-lowerable element `{}`",
                            other
                        ),
                        line: elem_inner.start_line.unwrap_or(0),
                        column: elem_inner.start_column.unwrap_or(0),
                    });
                }
            }
        }
        Ok((cond, prefix))
    }

    /// Phase FC — lower a `hash_pattern` node into a real structural
    /// match of `target`, keyed by symbol.  The hash analogue of
    /// [`lower_array_pattern`].
    ///
    /// `in {name: "ann", age: a}` against `s` produces:
    ///
    /// ```text
    /// cond   = ((true && (s[:name] == "ann")) && true)
    /// prefix = [ let a = s[:age] ]
    /// ```
    ///
    /// Each `hash_pattern_pair` `key: <subpat>` builds a `MapGet(target,
    /// :key)` sub-scrutinee and feeds it back through
    /// [`lower_in_clause_pattern`], so literal, binding, nested array, and
    /// nested hash sub-patterns all compose recursively (no separate
    /// lowerability guard is needed — a non-lowerable nested array simply
    /// markers itself).  The Ruby 3.1 shorthand `{key:}` (no sub-pattern)
    /// binds `key = target[:key]` — fixing the prior "shorthand doesn't
    /// bind" limitation.
    ///
    /// ## v0 limitation
    ///
    /// Ruby hash patterns additionally require each listed key to be
    /// *present* in the scrutinee.  SIR has no map has-key primitive, so
    /// presence is only enforced indirectly: a `key: <literal>` pair
    /// contributes a `target[:key] == literal` check (which fails for a
    /// missing key in every target language whose missing-key value isn't
    /// that literal), while a pure binding/shorthand pair contributes no
    /// guard.  A hash pattern consisting solely of bindings therefore
    /// matches on shape alone.  Requests `Feature::Maps` (for `MapGet`),
    /// `Feature::Symbols` (for the symbol keys), and — when any pair adds
    /// a condition — `Feature::ShortCircuit` (for the `&&` chain).
    fn lower_hash_pattern(
        &mut self,
        hash_inner: &GrammarASTNode,
        target: &Expr,
        span: Span,
    ) -> Result<(Expr, Vec<Stmt>), RubyLowerError> {
        self.features_used.insert(Feature::Maps);
        let pairs: Vec<&GrammarASTNode> = hash_inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_pattern_pair" => Some(n),
                _ => None,
            })
            .collect();

        // Base condition is trivially true; each pair ANDs in its check
        // (literal pairs) or contributes only a binding (binding pairs),
        // exactly mirroring how `binding_pattern` is "trivially true".
        let mut cond = Expr::BoolLit {
            value: true,
            span: span.clone(),
        };
        let mut prefix: Vec<Stmt> = Vec::new();

        for pair in pairs {
            // The key is the leading NAME token; the symbol `:key` is the
            // map key (matching how hash *literals* key their entries —
            // `a:` is sugar for `:a =>`, see `lower_hash_entry`).
            let key_tok = pair
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                    _ => None,
                })
                .ok_or_else(|| RubyLowerError {
                    message: "hash_pattern_pair missing key Name token".to_string(),
                    line: pair.start_line.unwrap_or(0),
                    column: pair.start_column.unwrap_or(0),
                })?;
            let key_name = key_tok.value.clone();
            let key_span = self.span_of_token(key_tok);
            self.features_used.insert(Feature::Symbols);
            // `target[:key]` — reused as the sub-scrutinee / binding value.
            let value = Expr::MapGet {
                map: Box::new(target.clone()),
                key: Box::new(Expr::SymLit {
                    name: key_name.clone(),
                    span: key_span.clone(),
                }),
                span: span.clone(),
            };

            // Optional sub-pattern: `key: <pattern>`.  Absent ⇒ shorthand.
            let subpat = pair.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "pattern" => Some(n),
                _ => None,
            });
            match subpat {
                Some(p) => {
                    // Recurse: match `target[:key]` against the sub-pattern.
                    let (sub_cond, sub_prefix) = self.lower_in_clause_pattern(p, &value)?;
                    self.features_used.insert(Feature::ShortCircuit);
                    cond = Expr::LogicalAnd {
                        lhs: Box::new(cond),
                        rhs: Box::new(sub_cond),
                        span: span.clone(),
                    };
                    prefix.extend(sub_prefix);
                }
                None => {
                    // Shorthand `{key:}` — bind `key = target[:key]`
                    // (runs in the match body only).
                    self.declared_locals.insert(key_name.clone());
                    prefix.push(Stmt::LetBinding {
                        name: key_name,
                        sir_type: None,
                        value,
                        span: key_span,
                    });
                }
            }
        }
        Ok((cond, prefix))
    }

    /// Phase FC — count `splat_pattern` children of an `array_pattern`.
    fn array_pattern_splat_count(&self, array_inner: &GrammarASTNode) -> usize {
        array_inner
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "splat_pattern"))
            .count()
    }

    /// Phase FC — lower a single-splat array pattern `[pre…, *mid, post…]`
    /// against `target`.
    ///
    /// `in [a, *mid, b]` against `s` produces:
    ///
    /// ```text
    /// cond   = (len(s) >= 2) && (<match a vs s[0]>) && (<match b vs s[len-1]>)
    /// prefix = [ let a = s[0], let mid = __seq_slice__(s, 1, len(s)-1), let b = s[len-1] ]
    /// ```
    ///
    /// Fixed (non-splat) elements before the splat index from the front
    /// (`s[i]`); those after the splat index from the back
    /// (`s[len - (post-j)]`).  A named splat binds the middle slice via a
    /// `__seq_slice__(seq, from, to)` marker `BuiltinCall` (SIR has no
    /// sequence-slice primitive); a bare `*` binds nothing.  Each fixed
    /// element is matched by recursing through `lower_in_clause_pattern`,
    /// so literal / binding / nested sub-patterns compose.  The relaxed
    /// `len >= fixed_count` check (vs the exact `==` of the no-splat path)
    /// is what the splat buys.  Requests `Feature::Sequences` +
    /// `Feature::ShortCircuit`.  Assumes exactly one splat (caller checked).
    fn lower_array_pattern_one_splat(
        &mut self,
        array_inner: &GrammarASTNode,
        target: &Expr,
        span: Span,
    ) -> Result<(Expr, Vec<Stmt>), RubyLowerError> {
        self.features_used.insert(Feature::Sequences);
        self.features_used.insert(Feature::ShortCircuit);

        // Walk children in source order, splitting fixed `pattern` nodes
        // into those before vs after the single `splat_pattern`.
        let mut pre: Vec<&GrammarASTNode> = Vec::new();
        let mut post: Vec<&GrammarASTNode> = Vec::new();
        let mut splat_name: Option<String> = None;
        let mut seen_splat = false;
        for c in &array_inner.children {
            if let ASTNodeOrToken::Node(n) = c {
                match n.rule_name.as_str() {
                    "pattern" => {
                        if seen_splat {
                            post.push(n);
                        } else {
                            pre.push(n);
                        }
                    }
                    "splat_pattern" => {
                        seen_splat = true;
                        // The splat's optional rest name (the Name token
                        // that isn't the `*` operator token).
                        splat_name = n.children.iter().find_map(|cc| match cc {
                            ASTNodeOrToken::Token(t)
                                if matches!(t.type_, TokenType::Name) && t.value != "*" =>
                            {
                                Some(t.value.clone())
                            }
                            _ => None,
                        });
                    }
                    _ => {}
                }
            }
        }

        let fixed = pre.len() + post.len();
        let seq_len = || Expr::SeqLen {
            seq: Box::new(target.clone()),
            span: span.clone(),
        };

        // Relaxed length check: `len(target) >= fixed`.
        let mut cond = Expr::BuiltinCall {
            name: ">=".to_string(),
            args: vec![
                seq_len(),
                Expr::IntLit {
                    value: fixed as i64,
                    span: span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span: span.clone(),
        };
        let mut prefix: Vec<Stmt> = Vec::new();

        // Front-anchored fixed elements: target[i] for i in 0..pre.len().
        for (i, p) in pre.iter().enumerate() {
            let index = Expr::SeqIndex {
                seq: Box::new(target.clone()),
                index: Box::new(Expr::IntLit {
                    value: i as i64,
                    span: span.clone(),
                }),
                span: span.clone(),
            };
            let (sub_cond, sub_prefix) = self.lower_in_clause_pattern(p, &index)?;
            cond = Expr::LogicalAnd {
                lhs: Box::new(cond),
                rhs: Box::new(sub_cond),
                span: span.clone(),
            };
            prefix.extend(sub_prefix);
        }

        // Back-anchored fixed elements: target[len - (post.len() - j)].
        for (j, p) in post.iter().enumerate() {
            let from_back = (post.len() - j) as i64;
            let index = Expr::SeqIndex {
                seq: Box::new(target.clone()),
                index: Box::new(Expr::BuiltinCall {
                    name: "-".to_string(),
                    args: vec![
                        seq_len(),
                        Expr::IntLit {
                            value: from_back,
                            span: span.clone(),
                        },
                    ],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                }),
                span: span.clone(),
            };
            let (sub_cond, sub_prefix) = self.lower_in_clause_pattern(p, &index)?;
            cond = Expr::LogicalAnd {
                lhs: Box::new(cond),
                rhs: Box::new(sub_cond),
                span: span.clone(),
            };
            prefix.extend(sub_prefix);
        }

        // Named splat → bind the middle slice `target[pre .. len-post]`
        // via a `__seq_slice__` marker (no first-class slice in SIR).
        if let Some(name) = splat_name {
            self.declared_locals.insert(name.clone());
            let to = Expr::BuiltinCall {
                name: "-".to_string(),
                args: vec![
                    seq_len(),
                    Expr::IntLit {
                        value: post.len() as i64,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span: span.clone(),
            };
            let slice = Expr::BuiltinCall {
                name: "__seq_slice__".to_string(),
                args: vec![
                    target.clone(),
                    Expr::IntLit {
                        value: pre.len() as i64,
                        span: span.clone(),
                    },
                    to,
                ],
                effects: EffectSet::PURE,
                span: span.clone(),
            };
            prefix.push(Stmt::LetBinding {
                name,
                sir_type: None,
                value: slice,
                span: span.clone(),
            });
        }

        Ok((cond, prefix))
    }

    /// Lower a `literal_pattern` Node into its `Expr` form.  Mirrors
    /// the factor-atom token dispatch but narrowed to the patterns the
    /// `literal_pattern` rule admits (NUMBER, STRING, symbol_literal,
    /// KEYWORD).
    fn lower_pattern_literal(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Walk children once to find either a literal Token or a
        // symbol_literal subnode.
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let span = self.span_of_token(tok);
                    match tok.type_ {
                        TokenType::Number => {
                            // Reuse the Phase-6z numeric dispatch so
                            // every shape (float/hex/bin/oct/dec) is
                            // handled identically here.
                            return self
                                .lower_numeric_literal(&tok.value, span, tok.line, tok.column);
                        }
                        TokenType::String => {
                            return Ok(Expr::StrLit {
                                value: tok.value.clone(),
                                span,
                            });
                        }
                        TokenType::Keyword => match tok.value.as_str() {
                            "nil" => return Ok(Expr::NilLit { span }),
                            "true" => return Ok(Expr::BoolLit { value: true, span }),
                            "false" => return Ok(Expr::BoolLit { value: false, span }),
                            other => {
                                return Err(RubyLowerError {
                                    message: format!(
                                        "literal_pattern: unexpected keyword `{}`",
                                        other
                                    ),
                                    line: tok.line,
                                    column: tok.column,
                                });
                            }
                        },
                        _ => {}
                    }
                }
                ASTNodeOrToken::Node(sub) if sub.rule_name == "symbol_literal" => {
                    // Reuse the existing symbol-literal lowering path
                    // (defined as part of Phase 6e).  Delegating into
                    // lower_factor_atom would re-enter the factor
                    // dispatch unnecessarily — instead we just emit
                    // the SymLit directly from the symbol token.
                    return self.lower_symbol_literal(sub);
                }
                _ => {}
            }
        }
        Err(RubyLowerError {
            message: "literal_pattern had no recognisable child".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    /// Best-effort source-text reconstruction for an array/hash
    /// pattern.  Walks the immediate Token children in source order,
    /// joining their values without whitespace insertion — good enough
    /// for the v0 marker (`BuiltinCall("__pattern_match__", …)`) where
    /// the body is round-tripped to a Ruby emitter as-is.  This is
    /// **not** a faithful reformatter; nested patterns are descended
    /// into so their tokens contribute as well.
    fn pattern_node_raw_text(&self, node: &GrammarASTNode) -> String {
        let mut out = String::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) => out.push_str(&t.value),
                ASTNodeOrToken::Node(sub) => {
                    out.push_str(&self.pattern_node_raw_text(sub));
                }
            }
        }
        out
    }

    // -------------------------------------------------------------------
    // Phase 6q — modifier conditionals/loops
    // -------------------------------------------------------------------

    /// Lower a `modifier_statement` node — Ruby's trailing-modifier
    /// surface syntax for one-line `if`/`unless`/`while`/`until`.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// modifier_statement = ( assignment
    ///                      | method_call_no_paren
    ///                      | method_call
    ///                      | expression_stmt )
    ///                      ( "if_modifier" | "unless_modifier"
    ///                      | "while_modifier" | "until_modifier" )
    ///                      expression ;
    /// ```
    ///
    /// AST children layout: `[ lhs_node, modifier_kw_token, cond_node ]`
    /// — the leading group lands a single inner-rule node (one of the
    /// four LHS alternatives), then a keyword token whose value is
    /// `if_modifier`/`unless_modifier`/`while_modifier`/`until_modifier`
    /// (re-tagged by the lexer's `tag_modifier_keywords` post-pass),
    /// then the trailing `expression` node for the condition.
    ///
    /// Lowering table (the table form is reproduced in `ruby.grammar`):
    ///
    /// | Source              | Lowered SIR                                              |
    /// |---------------------|----------------------------------------------------------|
    /// | `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
    /// | `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
    /// | `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
    /// | `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |
    ///
    /// Lowering identity with the leading-keyword forms — same `Expr::If` /
    /// `Stmt::While` shapes — means every downstream emitter
    /// (semantic-ir-to-python / -rust / -typescript / -go) needs zero
    /// new code paths.  The Ruby user sees a syntactic shortcut; the
    /// SIR sees the canonical conditional/loop.
    ///
    /// The LHS body is wrapped in a single-statement `Block` (with
    /// `value: NilLit` — the modifier form is statement-position only,
    /// never tail-promoted to expression).
    fn lower_modifier_statement(&mut self, node: &GrammarASTNode) -> Result<Stmt, RubyLowerError> {
        // 1. Find the LHS inner-rule node.  It's the first child that's
        //    one of the four LHS-eligible rules.
        let lhs_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n)
                    if matches!(
                        n.rule_name.as_str(),
                        "assignment" | "method_call" | "method_call_no_paren" | "expression_stmt"
                    ) =>
                {
                    Some(n)
                }
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing LHS inner-rule node".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 2. Find the modifier keyword token value.  The lexer
        //    guarantees one of the four `*_modifier` values lives in
        //    a Keyword token between LHS and cond.
        let modifier_kw = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(
                        t.value.as_str(),
                        "if_modifier" | "unless_modifier" | "while_modifier" | "until_modifier"
                    ) =>
                {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing modifier keyword token".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 3. Find the cond expression — the LAST `expression` rule
        //    node among direct children.  (LHS may contain nested
        //    `expression` nodes, but those are grand-children of
        //    `modifier_statement`, not direct children.  Using the
        //    last-direct-child position is robust against future
        //    grammar tweaks that might insert intermediate nodes.)
        let cond_node = node
            .children
            .iter()
            .rev()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing condition expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 4. Lower the LHS into a Stmt, then wrap in a single-stmt
        //    Block.  Block.value is NilLit — modifier forms never sit
        //    in tail position.
        let lhs_stmt = self.lower_statement_inner(lhs_node)?;
        let body_block = Block {
            stmts: vec![lhs_stmt],
            value: Expr::NilLit {
                span: self.span_of(node),
            },
            span: self.span_of(node),
        };

        // 5. Lower the condition.  For `unless_modifier` / `until_modifier`,
        //    wrap it in `not` — identical to the leading-keyword
        //    `unless_statement` / `until_statement` lowerings.
        let mut cond = self.lower_expression(cond_node)?;
        let negate = matches!(modifier_kw, "unless_modifier" | "until_modifier");
        if negate {
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }

        // 6. Emit If (conditional modifiers) or While (loop modifiers).
        match modifier_kw {
            "if_modifier" | "unless_modifier" => {
                let else_block = Block {
                    stmts: Vec::new(),
                    value: Expr::NilLit {
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr: Expr::If {
                        cond: Box::new(cond),
                        then_branch: Box::new(body_block),
                        else_branch: Box::new(else_block),
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "while_modifier" | "until_modifier" => {
                self.features_used.insert(Feature::Loops);
                Ok(Stmt::While {
                    cond,
                    body: body_block,
                    span: self.span_of(node),
                })
            }
            // The token-value filter above already rejected anything
            // outside the four valid modifier values; this arm is
            // unreachable.
            other => Err(RubyLowerError {
                message: format!("unknown modifier keyword `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6a — def_statement hoisting
    // -------------------------------------------------------------------

    /// Pre-pass: walk `program` children and lift every
    /// `def_statement` into a top-level `Function` on
    /// `self.user_functions`.  Method bodies are recursively
    /// lowered using a *fresh* declared-locals set so the outer
    /// program's let-bindings don't leak in.
    fn collect_def_statements(&mut self, program: &GrammarASTNode) -> Result<(), RubyLowerError> {
        for child in &program.children {
            let stmt = match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => n,
                _ => continue,
            };
            let inner = match self.first_node_child(stmt) {
                Some(n) => n,
                None => continue,
            };
            // Phase 7c — endless `def foo = expr` is also hoisted as a
            // top-level Function.  Both forms produce a `Function`
            // value; the helper below dispatches on rule name.
            match inner.rule_name.as_str() {
                "def_statement" => {
                    let func = self.lower_def_statement(inner)?;
                    self.user_functions.push(func);
                }
                "endless_def_statement" => {
                    let func = self.lower_endless_def_statement(inner)?;
                    self.user_functions.push(func);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // Phase 6f's `collect_def_statements_from_body` (a whole-body
    // recursive def-hoisting pre-pass) was retired in Phase 14b/14d:
    // `lower_decl_body_statements` now hoists each declaration's direct
    // `def` children itself and delegates nested `class`/`module`
    // declarations to the normal dispatch (whose own arm hoists their
    // direct `def`s), so every method is still hoisted exactly once
    // without a separate pre-pass.

    /// Phase 14b/14d (FC) — lower a `class_statement` *or*
    /// `module_statement` body into the `Vec<Stmt>` carried by
    /// `Stmt::ClassDef.body` / `Stmt::ModuleDef.body`.  Both
    /// declaration forms share identical body semantics, so they share
    /// this helper.
    ///
    /// Walks the declaration body's `statement` children once, in
    /// source order:
    ///
    /// - `def_statement` / `endless_def_statement` are **hoisted** to
    ///   top-level `Function`s on `self.user_functions` (SIR v0 has no
    ///   method-as-statement node, so a method body can't live inside
    ///   a `Vec<Stmt>`).  They contribute *nothing* to `body` — the
    ///   hoisted function is the canonical representation.
    /// - Every other statement (constant/expression assignments, bare
    ///   expressions, nested `class` / `module` declarations, loops, …)
    ///   is lowered via the same [`lower_statement_inner_multi`]
    ///   dispatch used for the program body and pushed onto `body`,
    ///   so it is preserved rather than discarded.
    ///
    /// Hoisting here is per-direct-child.  A nested `class`/`module`
    /// statement is lowered via the normal dispatch, whose own arm
    /// hoists *its* direct `def`s — so every method is hoisted exactly
    /// once and no name is double-registered (which would trip the
    /// validator's function name-uniqueness check).
    fn lower_decl_body_statements(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        let mut body: Vec<Stmt> = Vec::new();
        for child in &node.children {
            let stmt = match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => n,
                _ => continue,
            };
            let inner = match self.first_node_child(stmt) {
                Some(n) => n,
                None => continue,
            };
            match inner.rule_name.as_str() {
                "def_statement" => {
                    let func = self.lower_def_statement(inner)?;
                    self.user_functions.push(func);
                }
                "endless_def_statement" => {
                    let func = self.lower_endless_def_statement(inner)?;
                    self.user_functions.push(func);
                }
                _ => {
                    body.extend(self.lower_statement_inner_multi(inner)?);
                }
            }
        }
        Ok(body)
    }

    // ── O2 (OOP production): class lowering with method registration ──────
    //
    // Milestone O2 wires a Ruby class so it EXECUTES end to end.  Today a
    // class parses and its methods hoist to detached top-level functions, but
    // nothing records that `speak` belongs to `Dog`, `.new` is not connected
    // to `initialize`, and `attr_accessor` is a no-op.  This path emits the
    // missing wiring as ordinary `BuiltinCall`s (NO core-IR change) that the
    // O1 OOP runtime consumes:
    //
    //   • for each instance method `def m`   → `__def_method__("C", "m", ⟨m⟩)`
    //   • for each class method `def self.m` → `__def_class_method__("C","m",⟨m⟩)`
    //   • each `attr_reader`/`attr_writer`/`attr_accessor :x` expands into a
    //     synthesized getter (`def x; @x; end`) and/or setter
    //     (`def x=(v); @x = v; end`), hoisted like a hand-written method AND
    //     registered the same way.
    //
    // where `⟨m⟩` is `MakeClosure { fn_name: <hoisted name>, captures: [] }`
    // — the same hoisted top-level function, referenced by name so it resolves
    // at closure-construction time.  The registrations are returned as
    // statements that follow the `ClassDef` in program order, so they run once
    // at startup before any `Foo.new`.
    //
    // **Class-qualified hoisted names.**  A method defined in a class body is
    // hoisted under a *class-qualified* top-level name — `Dog__speak`, not the
    // bare `speak` (see [`Self::qualified_method_fn_name`]).  This is what makes
    // inheritance + `super` actually work: `Animal#initialize` and
    // `Cat#initialize` must be two DISTINCT top-level functions (bare `initialize`
    // would collide and the validator would reject the duplicate), yet BOTH must
    // be reachable so `super` in `Cat#initialize` can re-run `Animal#initialize`.
    // The double-underscore separator keeps the name a valid identifier and is
    // exceedingly unlikely to collide with a user's own top-level `def`.  The
    // runtime method table is keyed on `(class, bare_method)`, so *dispatch* uses
    // the bare method name; only the shared top-level function symbol is
    // qualified.  (Top-level `def`s — outside any class — keep their bare names,
    // so ordinary function calls are unaffected.)
    //
    // **Single-threaded self model.**  `initialize`/method dispatch runs under
    // the runtime's process-global self-stack (push on entry, pop on exit), so
    // `@ivar` reads/writes and `self` resolve to the right receiver without a
    // threaded `self` parameter.  This is the documented v0 model (see the
    // `sir-runtime-oop` module docs); true per-object/per-thread binding is out
    // of scope.

    /// O2 — lower a `class_statement` into the `ClassDef` (or
    /// `SingletonClassDef`) FOLLOWED BY its method registrations.  Callers that
    /// need a single statement (the multi-assignment LHS path, never a class)
    /// take the first element; the body/program paths keep the whole sequence.
    fn lower_class_statement_multi(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Singleton form `class << RECEIVER … end` — unchanged from Phase 14e.
        // Its methods hoist, but singleton-method *registration* (attaching to
        // a specific object's singleton class) is out of the v0 OOP-production
        // scope, so we emit no `__def_*__` calls here.
        if let Some(target) = self.extract_singleton_receiver(node) {
            self.features_used.insert(Feature::Classes);
            let body = self.lower_decl_body_statements(node)?;
            return Ok(vec![Stmt::SingletonClassDef {
                target,
                body,
                span: self.span_of(node),
            }]);
        }

        // Ordinary `class Foo [< Bar] … end`.
        let name = self.extract_class_name(node)?;
        let superclass = self.extract_superclass(node);
        self.features_used.insert(Feature::Classes);

        // Thread the class name so `super` inside a method body resolves to
        // `__super__(method, "Foo", …)`.  Saved/restored for nested classes.
        let saved_class = self.current_class.take();
        self.current_class = Some(name.clone());

        // Lower the body, collecting method registrations alongside the
        // executable (non-`def`) statements that stay in `ClassDef.body`.
        let (body, registrations) = self.lower_class_body(&name, node)?;

        self.current_class = saved_class;

        let mut out: Vec<Stmt> = Vec::with_capacity(1 + registrations.len());
        out.push(Stmt::ClassDef {
            name,
            superclass,
            body,
            span: self.span_of(node),
        });
        out.extend(registrations);
        Ok(out)
    }

    /// MX1 (mixins) — lower a `module_statement` into the `ModuleDef` FOLLOWED
    /// BY its method registrations and mixin directives.  This mirrors
    /// [`Self::lower_class_statement_multi`] one-for-one, differing only in the
    /// declaration node emitted (`ModuleDef`, which has no superclass) and the
    /// feature it observes (`Feature::Modules`).
    ///
    /// Before MX1, a module body was lowered by `lower_decl_body_statements`,
    /// which hoisted each `def` to a *detached* top-level `Function` and
    /// recorded nothing — so `module M; def greet; …; end; end` produced a
    /// `greet` function that no `include M` could ever find.  MX1 routes the
    /// body through [`Self::lower_class_body`] instead (the SAME builtin path
    /// classes use, keyed by the module name), so each module method now emits
    /// `__def_method__("M", "greet", MakeClosure{…})` into the runtime method
    /// table.  That table is what a later `__include__("C", "M")` copies from,
    /// making the mixin's methods reachable on including classes (MX2+).
    ///
    /// A module's `def self.m` still registers as a class method
    /// (`__def_class_method__`) via the shared body lowerer — a module's
    /// "module function" surface.  `super` inside a module method has no
    /// class to anchor and remains out of the v0 mixin scope (documented in
    /// the OOP spec), so we do NOT set `current_class` here.
    fn lower_module_statement_multi(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        let name = self.extract_module_name(node)?;
        self.features_used.insert(Feature::Modules);

        // Lower the body through the SAME registration-collecting path classes
        // use, keyed by the module name: method `def`s → `__def_method__` /
        // `__def_class_method__`, `include`/`extend` directives → their mixin
        // builtins, everything else preserved in `ModuleDef.body`.
        let (body, registrations) = self.lower_class_body(&name, node)?;

        let mut out: Vec<Stmt> = Vec::with_capacity(1 + registrations.len());
        out.push(Stmt::ModuleDef {
            name,
            body,
            span: self.span_of(node),
        });
        out.extend(registrations);
        Ok(out)
    }

    /// O2 — lower a class body's statements.  Returns `(body, registrations)`:
    /// `body` is the executable non-`def` statements kept in `ClassDef.body`
    /// (constants, nested classes, …, exactly as before); `registrations` is
    /// the sequence of `__def_method__` / `__def_class_method__` builtin-call
    /// statements to run after the `ClassDef`.  Every `def` still hoists to a
    /// top-level `Function` on `self.user_functions` (unchanged); this
    /// additionally records the registration for it.  `attr_*` calls expand
    /// into synthesized accessor functions + their registrations.
    fn lower_class_body(
        &mut self,
        class_name: &str,
        node: &GrammarASTNode,
    ) -> Result<(Vec<Stmt>, Vec<Stmt>), RubyLowerError> {
        let mut body: Vec<Stmt> = Vec::new();
        let mut registrations: Vec<Stmt> = Vec::new();
        for child in &node.children {
            let stmt = match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => n,
                _ => continue,
            };
            let inner = match self.first_node_child(stmt) {
                Some(n) => n,
                None => continue,
            };
            match inner.rule_name.as_str() {
                "def_statement" => {
                    // Issue #59 — a `def_receiver` child (`def self.m` /
                    // `def Recv.m`) marks this as a CLASS method: it registers
                    // in the class-method table (`__def_class_method__`) rather
                    // than the instance-method table.  The hoisted top-level
                    // function name is qualified so it never collides with a
                    // same-named instance method or a class method of another
                    // class (the `_cm` suffix keeps class methods distinct from
                    // the instance method `C__m`).
                    let is_class_method = self.def_has_receiver(inner);
                    let mut func = self.lower_def_statement(inner)?;
                    let method_name = func.name.clone();
                    let fn_name = if is_class_method {
                        self.qualified_class_method_fn_name(class_name, &method_name)
                    } else {
                        self.qualified_method_fn_name(class_name, &method_name)
                    };
                    func.name = fn_name.clone();
                    self.user_functions.push(func);
                    registrations.push(if is_class_method {
                        self.register_class_method(class_name, &method_name, &fn_name)
                    } else {
                        self.register_instance_method(class_name, &method_name, &fn_name)
                    });
                }
                "endless_def_statement" => {
                    let is_class_method = self.def_has_receiver(inner);
                    let mut func = self.lower_endless_def_statement(inner)?;
                    let method_name = func.name.clone();
                    let fn_name = if is_class_method {
                        self.qualified_class_method_fn_name(class_name, &method_name)
                    } else {
                        self.qualified_method_fn_name(class_name, &method_name)
                    };
                    func.name = fn_name.clone();
                    self.user_functions.push(func);
                    registrations.push(if is_class_method {
                        self.register_class_method(class_name, &method_name, &fn_name)
                    } else {
                        self.register_instance_method(class_name, &method_name, &fn_name)
                    });
                }
                "method_call_no_paren" | "method_call" => {
                    // `attr_accessor :x, :y` / `attr_reader` / `attr_writer`
                    // parse as a paren-less (or parenthesized) call inside the
                    // class body.  Intercept those three and expand into
                    // synthesized accessor methods + registrations; anything
                    // else (an ordinary call at class scope) stays in `body`.
                    //
                    // MX1 (mixins) — `include M` / `extend M` parse the same way
                    // (a call whose callee is `include`/`extend` and whose sole
                    // argument is the module constant `M`).  Intercept those and
                    // emit the mixin directive `__include__("Owner", "M")` /
                    // `__extend__("Owner", "M")`, keyed by the enclosing
                    // class/module name — the same registration slot the method
                    // `__def_*__` calls use, so directives run in source order
                    // right after the declaration.
                    if let Some(regs) = self.try_expand_attr_call(class_name, inner)? {
                        registrations.extend(regs);
                    } else if let Some(reg) = self.try_expand_mixin_call(class_name, inner)? {
                        registrations.push(reg);
                    } else {
                        body.extend(self.lower_statement_inner_multi(inner)?);
                    }
                }
                _ => {
                    body.extend(self.lower_statement_inner_multi(inner)?);
                }
            }
        }
        Ok((body, registrations))
    }

    /// O2 — the collision-safe top-level function name a class method hoists to.
    /// `Dog#speak` → `"Dog__speak"`.  Ruby method names may end in `?`/`!`/`=`
    /// (predicate / bang / setter), which are not identifier characters in the
    /// target languages, so they are mapped to word suffixes (`_p` / `_bang` /
    /// `_set`).  A qualified constant path (`Foo::Bar`, `::` in the class name)
    /// has its separators mapped too.  The result is always a valid identifier
    /// and injective enough for the small method sets these programs define.
    fn qualified_method_fn_name(&self, class_name: &str, method_name: &str) -> String {
        let sanitize = |s: &str| -> String {
            s.replace("::", "_")
                .replace('?', "_p")
                .replace('!', "_bang")
                .replace('=', "_set")
        };
        format!("{}__{}", sanitize(class_name), sanitize(method_name))
    }

    /// Issue #59 — the collision-safe top-level function name a CLASS method
    /// (`def self.m`) hoists to: `Counter#self.zero` → `"Counter__zero_cm"`.
    /// Uses the same sanitisation as [`Self::qualified_method_fn_name`] plus a
    /// `_cm` suffix so a class method and an instance method of the *same* name
    /// on the *same* class hoist to DISTINCT top-level functions (Ruby allows
    /// `def m` and `def self.m` to coexist).  The runtime class-method table is
    /// keyed on the bare method name, so dispatch is unaffected by the suffix.
    fn qualified_class_method_fn_name(&self, class_name: &str, method_name: &str) -> String {
        format!(
            "{}_cm",
            self.qualified_method_fn_name(class_name, method_name)
        )
    }

    /// Issue #59 — does this `def_statement` / `endless_def_statement` carry a
    /// `def_receiver` (`def self.m` / `def Recv.m`)?  Presence marks it as a
    /// class/singleton method; absence is an ordinary instance method.
    fn def_has_receiver(&self, node: &GrammarASTNode) -> bool {
        self.find_node_child(node, "def_receiver").is_some()
    }

    /// O2 — build the registration statement for an instance method:
    /// `__def_method__("C", "m", MakeClosure { fn_name, captures: [] })`.  The
    /// table is keyed on the *bare* method name `m` (so dispatch by name works),
    /// while the `MakeClosure` names the *qualified* hoisted top-level function
    /// (`C__m`) so it resolves at construction and does not collide with a
    /// same-named method of another class.  Empty captures — a hoisted method
    /// closes over nothing (its receiver arrives via the runtime self-stack, its
    /// args via the call).
    fn register_instance_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        fn_name: &str,
    ) -> Stmt {
        self.register_method_call("__def_method__", class_name, method_name, fn_name)
    }

    /// O2 / Issue #59 — build the registration statement for a class method
    /// (`def self.m`): `__def_class_method__("C", "m", MakeClosure { fn_name,
    /// captures: [] })`.  Now reachable: the grammar's `def_receiver` production
    /// (#59) lets `def self.m` parse, and `lower_class_body` routes any
    /// receiver-bearing `def` here.
    fn register_class_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        fn_name: &str,
    ) -> Stmt {
        self.register_method_call("__def_class_method__", class_name, method_name, fn_name)
    }

    /// O2 — shared builder for the two registration builtins.
    fn register_method_call(
        &mut self,
        builtin: &str,
        class_name: &str,
        method_name: &str,
        fn_name: &str,
    ) -> Stmt {
        self.features_used.insert(Feature::Closures);
        self.features_used.insert(Feature::Strings);
        let span = Span::point(&self.file_name, 0, 0);
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: builtin.to_string(),
                args: vec![
                    Expr::StrLit {
                        value: class_name.to_string(),
                        span: span.clone(),
                    },
                    Expr::StrLit {
                        value: method_name.to_string(),
                        span: span.clone(),
                    },
                    Expr::MakeClosure {
                        fn_name: fn_name.to_string(),
                        captures: Vec::new(),
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span: span.clone(),
            },
            span,
        }
    }

    /// O2 — if `call_node` is an `attr_reader` / `attr_writer` / `attr_accessor`
    /// invocation, expand each of its symbol arguments into synthesized accessor
    /// method(s), hoist them to `self.user_functions`, and return their
    /// registration statements.  Returns `None` when the call is *not* an
    /// accessor macro (so the caller lowers it as an ordinary class-body
    /// statement).
    ///
    /// The macros:
    ///   • `attr_reader  :x` → getter `def x;  @x;       end`
    ///   • `attr_writer  :x` → setter `def x=(v); @x = v; end`
    ///   • `attr_accessor :x` → both
    /// Each `:x` symbol arg is handled independently, so `attr_accessor :a, :b`
    /// generates accessors for both.
    fn try_expand_attr_call(
        &mut self,
        class_name: &str,
        call_node: &GrammarASTNode,
    ) -> Result<Option<Vec<Stmt>>, RubyLowerError> {
        // The callee is the first Name token directly under the call node.
        let callee = call_node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        let (want_reader, want_writer) = match callee {
            Some("attr_reader") => (true, false),
            Some("attr_writer") => (false, true),
            Some("attr_accessor") => (true, true),
            _ => return Ok(None),
        };

        // Collect the attribute names from the call's symbol arguments.  Each
        // argument is an `expression` subtree that bottoms out in a
        // `symbol_literal` (`:count`); we pull the bare name.  A non-symbol
        // argument (unusual) is skipped defensively rather than erroring.
        let mut attr_names: Vec<String> = Vec::new();
        for c in &call_node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "expression" {
                    if let Some(sym) = self.find_symbol_name_in(n) {
                        attr_names.push(sym);
                    }
                }
            }
        }

        let mut registrations: Vec<Stmt> = Vec::new();
        for attr in &attr_names {
            let ivar = format!("@{attr}");
            let span = Span::point(&self.file_name, 0, 0);
            if want_reader {
                // Getter: `def <attr>; @<attr>; end` → a hoisted function whose
                // body value is the instance-var read.  Registered under the bare
                // `attr`; hoisted under the class-qualified name.
                let getter_fn = self.qualified_method_fn_name(class_name, attr);
                let getter = Function {
                    name: getter_fn.clone(),
                    params: Vec::new(),
                    return_type: None,
                    captures: Vec::new(),
                    body: Block {
                        stmts: Vec::new(),
                        value: Expr::VarRef {
                            name: ivar.clone(),
                            scope: Scope::Instance,
                            span: span.clone(),
                        },
                        span: span.clone(),
                    },
                    effects: EffectSet::PURE,
                    metadata: Metadata::new(),
                    span: span.clone(),
                };
                self.features_used.insert(Feature::InstanceVars);
                self.user_functions.push(getter);
                registrations.push(self.register_instance_method(class_name, attr, &getter_fn));
            }
            if want_writer {
                // Setter: `def <attr>=(v); @<attr> = v; end`.  The method NAME
                // carries Ruby's `=` suffix (`count=`) so `obj.attr = v` — which
                // lowers to `__method__(obj, "attr=", v)` — dispatches here.  The
                // single parameter `v` is `Scope::Param`; the body assigns it to
                // the instance var and the method returns the assigned value
                // (Ruby setters evaluate to their RHS).
                let setter_name = format!("{attr}=");
                let setter_fn = self.qualified_method_fn_name(class_name, &setter_name);
                let assign = Stmt::Assign {
                    name: ivar.clone(),
                    scope: Scope::Instance,
                    value: Expr::VarRef {
                        name: "v".to_string(),
                        scope: Scope::Param,
                        span: span.clone(),
                    },
                    span: span.clone(),
                };
                let setter = Function {
                    name: setter_fn.clone(),
                    params: vec![Param {
                        name: "v".to_string(),
                        sir_type: None,
                        kind: ParamKind::Required,
                        default: None,
                        span: span.clone(),
                    }],
                    return_type: None,
                    captures: Vec::new(),
                    body: Block {
                        stmts: vec![assign],
                        // Return the assigned value (read the ivar back).
                        value: Expr::VarRef {
                            name: ivar.clone(),
                            scope: Scope::Instance,
                            span: span.clone(),
                        },
                        span: span.clone(),
                    },
                    effects: EffectSet::PURE,
                    metadata: Metadata::new(),
                    span: span.clone(),
                };
                self.features_used.insert(Feature::InstanceVars);
                self.features_used.insert(Feature::MutableBindings);
                self.features_used.insert(Feature::DynamicTyping);
                self.user_functions.push(setter);
                registrations.push(self.register_instance_method(
                    class_name,
                    &setter_name,
                    &setter_fn,
                ));
            }
        }
        Ok(Some(registrations))
    }

    /// MX1 (mixins) — if `call_node` is an `include M` / `extend M` directive
    /// inside a class or module body, emit the corresponding mixin builtin
    /// keyed by the enclosing declaration `owner`:
    ///
    ///   • `include M` → `__include__("Owner", "M")`
    ///   • `extend  M` → `__extend__("Owner", "M")`
    ///
    /// Returns `None` when the call is NOT a mixin directive (so the caller
    /// lowers it as an ordinary body statement).  Multiple modules
    /// (`include A, B`) are handled by emitting one directive per module
    /// argument — but that returns a *sequence*; to keep the caller simple we
    /// only special-case the single-module form here (`include M`), which is
    /// the overwhelmingly common shape and all MX1 needs.  A multi-arg
    /// `include A, B` therefore falls through to the ordinary-call path in v0
    /// (documented limitation; MX-later can lift it).
    ///
    /// The module argument `M` is a bare constant, which lowers to
    /// `Expr::VarRef { scope: Scope::Const, name }`.  We mirror how `Foo.new`
    /// and class-method dispatch extract the constant NAME as a `StrLit`
    /// (`lower_dot_call`): the runtime keys its method table on module NAMES,
    /// so the directive must carry the name as a string, not a live value
    /// reference.  This also keeps dispatch table-driven — never reflection on
    /// a source-derived name (the C3 RCE lesson).
    fn try_expand_mixin_call(
        &mut self,
        owner: &str,
        call_node: &GrammarASTNode,
    ) -> Result<Option<Stmt>, RubyLowerError> {
        // The callee is the first Name token directly under the call node.
        let callee = call_node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        let builtin = match callee {
            Some("include") => "__include__",
            Some("extend") => "__extend__",
            _ => return Ok(None),
        };

        // The module operand is the first `expression` argument.  We lower it
        // and require it to be a bare constant ref so we can read its name.
        // Anything else (`include some_expr`, `include A, B`) is left to the
        // ordinary-call path rather than mis-lowering it.
        let arg_node = call_node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
            _ => None,
        });
        let arg_node = match arg_node {
            Some(n) => n,
            None => return Ok(None),
        };
        let lowered = self.lower_expression(arg_node)?;
        let module_name = match lowered {
            Expr::VarRef {
                name,
                scope: Scope::Const,
                ..
            } => name,
            // Not a bare constant — fall through to the ordinary-call path.
            _ => return Ok(None),
        };

        // Emit `__include__/__extend__(StrLit("Owner"), StrLit("M"))`.  The two
        // string args make the directive fully self-describing at the table
        // level; the runtime (MX2+) copies `M`'s registered methods onto
        // `Owner` (include) or `Owner`'s singleton (extend).
        self.features_used.insert(Feature::Modules);
        self.features_used.insert(Feature::Strings);
        let span = Span::point(&self.file_name, 0, 0);
        Ok(Some(Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: builtin.to_string(),
                args: vec![
                    Expr::StrLit {
                        value: owner.to_string(),
                        span: span.clone(),
                    },
                    Expr::StrLit {
                        value: module_name,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span: span.clone(),
            },
            span,
        }))
    }

    /// O2 — find the first `symbol_literal`'s bare name anywhere under `node`
    /// (a shallow recursive walk of the single-child `expression` spine down to
    /// the `factor` that holds the `symbol_literal`).  Returns the symbol name
    /// (`"count"` for `:count`) or `None` if the subtree carries no symbol.
    fn find_symbol_name_in(&self, node: &GrammarASTNode) -> Option<String> {
        if node.rule_name == "symbol_literal" {
            return node.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(
                        t.type_,
                        TokenType::Name | TokenType::Keyword | TokenType::String
                    ) =>
                {
                    Some(t.value.clone())
                }
                _ => None,
            });
        }
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if let Some(found) = self.find_symbol_name_in(n) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Phase 14a (FC) — extract the class name from a `class_statement`
    /// AST node.
    ///
    /// Shape: `KEYWORD("class") NAME { !"end" statement } KEYWORD("end")`.
    /// The class name is the first `TokenType::Name` token in the child
    /// list (the `class` token has `TokenType::Keyword`, so a plain
    /// `expect_first_name_token` would also work, but we prefer an
    /// explicit Name-type filter for symmetry with the analogous
    /// helper inside `lower_def_statement` that extracts the method
    /// name).
    fn extract_class_name(&self, node: &GrammarASTNode) -> Result<String, RubyLowerError> {
        let name_token = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let name_token = name_token.ok_or_else(|| RubyLowerError {
            message: "class_statement missing class-name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        Ok(name_token.value.clone())
    }

    /// Phase 14d (FC) — extract the module name from a
    /// `module_statement` AST node (`module M … end`).
    ///
    /// Shape: `KEYWORD("module") NAME { !"end" statement } "end"`.  The
    /// name is the first `TokenType::Name` token — symmetric with
    /// `extract_class_name`, but with a module-specific error message.
    fn extract_module_name(&self, node: &GrammarASTNode) -> Result<String, RubyLowerError> {
        let name_token = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let name_token = name_token.ok_or_else(|| RubyLowerError {
            message: "module_statement missing module-name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        Ok(name_token.value.clone())
    }

    /// Phase 14c (FC) — extract the optional superclass name from a
    /// `class_statement` AST node (`class Foo < Bar`).
    ///
    /// Grammar shape: `"class" NAME [ "<" NAME ] { … } "end"`.  The
    /// `"<"` separator lexes as a `TokenType::Name` token whose *value*
    /// is `"<"` (the lexer reclassifies comparison operators as Name
    /// tokens; the grammar's `"<"` literal matches by value).  We scan
    /// the direct child tokens for that `<` separator and return the
    /// value of the *next* `Name`-type token — the superclass.  Returns
    /// `None` for a base class (`class Foo`), where no `<` is present.
    ///
    /// Only direct child tokens are inspected, so a `<` appearing deep
    /// inside a body statement (e.g. `a < b` as a comparison) is never
    /// mistaken for the superclass separator: body statements are
    /// `statement` *nodes*, not bare tokens, in the child list.
    fn extract_superclass(&self, node: &GrammarASTNode) -> Option<String> {
        let mut seen_lt = false;
        for child in &node.children {
            if let ASTNodeOrToken::Token(t) = child {
                if seen_lt && matches!(t.type_, TokenType::Name) {
                    return Some(t.value.clone());
                }
                if t.value == "<" {
                    seen_lt = true;
                }
            }
        }
        None
    }

    /// Phase 14e (FC) — extract the singleton-class receiver from a
    /// `class_statement` AST node, if it is the singleton form
    /// `class << RECEIVER … end`.
    ///
    /// The singleton form parses with a `singleton_receiver` child node
    /// (`singleton_receiver = "self" | NAME`); the ordinary
    /// `class Foo [< Bar]` form has none.  Returns `Some(receiver)` —
    /// the value of the receiver token (`"self"` or a bare name) — for
    /// the singleton form, and `None` for the ordinary form (which
    /// signals the caller to take the `ClassDef` path).
    fn extract_singleton_receiver(&self, node: &GrammarASTNode) -> Option<String> {
        let receiver_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "singleton_receiver" => Some(n),
            _ => None,
        })?;
        // The receiver node wraps exactly one token (`self` keyword or a
        // Name) — return its value.
        receiver_node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.clone()),
            _ => None,
        })
    }

    // -------------------------------------------------------------------
    // Phase Q9e (FC) — explicit-block-param ABI, part 1
    // -------------------------------------------------------------------

    /// Thread an explicit trailing block parameter through a freshly
    /// lowered method `Function` *iff* its body `yield`s **or** queries
    /// `block_given?` (Q10b) — either is a use of the method's implicit
    /// block and so requires the threaded `__sir_block__` parameter.
    ///
    /// ## Why
    ///
    /// Ruby's `yield` invokes the block passed implicitly at the call
    /// site — a side channel the narrow-waist SIR has no node for.  The
    /// chosen ABI (see the TRANCHE-2 plan) makes that channel explicit
    /// in the frontend: a method that `yield`s gains a reserved trailing
    /// parameter ([`BLOCK_PARAM_NAME`]) holding the block as an ordinary
    /// closure value, and every `yield` becomes an
    /// [`Expr::IndirectCall`] through that parameter.  Backends already
    /// emit `IndirectCall` (as runtime-core `apply`) and ordinary
    /// `Param`s natively, so **no backend change is needed** — the whole
    /// feature lives in this rewrite plus the Q9f call-site pass that
    /// threads the matching block argument.
    ///
    /// ## What counts as an in-body `yield`
    ///
    /// The walk descends through control-flow and ordinary
    /// expression/call children (so `yield` inside an `if`, a loop, a
    /// `begin/rescue`, or a call argument is rewritten) but deliberately
    /// **stops at [`Expr::MakeClosure`]**: a `yield` lexically inside a
    /// block literal belongs to *that block's* enclosing method, not to
    /// the method we are lowering, so rewriting it here would be wrong.
    /// Handling yield-inside-a-hoisted-block is a documented v0 cut-line.
    /// Nested `def`s never appear as body expressions (the lowerer
    /// hoists them to their own top-level `Function`s), so they need no
    /// special guard.
    ///
    /// Returns the function unchanged when its body contains no direct
    /// `yield` (the common case), so non-yielding methods keep their
    /// original arity and shape exactly.
    fn thread_block_param(&mut self, mut func: Function) -> Function {
        // In the method body, `yield` resolves to the method's own
        // trailing block parameter (`Scope::Param`).
        let found = Self::rewrite_yields_in_block(&mut func.body, Scope::Param);
        // Phase RB2 — a block literal lowered within this method's body may
        // itself have `yield`ed; `hoist_block_to_function` set the flag and
        // captured the enclosing block.  Either signal means this method
        // must gain the trailing `__sir_block__` parameter.
        let needs = found || self.block_captures_enclosing;
        if needs {
            let span = func.span.clone();
            func.params.push(Param {
                name: BLOCK_PARAM_NAME.to_string(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span,
            });
            // The synthesized parameter is untyped (`sir_type: None`),
            // and the rewritten `yield`s introduce `IndirectCall`s; both
            // must be reflected in the feature manifest, which the SIR
            // validator requires to exactly match observed usage.
            self.features_used.insert(Feature::DynamicTyping);
            self.features_used.insert(Feature::Closures);
            self.block_param_methods.insert(func.name.clone());
        }
        func
    }

    /// Rewrite every direct-in-body `yield` within a [`Block`], returning
    /// whether at least one was found.  Recurses through the block's
    /// statements and its trailing value expression.
    fn rewrite_yields_in_block(block: &mut Block, block_scope: Scope) -> bool {
        let mut found = false;
        for s in &mut block.stmts {
            found |= Self::rewrite_yields_in_stmt(s, block_scope);
        }
        found |= Self::rewrite_yields_in_expr(&mut block.value, block_scope);
        found
    }

    /// Rewrite a bare statement list (used by `Stmt::TryCatch` bodies and
    /// rescue/ensure clause bodies, which carry `Vec<Stmt>` with no
    /// trailing value slot).
    fn rewrite_yields_in_stmts(stmts: &mut [Stmt], block_scope: Scope) -> bool {
        let mut found = false;
        for s in stmts {
            found |= Self::rewrite_yields_in_stmt(s, block_scope);
        }
        found
    }

    /// Rewrite every direct-in-body `yield` reachable from a single
    /// statement.  Descends into loop/`while` bodies and `try/catch`
    /// regions, but NOT into class/module/singleton declaration bodies
    /// (whose `def`s — and any `yield`s therein — belong to their own
    /// methods, hoisted separately).
    fn rewrite_yields_in_stmt(stmt: &mut Stmt, block_scope: Scope) -> bool {
        match stmt {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::ExprStmt { expr: value, .. } => {
                Self::rewrite_yields_in_expr(value, block_scope)
            }
            Stmt::While { cond, body, .. } => {
                let mut found = Self::rewrite_yields_in_expr(cond, block_scope);
                found |= Self::rewrite_yields_in_block(body, block_scope);
                found
            }
            Stmt::ForRange {
                start,
                stop,
                step,
                body,
                ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(start, block_scope);
                found |= Self::rewrite_yields_in_expr(stop, block_scope);
                found |= Self::rewrite_yields_in_expr(step, block_scope);
                found |= Self::rewrite_yields_in_block(body, block_scope);
                found
            }
            Stmt::ForEach { iter, body, .. } => {
                let mut found = Self::rewrite_yields_in_expr(iter, block_scope);
                found |= Self::rewrite_yields_in_block(body, block_scope);
                found
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(seq, block_scope);
                found |= Self::rewrite_yields_in_expr(index, block_scope);
                found |= Self::rewrite_yields_in_expr(value, block_scope);
                found
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(map, block_scope);
                found |= Self::rewrite_yields_in_expr(key, block_scope);
                found |= Self::rewrite_yields_in_expr(value, block_scope);
                found
            }
            // SIR22 compile-compat stub: this frontend never emits
            // `IndexSet` today, so this arm is unreachable in practice.
            // It still recurses into every child `Expr` (including each
            // `IndexArg`'s embedded expression), mirroring the `SeqSet` /
            // `MapSet` arms above, so a nested `yield` would still be
            // found and rewritten if a future lowering path ever produced
            // this node inside a `yield`-bearing method body.
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(target, block_scope);
                for idx in indices.iter_mut() {
                    found |= Self::rewrite_yields_in_index_arg(idx, block_scope);
                }
                found |= Self::rewrite_yields_in_expr(value, block_scope);
                found
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                let mut found = Self::rewrite_yields_in_stmts(body, block_scope);
                for r in rescues {
                    found |= Self::rewrite_yields_in_stmts(&mut r.body, block_scope);
                }
                if let Some(eb) = ensure_body {
                    found |= Self::rewrite_yields_in_stmts(eb, block_scope);
                }
                found
            }
            // Class/module/singleton declaration bodies are NOT descended:
            // their method `def`s are hoisted to their own top-level
            // Functions, where any `yield` is rewritten in its own right.
            Stmt::ClassDef { .. } | Stmt::ModuleDef { .. } | Stmt::SingletonClassDef { .. } => {
                false
            }
        }
    }

    /// SIR22 helper for the `yield`-rewrite pass: `IndexArg` wraps an
    /// optional inner `Expr` (see `Expr::IndexGet`/`Stmt::IndexSet`);
    /// recurse into it exactly like any other single-child wrapper above.
    fn rewrite_yields_in_index_arg(idx: &mut IndexArg, block_scope: Scope) -> bool {
        match idx {
            IndexArg::Scalar(e) | IndexArg::Range(e) => {
                Self::rewrite_yields_in_expr(e, block_scope)
            }
            IndexArg::Whole => false,
        }
    }

    /// Rewrite every direct-in-body `yield` reachable from a single
    /// expression.  A `BuiltinCall("yield", args)` is replaced in place
    /// with an `IndirectCall` through the reserved block parameter (after
    /// first rewriting any `yield`s nested in its own `args`).  All other
    /// expression variants recurse into their children — except
    /// [`Expr::MakeClosure`], which is intentionally NOT descended (a
    /// `yield` inside a block literal belongs to the enclosing method).
    fn rewrite_yields_in_expr(expr: &mut Expr, block_scope: Scope) -> bool {
        match expr {
            // SIR26 conversion (not currently emitted here) — recurse.
            Expr::Convert { value, .. } => Self::rewrite_yields_in_expr(value, block_scope),
            Expr::BuiltinCall {
                name,
                args,
                effects,
                span,
            } if name == "yield" => {
                // Rewrite any yields nested within this yield's own
                // arguments first (e.g. `yield(yield x)`), then replace
                // the whole node with the indirect call.
                let mut found = false;
                for a in args.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(a, block_scope);
                }
                let _ = found; // a nested rewrite still counts as "found"
                let span = span.clone();
                let target = Box::new(Expr::VarRef {
                    name: BLOCK_PARAM_NAME.to_string(),
                    scope: block_scope,
                    span: span.clone(),
                });
                *expr = Expr::IndirectCall {
                    target,
                    args: std::mem::take(args),
                    effects: *effects,
                    span,
                };
                true
            }
            Expr::BuiltinCall { args, .. }
            | Expr::DirectCall { args, .. }
            | Expr::Intrinsic { args, .. } => {
                let mut found = false;
                for a in args.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(a, block_scope);
                }
                found
            }
            Expr::IndirectCall { target, args, .. } => {
                let mut found = Self::rewrite_yields_in_expr(target, block_scope);
                for a in args.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(a, block_scope);
                }
                found
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(cond, block_scope);
                found |= Self::rewrite_yields_in_block(then_branch, block_scope);
                found |= Self::rewrite_yields_in_block(else_branch, block_scope);
                found
            }
            Expr::Block(b) => Self::rewrite_yields_in_block(b, block_scope),
            Expr::SeqLit { items, .. } => {
                let mut found = false;
                for i in items.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(i, block_scope);
                }
                found
            }
            Expr::SeqIndex { seq, index, .. } => {
                let mut found = Self::rewrite_yields_in_expr(seq, block_scope);
                found |= Self::rewrite_yields_in_expr(index, block_scope);
                found
            }
            Expr::SeqLen { seq, .. } => Self::rewrite_yields_in_expr(seq, block_scope),
            Expr::MapLit { entries, .. } => {
                let mut found = false;
                for e in entries.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(&mut e.key, block_scope);
                    found |= Self::rewrite_yields_in_expr(&mut e.value, block_scope);
                }
                found
            }
            Expr::MapGet { map, key, .. } => {
                let mut found = Self::rewrite_yields_in_expr(map, block_scope);
                found |= Self::rewrite_yields_in_expr(key, block_scope);
                found
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                let mut found = Self::rewrite_yields_in_expr(lhs, block_scope);
                found |= Self::rewrite_yields_in_expr(rhs, block_scope);
                found
            }
            Expr::StrConcat { parts, .. } => {
                let mut found = false;
                for p in parts.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(p, block_scope);
                }
                found
            }
            // KW1 compile-compat stub: recurse into the `KeywordArg`'s inner
            // `value` (its runtime meaning) so a `yield` nested inside a
            // keyword argument is still rewritten.  Real support pending
            // KW2–KW8.
            Expr::KeywordArg { value, .. } => Self::rewrite_yields_in_expr(value, block_scope),
            // Phase Q10b — `block_given?` reaches the lowerer as a bare
            // `VarRef` named "block_given?" (it is parenless, so the
            // method-call parser treats it as a name).  Inside a method
            // it means "was a block passed?", which under the explicit
            // block-param ABI is exactly "is __sir_block__ non-nil".
            // Rewrite it to `not(null?(__sir_block__))` — both builtins
            // are already supported (a native `not` arm + runtime-core
            // `null?` dispatch) — and count it as a block reference so
            // `thread_block_param` appends the parameter even when the
            // method has no `yield`.
            Expr::VarRef { name, span, .. } if name == "block_given?" => {
                let span = span.clone();
                let block_ref = Expr::VarRef {
                    name: BLOCK_PARAM_NAME.to_string(),
                    scope: block_scope,
                    span: span.clone(),
                };
                let is_nil = Expr::BuiltinCall {
                    name: "null?".to_string(),
                    args: vec![block_ref],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                };
                *expr = Expr::BuiltinCall {
                    name: "not".to_string(),
                    args: vec![is_nil],
                    effects: EffectSet::PURE,
                    span,
                };
                true
            }
            // MakeClosure is deliberately NOT descended (v0 cut-line:
            // yield inside a hoisted block belongs to the enclosing
            // method).  Atomic literals and other VarRefs have no
            // sub-exprs to rewrite.
            Expr::MakeClosure { .. }
            | Expr::IntLit { .. }
            | Expr::BoolLit { .. }
            | Expr::NilLit { .. }
            | Expr::SymLit { .. }
            | Expr::StrLit { .. }
            | Expr::FloatLit { .. }
            | Expr::VarRef { .. } => false,

            // SIR22 compile-compat stubs: unreachable in practice (this
            // frontend never emits these array/matrix nodes), but they
            // recurse into every child `Expr` — same convention as
            // `SeqLit`/`MapLit`/etc. above — so a nested `yield` inside one
            // would still be found if such a node were ever produced.
            Expr::ArrayLit { rows, .. } => {
                let mut found = false;
                for row in rows.iter_mut() {
                    for e in row.iter_mut() {
                        found |= Self::rewrite_yields_in_expr(e, block_scope);
                    }
                }
                found
            }
            Expr::Range {
                start, step, stop, ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(start, block_scope);
                if let Some(s) = step {
                    found |= Self::rewrite_yields_in_expr(s, block_scope);
                }
                found |= Self::rewrite_yields_in_expr(stop, block_scope);
                found
            }
            Expr::MatMul { lhs, rhs, .. } => {
                let mut found = Self::rewrite_yields_in_expr(lhs, block_scope);
                found |= Self::rewrite_yields_in_expr(rhs, block_scope);
                found
            }
            Expr::ElementwiseOp { lhs, rhs, .. } => {
                let mut found = Self::rewrite_yields_in_expr(lhs, block_scope);
                found |= Self::rewrite_yields_in_expr(rhs, block_scope);
                found
            }
            Expr::Transpose { target, .. } => Self::rewrite_yields_in_expr(target, block_scope),
            Expr::IndexGet {
                target, indices, ..
            } => {
                let mut found = Self::rewrite_yields_in_expr(target, block_scope);
                for idx in indices.iter_mut() {
                    found |= Self::rewrite_yields_in_index_arg(idx, block_scope);
                }
                found
            }

            // SIR23 compile-compat stubs: same rationale as the SIR22 stubs
            // above — this frontend never emits any symbolic-expression or
            // pattern/rewrite node today, but every arm still recurses into
            // its children so a nested `yield` would still be found if such
            // a node were ever produced.
            Expr::SymSymbol { .. } | Expr::SymRational { .. } => false,
            Expr::SymApply { head, args, .. } => {
                let mut found = Self::rewrite_yields_in_expr(head, block_scope);
                for a in args.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(a, block_scope);
                }
                found
            }
            Expr::SymPatternBlank { head, .. } => head
                .as_mut()
                .map(|h| Self::rewrite_yields_in_expr(h, block_scope))
                .unwrap_or(false),
            Expr::SymPatternNamed { pattern, .. } => {
                Self::rewrite_yields_in_expr(pattern, block_scope)
            }
            Expr::SymRule { lhs, rhs, .. } => {
                let mut found = Self::rewrite_yields_in_expr(lhs, block_scope);
                found |= Self::rewrite_yields_in_expr(rhs, block_scope);
                found
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                let mut found = Self::rewrite_yields_in_expr(expr, block_scope);
                for r in rules.iter_mut() {
                    found |= Self::rewrite_yields_in_expr(r, block_scope);
                }
                found
            }
        }
    }

    // ===================================================================
    // M4 (FC) — general outer-local captures for hoisted blocks
    //
    // A Ruby block closes over the locals of its enclosing scope:
    //
    //     def f
    //       x = 10
    //       [1, 2, 3].each { |n| puts n + x }   # `x` is captured
    //     end
    //
    // The block body is hoisted to a top-level `__block_<n>` function, so a
    // reference to the enclosing `x` (lowered as `VarRef{scope:Local}`)
    // would be an unbound name inside that function.  M4 detects such free
    // reads, rewrites them to `Scope::Capture`, and threads the enclosing
    // value in as a `MakeClosure` capture (which the backends prepend as a
    // leading parameter).
    //
    // **Capture rule (v0, read-only, single-level).** A name is captured
    // iff it is *read* (`VarRef{Local}`) in the block body, is bound in the
    // *immediate* enclosing scope (a method/outer-block param or local), and
    // is NOT bound inside the block itself (block param, block-local, or a
    // name assigned anywhere in the block body — an in-block assignment
    // makes it block-local, and capture-then-reassign would need
    // by-reference capture, a documented cut-line shared with RB2's nested
    // `yield`).  Capturing a variable two scopes up (capture chaining) is
    // likewise deferred.
    // ===================================================================

    /// Collect every name *bound within* a block body — `Assign` /
    /// `LetBinding` / `LetStarBinding` targets, `for`-loop variables, and a
    /// typed-rescue exception binding.  A bound name shadows any enclosing
    /// binding, so it is excluded from capture.  Nested `MakeClosure`
    /// bodies are hoisted separately and are NOT descended.
    fn collect_bound_names_in_block(block: &Block, out: &mut HashSet<String>) {
        for s in &block.stmts {
            Self::collect_bound_names_in_stmt(s, out);
        }
        Self::collect_bound_names_in_expr(&block.value, out);
    }

    fn collect_bound_names_in_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
        match stmt {
            Stmt::LetBinding { name, value, .. }
            | Stmt::LetStarBinding { name, value, .. }
            | Stmt::Assign { name, value, .. } => {
                out.insert(name.clone());
                Self::collect_bound_names_in_expr(value, out);
            }
            Stmt::ExprStmt { expr, .. } => Self::collect_bound_names_in_expr(expr, out),
            Stmt::While { cond, body, .. } => {
                Self::collect_bound_names_in_expr(cond, out);
                Self::collect_bound_names_in_block(body, out);
            }
            Stmt::ForRange {
                var,
                start,
                stop,
                step,
                body,
                ..
            } => {
                out.insert(var.clone());
                Self::collect_bound_names_in_expr(start, out);
                Self::collect_bound_names_in_expr(stop, out);
                Self::collect_bound_names_in_expr(step, out);
                Self::collect_bound_names_in_block(body, out);
            }
            Stmt::ForEach {
                var, iter, body, ..
            } => {
                out.insert(var.clone());
                Self::collect_bound_names_in_expr(iter, out);
                Self::collect_bound_names_in_block(body, out);
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                Self::collect_bound_names_in_expr(seq, out);
                Self::collect_bound_names_in_expr(index, out);
                Self::collect_bound_names_in_expr(value, out);
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                Self::collect_bound_names_in_expr(map, out);
                Self::collect_bound_names_in_expr(key, out);
                Self::collect_bound_names_in_expr(value, out);
            }
            // SIR22 compile-compat stub (never emitted by this frontend):
            // an index-assignment binds no new name, but its operand
            // expressions can still contain a `MakeClosure` whose captures
            // read outer locals, so recurse into all of them exactly like
            // the `SeqSet`/`MapSet` arms above.
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                Self::collect_bound_names_in_expr(target, out);
                for idx in indices {
                    Self::collect_bound_names_in_index_arg(idx, out);
                }
                Self::collect_bound_names_in_expr(value, out);
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                for s in body {
                    Self::collect_bound_names_in_stmt(s, out);
                }
                for r in rescues {
                    if let Some(n) = &r.binding {
                        out.insert(n.clone());
                    }
                    for s in &r.body {
                        Self::collect_bound_names_in_stmt(s, out);
                    }
                }
                if let Some(eb) = ensure_body {
                    for s in eb {
                        Self::collect_bound_names_in_stmt(s, out);
                    }
                }
            }
            // Declaration bodies hoist their own methods; not descended.
            Stmt::ClassDef { .. } | Stmt::ModuleDef { .. } | Stmt::SingletonClassDef { .. } => {}
        }
    }

    fn collect_bound_names_in_expr(expr: &Expr, out: &mut HashSet<String>) {
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::collect_bound_names_in_expr(cond, out);
                Self::collect_bound_names_in_block(then_branch, out);
                Self::collect_bound_names_in_block(else_branch, out);
            }
            Expr::Block(b) => Self::collect_bound_names_in_block(b, out),
            Expr::BuiltinCall { args, .. }
            | Expr::DirectCall { args, .. }
            | Expr::Intrinsic { args, .. } => {
                for a in args {
                    Self::collect_bound_names_in_expr(a, out);
                }
            }
            Expr::IndirectCall { target, args, .. } => {
                Self::collect_bound_names_in_expr(target, out);
                for a in args {
                    Self::collect_bound_names_in_expr(a, out);
                }
            }
            Expr::SeqLit { items, .. } => {
                for i in items {
                    Self::collect_bound_names_in_expr(i, out);
                }
            }
            Expr::SeqIndex { seq, index, .. } => {
                Self::collect_bound_names_in_expr(seq, out);
                Self::collect_bound_names_in_expr(index, out);
            }
            Expr::SeqLen { seq, .. } => Self::collect_bound_names_in_expr(seq, out),
            Expr::MapLit { entries, .. } => {
                for e in entries {
                    Self::collect_bound_names_in_expr(&e.key, out);
                    Self::collect_bound_names_in_expr(&e.value, out);
                }
            }
            Expr::MapGet { map, key, .. } => {
                Self::collect_bound_names_in_expr(map, out);
                Self::collect_bound_names_in_expr(key, out);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                Self::collect_bound_names_in_expr(lhs, out);
                Self::collect_bound_names_in_expr(rhs, out);
            }
            Expr::StrConcat { parts, .. } => {
                for p in parts {
                    Self::collect_bound_names_in_expr(p, out);
                }
            }
            // MakeClosure not descended; atoms/VarRef bind nothing.
            _ => {}
        }
    }

    /// SIR22 helper for `collect_bound_names_in_stmt`: recurse into an
    /// `IndexArg`'s embedded expression (if any), mirroring the treatment
    /// above of other single-child wrapper shapes.
    fn collect_bound_names_in_index_arg(idx: &IndexArg, out: &mut HashSet<String>) {
        match idx {
            IndexArg::Scalar(e) | IndexArg::Range(e) => Self::collect_bound_names_in_expr(e, out),
            IndexArg::Whole => {}
        }
    }

    /// Rewrite every free *read* (`VarRef{scope:Local}`) of a name for which
    /// `is_free(name)` holds to `Scope::Capture`, recording each captured
    /// name once in first-occurrence order.  Nested `MakeClosure` bodies are
    /// NOT descended (they capture in their own right).
    fn recapture_reads_in_block(
        block: &mut Block,
        is_free: &impl Fn(&str) -> bool,
        found: &mut Vec<String>,
    ) {
        for s in &mut block.stmts {
            Self::recapture_reads_in_stmt(s, is_free, found);
        }
        Self::recapture_reads_in_expr(&mut block.value, is_free, found);
    }

    fn recapture_reads_in_stmt(
        stmt: &mut Stmt,
        is_free: &impl Fn(&str) -> bool,
        found: &mut Vec<String>,
    ) {
        match stmt {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::ExprStmt { expr: value, .. } => {
                Self::recapture_reads_in_expr(value, is_free, found)
            }
            Stmt::While { cond, body, .. } => {
                Self::recapture_reads_in_expr(cond, is_free, found);
                Self::recapture_reads_in_block(body, is_free, found);
            }
            Stmt::ForRange {
                start,
                stop,
                step,
                body,
                ..
            } => {
                Self::recapture_reads_in_expr(start, is_free, found);
                Self::recapture_reads_in_expr(stop, is_free, found);
                Self::recapture_reads_in_expr(step, is_free, found);
                Self::recapture_reads_in_block(body, is_free, found);
            }
            Stmt::ForEach { iter, body, .. } => {
                Self::recapture_reads_in_expr(iter, is_free, found);
                Self::recapture_reads_in_block(body, is_free, found);
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                Self::recapture_reads_in_expr(seq, is_free, found);
                Self::recapture_reads_in_expr(index, is_free, found);
                Self::recapture_reads_in_expr(value, is_free, found);
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                Self::recapture_reads_in_expr(map, is_free, found);
                Self::recapture_reads_in_expr(key, is_free, found);
                Self::recapture_reads_in_expr(value, is_free, found);
            }
            // SIR22 compile-compat stub (never emitted by this frontend):
            // recurse into every child `Expr`, including each `IndexArg`'s
            // embedded expression, mirroring `SeqSet`/`MapSet` above, so a
            // free outer-local read nested inside one is still recaptured.
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                Self::recapture_reads_in_expr(target, is_free, found);
                for idx in indices.iter_mut() {
                    Self::recapture_reads_in_index_arg(idx, is_free, found);
                }
                Self::recapture_reads_in_expr(value, is_free, found);
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                for s in body {
                    Self::recapture_reads_in_stmt(s, is_free, found);
                }
                for r in rescues {
                    for s in &mut r.body {
                        Self::recapture_reads_in_stmt(s, is_free, found);
                    }
                }
                if let Some(eb) = ensure_body {
                    for s in eb {
                        Self::recapture_reads_in_stmt(s, is_free, found);
                    }
                }
            }
            Stmt::ClassDef { .. } | Stmt::ModuleDef { .. } | Stmt::SingletonClassDef { .. } => {}
        }
    }

    /// SIR22 helper for `recapture_reads_in_stmt`: recurse into an
    /// `IndexArg`'s embedded expression (if any).
    fn recapture_reads_in_index_arg(
        idx: &mut IndexArg,
        is_free: &impl Fn(&str) -> bool,
        found: &mut Vec<String>,
    ) {
        match idx {
            IndexArg::Scalar(e) | IndexArg::Range(e) => {
                Self::recapture_reads_in_expr(e, is_free, found)
            }
            IndexArg::Whole => {}
        }
    }

    fn recapture_reads_in_expr(
        expr: &mut Expr,
        is_free: &impl Fn(&str) -> bool,
        found: &mut Vec<String>,
    ) {
        match expr {
            Expr::VarRef { name, scope, .. } if *scope == Scope::Local && is_free(name) => {
                if !found.iter().any(|n| n == name) {
                    found.push(name.clone());
                }
                *scope = Scope::Capture;
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::recapture_reads_in_expr(cond, is_free, found);
                Self::recapture_reads_in_block(then_branch, is_free, found);
                Self::recapture_reads_in_block(else_branch, is_free, found);
            }
            Expr::Block(b) => Self::recapture_reads_in_block(b, is_free, found),
            Expr::BuiltinCall { args, .. }
            | Expr::DirectCall { args, .. }
            | Expr::Intrinsic { args, .. } => {
                for a in args.iter_mut() {
                    Self::recapture_reads_in_expr(a, is_free, found);
                }
            }
            Expr::IndirectCall { target, args, .. } => {
                Self::recapture_reads_in_expr(target, is_free, found);
                for a in args.iter_mut() {
                    Self::recapture_reads_in_expr(a, is_free, found);
                }
            }
            Expr::SeqLit { items, .. } => {
                for i in items.iter_mut() {
                    Self::recapture_reads_in_expr(i, is_free, found);
                }
            }
            Expr::SeqIndex { seq, index, .. } => {
                Self::recapture_reads_in_expr(seq, is_free, found);
                Self::recapture_reads_in_expr(index, is_free, found);
            }
            Expr::SeqLen { seq, .. } => Self::recapture_reads_in_expr(seq, is_free, found),
            Expr::MapLit { entries, .. } => {
                for e in entries.iter_mut() {
                    Self::recapture_reads_in_expr(&mut e.key, is_free, found);
                    Self::recapture_reads_in_expr(&mut e.value, is_free, found);
                }
            }
            Expr::MapGet { map, key, .. } => {
                Self::recapture_reads_in_expr(map, is_free, found);
                Self::recapture_reads_in_expr(key, is_free, found);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                Self::recapture_reads_in_expr(lhs, is_free, found);
                Self::recapture_reads_in_expr(rhs, is_free, found);
            }
            Expr::StrConcat { parts, .. } => {
                for p in parts.iter_mut() {
                    Self::recapture_reads_in_expr(p, is_free, found);
                }
            }
            // MakeClosure not descended; remaining atoms/VarRefs untouched.
            _ => {}
        }
    }

    // -------------------------------------------------------------------
    // Phase Q9f (FC) — explicit block-param ABI, part 2: call-site
    // normalization
    // -------------------------------------------------------------------

    /// Thread the matching block argument at every `DirectCall` to a
    /// method that gained a trailing `__sir_block__` parameter (recorded
    /// in `blk` by Q9e's [`Lowerer::thread_block_param`]).
    ///
    /// For each such call, the trailing argument slot is normalized so
    /// the call's arity matches the threaded def's (one extra trailing
    /// block parameter):
    ///
    /// - trailing arg is a `MakeClosure` (`foo { … }` / `foo do … end`) —
    ///   already the block; **left as-is**.
    /// - trailing arg is `BuiltinCall("block_pass", [inner])` (`foo(&p)`)
    ///   — **unwrapped** to `inner`, the proc/block value itself.
    /// - otherwise (`foo`, `foo(1, 2)`) — **append `NilLit`**: no block
    ///   was passed, so the parameter binds nil (and a later `yield`
    ///   through a nil block is the documented v0 LocalJumpError analogue).
    ///
    /// The walk descends through every statement, expression, and nested
    /// call (including `MakeClosure` capture values) so calls anywhere in
    /// the program are threaded. It is idempotent in practice because it
    /// runs exactly once, after the whole program is lowered.
    fn normalize_block_call_args(block: &mut Block, ctx: &BlockNormCtx) {
        for s in &mut block.stmts {
            Self::normalize_calls_in_stmt(s, ctx);
        }
        Self::normalize_calls_in_expr(&mut block.value, ctx);
    }

    fn normalize_calls_in_stmts(stmts: &mut [Stmt], ctx: &BlockNormCtx) {
        for s in stmts {
            Self::normalize_calls_in_stmt(s, ctx);
        }
    }

    fn normalize_calls_in_stmt(stmt: &mut Stmt, ctx: &BlockNormCtx) {
        match stmt {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::ExprStmt { expr: value, .. } => Self::normalize_calls_in_expr(value, ctx),
            Stmt::While { cond, body, .. } => {
                Self::normalize_calls_in_expr(cond, ctx);
                Self::normalize_block_call_args(body, ctx);
            }
            Stmt::ForRange {
                start,
                stop,
                step,
                body,
                ..
            } => {
                Self::normalize_calls_in_expr(start, ctx);
                Self::normalize_calls_in_expr(stop, ctx);
                Self::normalize_calls_in_expr(step, ctx);
                Self::normalize_block_call_args(body, ctx);
            }
            Stmt::ForEach { iter, body, .. } => {
                Self::normalize_calls_in_expr(iter, ctx);
                Self::normalize_block_call_args(body, ctx);
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                Self::normalize_calls_in_expr(seq, ctx);
                Self::normalize_calls_in_expr(index, ctx);
                Self::normalize_calls_in_expr(value, ctx);
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                Self::normalize_calls_in_expr(map, ctx);
                Self::normalize_calls_in_expr(key, ctx);
                Self::normalize_calls_in_expr(value, ctx);
            }
            // SIR22 compile-compat stub (never emitted by this frontend):
            // recurse into every child `Expr`, including each `IndexArg`'s
            // embedded expression, mirroring `SeqSet`/`MapSet` above, so a
            // parenless block-method call nested inside one is still
            // threaded/normalized.
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                Self::normalize_calls_in_expr(target, ctx);
                for idx in indices.iter_mut() {
                    Self::normalize_calls_in_index_arg(idx, ctx);
                }
                Self::normalize_calls_in_expr(value, ctx);
            }
            // Class/module/singleton bodies carry non-`def` statements
            // (their `def`s are hoisted to top-level functions, which the
            // outer loop over `functions` already visits) — descend so a
            // call inside e.g. a constant initializer is threaded too.
            Stmt::ClassDef { body, .. }
            | Stmt::ModuleDef { body, .. }
            | Stmt::SingletonClassDef { body, .. } => Self::normalize_calls_in_stmts(body, ctx),
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                Self::normalize_calls_in_stmts(body, ctx);
                for r in rescues {
                    Self::normalize_calls_in_stmts(&mut r.body, ctx);
                }
                if let Some(eb) = ensure_body {
                    Self::normalize_calls_in_stmts(eb, ctx);
                }
            }
        }
    }

    fn normalize_calls_in_expr(expr: &mut Expr, ctx: &BlockNormCtx) {
        match expr {
            // SIR26 conversion (not currently emitted here) — recurse.
            Expr::Convert { value, .. } => Self::normalize_calls_in_expr(value, ctx),
            // Phase Q10c — a bare, parenless reference to a known
            // block-taking method (`foo` with no `()`/args) reaches the
            // lowerer as `VarRef { scope: Local }` (the method-call parser
            // can't tell a zero-arg call from a variable).  When the name
            // is a block-param method AND is not shadowed by a real
            // local/param in this function, it is actually a call: rewrite
            // it to a `DirectCall` with a threaded nil block so its arity
            // matches the def's trailing `__sir_block__` parameter.  The
            // synthesized call already carries its block slot, so it is
            // not re-walked (no double-padding).
            Expr::VarRef {
                name,
                scope: Scope::Local,
                span,
            } if ctx.methods.contains(name) && !ctx.bound.contains(name) => {
                let span = span.clone();
                *expr = Expr::DirectCall {
                    fn_name: name.clone(),
                    args: vec![Expr::NilLit { span: span.clone() }],
                    effects: EffectSet::PURE,
                    span,
                };
            }
            Expr::DirectCall {
                fn_name,
                args,
                span,
                ..
            } => {
                // Normalize nested calls in the arguments first (so an
                // unwrapped `block_pass` inner / a closure capture value
                // is itself threaded), then fix this call's trailing slot.
                for a in args.iter_mut() {
                    Self::normalize_calls_in_expr(a, ctx);
                }
                if ctx.methods.contains(fn_name) {
                    let n = args.len();
                    let trailing_is_closure =
                        n > 0 && matches!(args[n - 1], Expr::MakeClosure { .. });
                    let trailing_is_block_pass = n > 0
                        && matches!(&args[n - 1],
                            Expr::BuiltinCall { name, .. } if name == "block_pass");
                    if trailing_is_block_pass {
                        // Unwrap `block_pass(inner)` → `inner` (the proc
                        // value passed as the block). A malformed envelope
                        // (not exactly one operand) is left untouched.
                        if let Expr::BuiltinCall { args: inner, .. } = &mut args[n - 1] {
                            if inner.len() == 1 {
                                let v = inner.remove(0);
                                args[n - 1] = v;
                            }
                        }
                    } else if !trailing_is_closure {
                        // No block syntactically passed: bind nil.
                        args.push(Expr::NilLit { span: span.clone() });
                    }
                }
            }
            Expr::BuiltinCall { args, .. } | Expr::Intrinsic { args, .. } => {
                for a in args.iter_mut() {
                    Self::normalize_calls_in_expr(a, ctx);
                }
            }
            Expr::IndirectCall { target, args, .. } => {
                Self::normalize_calls_in_expr(target, ctx);
                for a in args.iter_mut() {
                    Self::normalize_calls_in_expr(a, ctx);
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::normalize_calls_in_expr(cond, ctx);
                Self::normalize_block_call_args(then_branch, ctx);
                Self::normalize_block_call_args(else_branch, ctx);
            }
            Expr::Block(b) => Self::normalize_block_call_args(b, ctx),
            Expr::MakeClosure { captures, .. } => {
                for c in captures.iter_mut() {
                    Self::normalize_calls_in_expr(&mut c.value, ctx);
                }
            }
            Expr::SeqLit { items, .. } => {
                for i in items.iter_mut() {
                    Self::normalize_calls_in_expr(i, ctx);
                }
            }
            Expr::SeqIndex { seq, index, .. } => {
                Self::normalize_calls_in_expr(seq, ctx);
                Self::normalize_calls_in_expr(index, ctx);
            }
            Expr::SeqLen { seq, .. } => Self::normalize_calls_in_expr(seq, ctx),
            Expr::MapLit { entries, .. } => {
                for e in entries.iter_mut() {
                    Self::normalize_calls_in_expr(&mut e.key, ctx);
                    Self::normalize_calls_in_expr(&mut e.value, ctx);
                }
            }
            Expr::MapGet { map, key, .. } => {
                Self::normalize_calls_in_expr(map, ctx);
                Self::normalize_calls_in_expr(key, ctx);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                Self::normalize_calls_in_expr(lhs, ctx);
                Self::normalize_calls_in_expr(rhs, ctx);
            }
            Expr::StrConcat { parts, .. } => {
                for p in parts.iter_mut() {
                    Self::normalize_calls_in_expr(p, ctx);
                }
            }
            // KW1 compile-compat stub: recurse into the `KeywordArg`'s inner
            // `value` so a parenless block-method call nested in a keyword
            // argument is still normalized.  Real support pending KW2–KW8.
            Expr::KeywordArg { value, .. } => {
                Self::normalize_calls_in_expr(value, ctx);
            }
            // SIR22 compile-compat stubs (never emitted by this frontend):
            // recurse into every child `Expr`, matching the treatment of
            // `SeqLit`/`MapLit`/etc. above, so a parenless block-method call
            // nested inside one of these would still be normalized.
            Expr::ArrayLit { rows, .. } => {
                for row in rows.iter_mut() {
                    for e in row.iter_mut() {
                        Self::normalize_calls_in_expr(e, ctx);
                    }
                }
            }
            Expr::Range {
                start, step, stop, ..
            } => {
                Self::normalize_calls_in_expr(start, ctx);
                if let Some(s) = step {
                    Self::normalize_calls_in_expr(s, ctx);
                }
                Self::normalize_calls_in_expr(stop, ctx);
            }
            Expr::MatMul { lhs, rhs, .. } => {
                Self::normalize_calls_in_expr(lhs, ctx);
                Self::normalize_calls_in_expr(rhs, ctx);
            }
            Expr::ElementwiseOp { lhs, rhs, .. } => {
                Self::normalize_calls_in_expr(lhs, ctx);
                Self::normalize_calls_in_expr(rhs, ctx);
            }
            Expr::Transpose { target, .. } => Self::normalize_calls_in_expr(target, ctx),
            Expr::IndexGet {
                target, indices, ..
            } => {
                Self::normalize_calls_in_expr(target, ctx);
                for idx in indices.iter_mut() {
                    Self::normalize_calls_in_index_arg(idx, ctx);
                }
            }
            // SIR23 compile-compat stubs (never emitted by this frontend):
            // recurse into every child `Expr`, matching the SIR22 stubs
            // above, so a parenless block-method call nested inside one of
            // these would still be normalized.
            Expr::SymSymbol { .. } | Expr::SymRational { .. } => {}
            Expr::SymApply { head, args, .. } => {
                Self::normalize_calls_in_expr(head, ctx);
                for a in args.iter_mut() {
                    Self::normalize_calls_in_expr(a, ctx);
                }
            }
            Expr::SymPatternBlank { head, .. } => {
                if let Some(h) = head {
                    Self::normalize_calls_in_expr(h, ctx);
                }
            }
            Expr::SymPatternNamed { pattern, .. } => {
                Self::normalize_calls_in_expr(pattern, ctx);
            }
            Expr::SymRule { lhs, rhs, .. } => {
                Self::normalize_calls_in_expr(lhs, ctx);
                Self::normalize_calls_in_expr(rhs, ctx);
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                Self::normalize_calls_in_expr(expr, ctx);
                for r in rules.iter_mut() {
                    Self::normalize_calls_in_expr(r, ctx);
                }
            }
            // Atomic literals and a non-rewritten VarRef carry no
            // sub-expressions.
            Expr::IntLit { .. }
            | Expr::BoolLit { .. }
            | Expr::NilLit { .. }
            | Expr::SymLit { .. }
            | Expr::StrLit { .. }
            | Expr::FloatLit { .. }
            | Expr::VarRef { .. } => {}
        }
    }

    /// SIR22 helper for `normalize_calls_in_stmt`/`normalize_calls_in_expr`:
    /// recurse into an `IndexArg`'s embedded expression (if any).
    fn normalize_calls_in_index_arg(idx: &mut IndexArg, ctx: &BlockNormCtx) {
        match idx {
            IndexArg::Scalar(e) | IndexArg::Range(e) => Self::normalize_calls_in_expr(e, ctx),
            IndexArg::Whole => {}
        }
    }

    /// Phase Q10c — collect every name bound by a `let`/`let*`/`Assign`
    /// anywhere in a function body (descending into control-flow and
    /// nested blocks), so the call-site rewrite can tell a parenless
    /// method call from a reference to a same-named local.  Conservative:
    /// a name bound *anywhere* in the function shadows the method name for
    /// the whole function (we do not model block-scoped shadowing), which
    /// only ever *suppresses* a rewrite — never produces a wrong one.
    fn collect_bound_names_block(block: &Block, out: &mut HashSet<String>) {
        for s in &block.stmts {
            Self::collect_bound_names_stmt(s, out);
        }
        Self::collect_bound_names_expr(&block.value, out);
    }

    fn collect_bound_names_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
        for s in stmts {
            Self::collect_bound_names_stmt(s, out);
        }
    }

    fn collect_bound_names_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
        match stmt {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                out.insert(name.clone());
                Self::collect_bound_names_expr(value, out);
            }
            Stmt::Assign { name, value, .. } => {
                out.insert(name.clone());
                Self::collect_bound_names_expr(value, out);
            }
            Stmt::ExprStmt { expr, .. } => Self::collect_bound_names_expr(expr, out),
            Stmt::While { cond, body, .. } => {
                Self::collect_bound_names_expr(cond, out);
                Self::collect_bound_names_block(body, out);
            }
            Stmt::ForRange {
                var,
                start,
                stop,
                step,
                body,
                ..
            } => {
                out.insert(var.clone());
                Self::collect_bound_names_expr(start, out);
                Self::collect_bound_names_expr(stop, out);
                Self::collect_bound_names_expr(step, out);
                Self::collect_bound_names_block(body, out);
            }
            Stmt::ForEach {
                var, iter, body, ..
            } => {
                out.insert(var.clone());
                Self::collect_bound_names_expr(iter, out);
                Self::collect_bound_names_block(body, out);
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                Self::collect_bound_names_expr(seq, out);
                Self::collect_bound_names_expr(index, out);
                Self::collect_bound_names_expr(value, out);
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                Self::collect_bound_names_expr(map, out);
                Self::collect_bound_names_expr(key, out);
                Self::collect_bound_names_expr(value, out);
            }
            // SIR22 compile-compat stub (never emitted by this frontend):
            // an index-assignment binds no new name itself, but recurse
            // into every child `Expr` — including each `IndexArg`'s
            // embedded expression — matching `SeqSet`/`MapSet` above, in
            // case a nested closure or let-binding is ever produced there.
            Stmt::IndexSet {
                target,
                indices,
                value,
                ..
            } => {
                Self::collect_bound_names_expr(target, out);
                for idx in indices {
                    Self::collect_bound_names_expr_in_index_arg(idx, out);
                }
                Self::collect_bound_names_expr(value, out);
            }
            Stmt::ClassDef { body, .. }
            | Stmt::ModuleDef { body, .. }
            | Stmt::SingletonClassDef { body, .. } => Self::collect_bound_names_stmts(body, out),
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                Self::collect_bound_names_stmts(body, out);
                for r in rescues {
                    if let Some(b) = &r.binding {
                        out.insert(b.clone());
                    }
                    Self::collect_bound_names_stmts(&r.body, out);
                }
                if let Some(eb) = ensure_body {
                    Self::collect_bound_names_stmts(eb, out);
                }
            }
        }
    }

    fn collect_bound_names_expr(expr: &Expr, out: &mut HashSet<String>) {
        match expr {
            // SIR26 conversion (not currently emitted here) — recurse.
            Expr::Convert { value, .. } => Self::collect_bound_names_expr(value, out),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::collect_bound_names_expr(cond, out);
                Self::collect_bound_names_block(then_branch, out);
                Self::collect_bound_names_block(else_branch, out);
            }
            Expr::Block(b) => Self::collect_bound_names_block(b, out),
            Expr::DirectCall { args, .. }
            | Expr::BuiltinCall { args, .. }
            | Expr::Intrinsic { args, .. } => {
                for a in args {
                    Self::collect_bound_names_expr(a, out);
                }
            }
            Expr::IndirectCall { target, args, .. } => {
                Self::collect_bound_names_expr(target, out);
                for a in args {
                    Self::collect_bound_names_expr(a, out);
                }
            }
            Expr::MakeClosure { captures, .. } => {
                for c in captures {
                    Self::collect_bound_names_expr(&c.value, out);
                }
            }
            Expr::SeqLit { items, .. } => {
                for i in items {
                    Self::collect_bound_names_expr(i, out);
                }
            }
            Expr::SeqIndex { seq, index, .. } => {
                Self::collect_bound_names_expr(seq, out);
                Self::collect_bound_names_expr(index, out);
            }
            Expr::SeqLen { seq, .. } => Self::collect_bound_names_expr(seq, out),
            Expr::MapLit { entries, .. } => {
                for e in entries {
                    Self::collect_bound_names_expr(&e.key, out);
                    Self::collect_bound_names_expr(&e.value, out);
                }
            }
            Expr::MapGet { map, key, .. } => {
                Self::collect_bound_names_expr(map, out);
                Self::collect_bound_names_expr(key, out);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                Self::collect_bound_names_expr(lhs, out);
                Self::collect_bound_names_expr(rhs, out);
            }
            Expr::StrConcat { parts, .. } => {
                for p in parts {
                    Self::collect_bound_names_expr(p, out);
                }
            }
            // KW1 compile-compat stub: recurse into the `KeywordArg`'s inner
            // `value` so a name bound inside a keyword argument is still
            // collected.  Real support pending KW2–KW8.
            Expr::KeywordArg { value, .. } => Self::collect_bound_names_expr(value, out),
            // SIR22 compile-compat stubs (never emitted by this frontend):
            // recurse into every child `Expr`, matching the treatment of
            // `SeqLit`/`MapLit`/etc. above, so a name bound inside a nested
            // closure would still be collected.
            Expr::ArrayLit { rows, .. } => {
                for row in rows {
                    for e in row {
                        Self::collect_bound_names_expr(e, out);
                    }
                }
            }
            Expr::Range {
                start, step, stop, ..
            } => {
                Self::collect_bound_names_expr(start, out);
                if let Some(s) = step {
                    Self::collect_bound_names_expr(s, out);
                }
                Self::collect_bound_names_expr(stop, out);
            }
            Expr::MatMul { lhs, rhs, .. } => {
                Self::collect_bound_names_expr(lhs, out);
                Self::collect_bound_names_expr(rhs, out);
            }
            Expr::ElementwiseOp { lhs, rhs, .. } => {
                Self::collect_bound_names_expr(lhs, out);
                Self::collect_bound_names_expr(rhs, out);
            }
            Expr::Transpose { target, .. } => Self::collect_bound_names_expr(target, out),
            Expr::IndexGet {
                target, indices, ..
            } => {
                Self::collect_bound_names_expr(target, out);
                for idx in indices {
                    Self::collect_bound_names_expr_in_index_arg(idx, out);
                }
            }
            // SIR23 compile-compat stubs (never emitted by this frontend):
            // recurse into every child `Expr`, matching the SIR22 stubs
            // above, so a name bound inside a nested closure would still be
            // collected.
            Expr::SymSymbol { .. } | Expr::SymRational { .. } => {}
            Expr::SymApply { head, args, .. } => {
                Self::collect_bound_names_expr(head, out);
                for a in args {
                    Self::collect_bound_names_expr(a, out);
                }
            }
            Expr::SymPatternBlank { head, .. } => {
                if let Some(h) = head {
                    Self::collect_bound_names_expr(h, out);
                }
            }
            Expr::SymPatternNamed { pattern, .. } => {
                Self::collect_bound_names_expr(pattern, out);
            }
            Expr::SymRule { lhs, rhs, .. } => {
                Self::collect_bound_names_expr(lhs, out);
                Self::collect_bound_names_expr(rhs, out);
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                Self::collect_bound_names_expr(expr, out);
                for r in rules {
                    Self::collect_bound_names_expr(r, out);
                }
            }
            Expr::IntLit { .. }
            | Expr::BoolLit { .. }
            | Expr::NilLit { .. }
            | Expr::SymLit { .. }
            | Expr::StrLit { .. }
            | Expr::FloatLit { .. }
            | Expr::VarRef { .. } => {}
        }
    }

    /// SIR22 helper for `collect_bound_names_stmt`/`collect_bound_names_expr`
    /// (Q10c section): recurse into an `IndexArg`'s embedded expression (if
    /// any).
    fn collect_bound_names_expr_in_index_arg(idx: &IndexArg, out: &mut HashSet<String>) {
        match idx {
            IndexArg::Scalar(e) | IndexArg::Range(e) => Self::collect_bound_names_expr(e, out),
            IndexArg::Whole => {}
        }
    }

    /// Phase 7c — lower an endless method definition `def foo = expr`
    /// (or `def foo(x, y) = expr`) into a top-level `Function`.
    ///
    /// Endless methods are Ruby 3.0's terser one-liner syntax for
    /// pure functions: the entire method body is a single expression
    /// after the `=`, with no `end` keyword.  The parse-tree shape is:
    ///
    /// ```text
    /// endless_def_statement
    ///   ├─ Keyword("def")
    ///   ├─ Name(method_name)
    ///   ├─ [params]   (optional)
    ///   ├─ Equals
    ///   └─ expression (node)
    /// ```
    ///
    /// We reuse the parameter-extraction logic from `lower_def_statement`
    /// (identical shape) and then collect the single trailing
    /// `expression` Node as the function's tail value.  No statements
    /// in the body — `Block.stmts` is empty, `Block.value` is the
    /// lowered expression.
    fn lower_endless_def_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Function, RubyLowerError> {
        // Method name — first Name-typed token (skips the `def` keyword).
        let name_token = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "endless_def_statement missing method-name token".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let name = name_token.value.clone();

        // Parameter list — same `params` rule shape as `def_statement`.
        // Reuse the same extraction (find each `param` subnode, detect the
        // `*`/`**` splat prefix → ParamKind, take the identifier Name).  See
        // the matching code in `lower_def_statement` (M3).
        // Phase P7: extract params with `name = expr` defaults (see
        // `extract_params`).  Endless defs share the `params` rule shape.
        let params_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
            _ => None,
        });
        let params: Vec<Param> = self.extract_params(params_node)?;

        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // Set up a fresh locals + params scope for the body expression
        // — identical to `lower_def_statement` so any VarRef to a
        // parameter inside the expression resolves as `Scope::Param`.
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        // Phase RB2 — mark that we are inside a method body so a block
        // literal that `yield`s captures *this* method's block.
        let saved_in_def = self.in_def_body;
        let saved_block_cap = self.block_captures_enclosing;
        self.in_def_body = true;
        self.block_captures_enclosing = false;
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // The body is the single `expression` Node child.  PEG
        // guarantees exactly one such child (the grammar rule has it
        // as a non-repeated, non-optional element after the `EQUALS`).
        let expr_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "endless_def_statement missing body expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let value = self.lower_expression(expr_node)?;

        // Restore the outer scope.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;

        // Phase Q9e — an endless def may also `yield` (`def t = yield`);
        // thread the explicit block param if so.
        let func = Function {
            name,
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: Vec::new(),
                value,
                span: self.span_of(node),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(node),
        };
        let threaded = self.thread_block_param(func);
        // Phase RB2 — restore the enclosing method's block-context flags.
        self.in_def_body = saved_in_def;
        self.block_captures_enclosing = saved_block_cap;
        Ok(threaded)
    }

    fn lower_def_statement(&mut self, node: &GrammarASTNode) -> Result<Function, RubyLowerError> {
        // Shape:
        //   KEYWORD("def") NAME [ LPAREN [ params ] RPAREN ]
        //                  { !"end" statement } KEYWORD("end")
        // The first child token is the `def` keyword itself; the
        // method name is the *Name* token that follows.  We can't
        // use `expect_first_name_token` because it accepts both
        // Name and Keyword — it would return "def".
        let name_token = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let name_token = name_token.ok_or_else(|| RubyLowerError {
            message: "def_statement missing method-name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let name = name_token.value.clone();

        // Collect parameters.  The optional `params` rule node holds
        // a sequence of `param` subnodes (Phase 6s — each param is
        // wrapped in its own rule so the optional `*` / `**` splat
        // prefix can sit inside the param slot).  We walk each `param`,
        // detect the splat prefix from its leading Token (`*` or `**`,
        // both with `value` set), and extract the parameter Name.
        //
        // M3: the splat-ness of a param is now preserved on the SIR
        // `Param.kind`.  A `*rest` param lowers to `ParamKind::Rest`,
        // `**opts` to `ParamKind::KwRest`, everything else
        // `ParamKind::Required` — so the backends can emit faithful
        // `*args`/`**kwargs` (Python) / `...rest` (TypeScript).  See
        // `code/specs/sir-variadic-params.md`.
        // Phase P7: extract params, lowering `name = expr` defaults into
        // `Param.default` (param-scoped, so later defaults may reference
        // earlier params).  See `extract_params`.
        let params_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
            _ => None,
        });
        let params: Vec<Param> = self.extract_params(params_node)?;

        // Phase 6b: any non-empty parameter list means we'll emit
        // untyped Params (sir_type=None), which the SIR validator
        // requires `dynamic-typing` to be declared for.
        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // Lower the body using a fresh locals + params scope so the
        // outer program's bindings don't leak into the method.
        // Parameters are pre-declared as "locals" so a re-assignment
        // to a param routes through `Stmt::Assign` (SIR-correct),
        // *and* are tracked in `current_params` so any `VarRef` to
        // them inside the body gets `Scope::Param` (validator-correct).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        // Phase RB2 — mark that we are inside a method body so a block
        // literal that `yield`s captures *this* method's block.
        let saved_in_def = self.in_def_body;
        let saved_block_cap = self.block_captures_enclosing;
        self.in_def_body = true;
        self.block_captures_enclosing = false;
        // O2 — record the method name so a `super` lowered inside this body
        // knows which method to re-dispatch on the parent.  Saved/restored so
        // a nested `def`/block does not leak this method's name outward.
        let saved_method = self.current_method.take();
        self.current_method = Some(name.clone());
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // The body is every `statement` child of the def_statement
        // that *isn't* the method's own def_statement (we already
        // matched that), in source order.
        let body_stmts: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();

        // Phase 16e (FC) — method-level rescue/ensure.  If the def body
        // carries trailing `rescue`/`ensure` clauses (no explicit
        // `begin`), the WHOLE method body is the protected region: wrap
        // the body statements and the clauses in a single
        // `Stmt::TryCatch` (the method's value is then nil).
        let has_exception_clauses = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Node(n)
                if n.rule_name == "rescue_clause" || n.rule_name == "ensure_clause")
        });

        let (mut stmts_out, value): (Vec<Stmt>, Expr) = if has_exception_clauses {
            self.features_used.insert(Feature::Exceptions);
            let try_body = self.lower_flat_statements(node)?;
            let (rescues, ensure_body) = self.lower_rescue_ensure_clauses(node)?;
            (
                vec![Stmt::TryCatch {
                    body: try_body,
                    rescues,
                    ensure_body,
                    span: self.span_of(node),
                }],
                Expr::NilLit {
                    span: self.span_of(node),
                },
            )
        } else {
            let mut stmts_out: Vec<Stmt> = Vec::new();
            let mut value: Option<Expr> = None;
            if body_stmts.is_empty() {
                value = Some(Expr::NilLit {
                    span: self.span_of(node),
                });
            } else {
                let last_idx = body_stmts.len() - 1;
                for (i, s) in body_stmts.iter().enumerate() {
                    let inner = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                        message: "statement node had no child rule".to_string(),
                        line: s.start_line.unwrap_or(0),
                        column: s.start_column.unwrap_or(0),
                    })?;
                    let is_tail = i == last_idx;
                    // Phase FC — Ruby methods have no explicit `return`: the
                    // body's value is its last evaluated expression.  Promote
                    // that tail into the `Block`'s `value` slot so the backends
                    // emit it as the implicit return.  A trailing `if`/`unless`
                    // now promotes too (via [`Self::lower_tail_value`]), so a
                    // method whose body ends in a conditional returns the
                    // branch value instead of `nil`.
                    if is_tail {
                        if let Some(v) = self.lower_tail_value(inner)? {
                            value = Some(v);
                            continue;
                        }
                    }
                    // Phase 6r — multi-stmt fan-out for `multi_assignment`.
                    stmts_out.extend(self.lower_statement_inner_multi(inner)?);
                }
            }
            (
                stmts_out,
                value.unwrap_or(Expr::NilLit {
                    span: self.span_of(node),
                }),
            )
        };

        // Restore the outer scope's locals + params so the rest of
        // the program lowers correctly.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;
        // O2 — restore the enclosing method name (usually `None`).
        self.current_method = saved_method;

        // Sequential-assignment fix-up for the method body (see
        // `sequentialize_let_bindings`) — `def f; a = 1; x = a; end` must
        // resolve the read of `a`.
        sequentialize_let_bindings(&mut stmts_out);

        // Phase Q9e — if the method body `yield`s, thread the explicit
        // trailing block parameter and rewrite each `yield` into an
        // `IndirectCall` through it.  Non-yielding methods are returned
        // unchanged.
        let func = Function {
            name,
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: stmts_out,
                value,
                span: self.span_of(node),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(node),
        };
        let threaded = self.thread_block_param(func);
        // Phase RB2 — restore the enclosing method's block-context flags.
        self.in_def_body = saved_in_def;
        self.block_captures_enclosing = saved_block_cap;
        Ok(threaded)
    }

    // -------------------------------------------------------------------
    // assignment → LetBinding (first) or Assign (subsequent)
    // -------------------------------------------------------------------

    fn lower_assignment(&mut self, node: &GrammarASTNode) -> Result<Stmt, RubyLowerError> {
        // Shape (post-6p): NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "||=" | "&&=" ) expression
        let (name, name_span) = self.expect_first_name_token(node)?;
        let expr_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: "assignment missing RHS expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let rhs = self.lower_expression(expr_node)?;

        // Phase 6p — detect compound-assign operator.  The lexer
        // pre-fuses `+=`, `-=`, `*=`, `/=`, `||=`, `&&=` into single
        // Name-typed tokens; here we read the operator token (skipping
        // the leading NAME) to dispatch.
        //
        // Phase 8a (FC) extension — the lexer now also fuses the
        // additional compound forms `%=`, `**=`, `<<=`, `&=`, `|=`,
        // `^=` (Ruby's full arithmetic/bitwise/shift compound family).
        // `>>=` is deliberately NOT in this list because the 1.8-era
        // lexer state machine splits `>>` into two `>` tokens; folding
        // that requires a dedicated `>>` pre-fusion pass and is
        // tracked as a follow-up chunk.
        let op_token = node.children.iter().skip(1).find_map(|c| match c {
            ASTNodeOrToken::Token(t) => {
                let v = t.value.as_str();
                if matches!(
                    v,
                    "+=" | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "**="
                        | "<<="
                        | ">>="
                        | "&="
                        | "|="
                        | "^="
                        | "||="
                        | "&&="
                ) {
                    Some(v.to_string())
                } else {
                    None
                }
            }
            _ => None,
        });

        let span = self.span_of(node);

        // ── Phase 8b — short-circuit op-assign (`||=`, `&&=`) ───────────
        //
        // Ruby's short-circuit compound assignments do NOT eagerly
        // evaluate the RHS when the LHS already short-circuits the
        // expression.  Specifically:
        //
        //   `x ||= y`  ≡  `x || (x = y)`
        //                — if `x` is truthy, `y` is NOT evaluated and
        //                  `x` is NOT re-assigned (side effects on `y`
        //                  must not fire).
        //   `x &&= y`  ≡  `x && (x = y)`
        //                — if `x` is falsy, `y` is NOT evaluated and
        //                  `x` is NOT re-assigned.
        //
        // Phase 6p lowered both forms eagerly to
        // `Assign(x, BuiltinCall("or"/"and", [VarRef(x), y]))` — that
        // ALWAYS evaluates `y` and ALWAYS re-binds `x`, which silently
        // breaks side-effect ordering when `y` has them.  Phase 8b
        // replaces that with a gated `Expr::If` so the assignment is
        // skipped entirely when the short-circuit condition fires.
        //
        // SIR shape for `x ||= y` (and analogously for `&&=`):
        //
        //   ExprStmt(If(
        //     cond:        VarRef(x),
        //     then_branch: Block { stmts: [],            value: VarRef(x) },  // truthy → keep x
        //     else_branch: Block { stmts: [Assign(x,y)], value: VarRef(x) },  // falsy → assign
        //   ))
        //
        // For `&&=`, the two branches swap (truthy → assign, falsy → keep).
        //
        // The compound-assign path below stays in charge of all the
        // arithmetic/bitwise/shift forms (`+=`, `-=`, `*=`, …, `>>=`) —
        // those have no short-circuit semantics, so the existing
        // BuiltinCall lowering is correct for them.
        if let Some(op) = op_token.as_deref() {
            if op == "||=" || op == "&&=" {
                // Mark mutation + record local so subsequent `x = …`
                // statements don't re-binding-error.  Matches the
                // bookkeeping the generic compound path does below.
                self.features_used.insert(Feature::MutableBindings);
                self.declared_locals.insert(name.clone());

                let cond_ref = Expr::VarRef {
                    name: name.clone(),
                    scope: Scope::Local,
                    span: span.clone(),
                };
                let result_ref = Expr::VarRef {
                    name: name.clone(),
                    scope: Scope::Local,
                    span: span.clone(),
                };
                let assign_stmt = Stmt::Assign {
                    name: name.clone(),
                    scope: Scope::Local,
                    value: rhs,
                    span: span.clone(),
                };
                let empty_block = Block {
                    stmts: vec![],
                    value: result_ref.clone(),
                    span: span.clone(),
                };
                let assign_block = Block {
                    stmts: vec![assign_stmt],
                    value: result_ref,
                    span: span.clone(),
                };
                // `||=`: truthy → keep (empty); falsy → assign
                // `&&=`: truthy → assign;        falsy → keep (empty)
                let (then_branch, else_branch) = if op == "||=" {
                    (empty_block, assign_block)
                } else {
                    (assign_block, empty_block)
                };
                return Ok(Stmt::ExprStmt {
                    expr: Expr::If {
                        cond: Box::new(cond_ref),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                        span: span.clone(),
                    },
                    span,
                });
            }
        }

        // Build the effective RHS.  For plain `=`, it's just `rhs`.
        // For compound forms, wrap it as `BuiltinCall(op, [VarRef(x), rhs])`
        // where `op` is the underlying binary operator (`+` for `+=`,
        // `or` for `||=`, etc.).  Lowering identically to
        // `x = x op rhs` keeps downstream emitters simple — no new
        // compound-assign-aware code paths required.
        //
        // Builtin-name table for Phase 8a additions:
        //   `%=`  → `%`   (modulo)
        //   `**=` → `**`  (power)
        //   `<<=` → `<<`  (left shift / append)
        //   `&=`  → `&`   (bitwise and)
        //   `|=`  → `|`   (bitwise or)
        //   `^=`  → `^`   (bitwise xor)
        //
        // These BuiltinCall names match the surface operator literally,
        // following the same convention as `+`/`-`/`*`/`/` already in
        // use; downstream emitters that target Ruby (or any language
        // with the same operator spellings) can pass the name through
        // unchanged.
        let value = if let Some(op) = op_token.as_deref() {
            let (builtin_name, effects) = match op {
                "+=" => ("+", EffectSet::PURE),
                "-=" => ("-", EffectSet::PURE),
                "*=" => ("*", EffectSet::PURE),
                "/=" => ("/", EffectSet::PURE),
                "%=" => ("%", EffectSet::PURE),
                "**=" => ("**", EffectSet::PURE),
                "<<=" => ("<<", EffectSet::PURE),
                ">>=" => (">>", EffectSet::PURE),
                "&=" => ("&", EffectSet::PURE),
                "|=" => ("|", EffectSet::PURE),
                "^=" => ("^", EffectSet::PURE),
                // `||=` and `&&=` are handled by the short-circuit
                // branch above (Phase 8b) and never reach this match.
                _ => unreachable!("op_token matched only the eager compound forms above"),
            };
            // Phase 15a/15b/15c — a compound assign to a sigil var or a
            // constant (`@x += 1`, `@@x += 1`, `FOO += 1`) reads it back
            // with the matching scope.
            let lhs_scope = if is_class_var_name(&name) {
                Scope::ClassVar
            } else if is_instance_var_name(&name) {
                Scope::Instance
            } else if is_constant_name(&name) {
                Scope::Const
            } else {
                Scope::Local
            };
            let lhs_ref = Expr::VarRef {
                name: name.clone(),
                scope: lhs_scope,
                span: span.clone(),
            };
            Expr::BuiltinCall {
                name: builtin_name.to_string(),
                args: vec![lhs_ref, rhs],
                effects,
                span: span.clone(),
            }
        } else {
            rhs
        };

        // Phase 15a/15b (FC) — instance- / class-variable assignment
        // (`@x = …`, `@@x = …`, and their compound forms) lowers to
        // `Stmt::Assign { scope: Instance | ClassVar }` regardless of
        // prior sightings: a sigil var needs no `let` declaration and is
        // never a local.  The store is a `Stmt::Assign`, whose validator
        // arm observes `MutableBindings`, so we declare both features.
        // We do NOT touch `declared_locals`.  Class var is checked first
        // (`@@x` also starts with `@`).
        if is_class_var_name(&name) {
            self.features_used.insert(Feature::ClassVars);
            self.features_used.insert(Feature::MutableBindings);
            return Ok(Stmt::Assign {
                name,
                scope: Scope::ClassVar,
                value,
                span,
            });
        }
        if is_instance_var_name(&name) {
            self.features_used.insert(Feature::InstanceVars);
            self.features_used.insert(Feature::MutableBindings);
            return Ok(Stmt::Assign {
                name,
                scope: Scope::Instance,
                value,
                span,
            });
        }

        // Phase 15c (FC) — a constant assignment (`FOO = …`) lowers to
        // `Stmt::Assign { scope: Const }` rather than a `LetBinding`: a
        // constant is resolved against the constant scope, not the
        // local env, so it is never registered in `declared_locals`.
        // The store is a `Stmt::Assign` whose validator arm observes
        // `MutableBindings`, so we declare both features.  (Re-assigning
        // a constant is a warning in real Ruby, not an error, so we do
        // not try to enforce single-assignment here.)
        if is_constant_name(&name) {
            self.features_used.insert(Feature::Constants);
            self.features_used.insert(Feature::MutableBindings);
            return Ok(Stmt::Assign {
                name,
                scope: Scope::Const,
                value,
                span,
            });
        }

        // A compound assignment ALWAYS reads then re-binds, so it
        // must emit `Stmt::Assign` (never `LetBinding`).  Plain `=`
        // keeps the original "first sighting → LetBinding, subsequent
        // → Assign" behaviour.
        let is_compound = op_token.is_some();
        if is_compound || self.declared_locals.contains(&name) {
            // Re-bind path: mutable-bindings feature required.
            self.features_used.insert(Feature::MutableBindings);
            // Compound `x ||= 1` without a prior `x = …` is still
            // valid Ruby (treats `x` as nil), but we record it as a
            // local so any subsequent `x = 1` doesn't re-binding-error.
            self.declared_locals.insert(name.clone());
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declared_locals.insert(name.clone());
            Ok(Stmt::LetBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
        // `name_span` is intentionally unused for now — the SIR Stmt
        // span covers the whole statement.  Keeping the binding so
        // the lookup helper stays useful for callers that need it
        // (e.g. error messages).
        .inspect(|s| {
            let _ = name_span;
        })
    }

    // -------------------------------------------------------------------
    // Phase 7e — Ruby 3.0 rightward assignment `expr => var`
    // -------------------------------------------------------------------

    /// Lower a `rightward_assignment` node into the same `Stmt` shape
    /// as a regular `assignment`.  Ruby 3.0's rightward form is purely
    /// syntactic — `expr => var` and `var = expr` produce identical
    /// runtime bindings — so we lower identically to the `=`-form's
    /// LetBinding-on-first-sighting / Assign-on-rebind dispatch.
    ///
    /// Grammar shape:
    ///
    /// ```text
    /// rightward_assignment = expression "=>" NAME ;
    /// ```
    ///
    /// AST children layout: `[ expression_node, "=>" token, name_token ]`.
    ///
    /// Lowering table:
    ///
    /// | Source              | SIR shape                                          |
    /// |---------------------|----------------------------------------------------|
    /// | `1 + 2 => x`        | `LetBinding(x, BuiltinCall("+", [IntLit 1, IntLit 2]))` |
    /// | `[1,2] => arr`      | `LetBinding(arr, SeqLit([IntLit 1, IntLit 2]))`    |
    /// | (re-bind) `5 => x`  | `Assign(x, IntLit 5)` + Feature::MutableBindings   |
    fn lower_rightward_assignment(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, RubyLowerError> {
        // Step 1: the LHS-as-source (Ruby left side) is the value
        // expression — the `expression` Node child.
        let expr_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: "rightward_assignment missing LHS expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let value = self.lower_expression(expr_node)?;

        // Step 2: the binding name is the trailing `NAME` token.  Walk
        // children in reverse to find the *last* Name-typed token (the
        // expression itself may have contained Name tokens, but those
        // are inside the expression Node — direct children of the
        // rightward_assignment node are the expression Node, the `=>`
        // Op token, and the final Name token).
        let name_token = node
            .children
            .iter()
            .rev()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "rightward_assignment missing binding name".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let name = name_token.value.clone();
        let span = self.span_of(node);

        // Step 3: dispatch on whether the local is already declared —
        // same first-sighting / re-bind split as `lower_assignment`.
        if self.declared_locals.contains(&name) {
            self.features_used.insert(Feature::MutableBindings);
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declared_locals.insert(name.clone());
            Ok(Stmt::LetBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
    }

    // -------------------------------------------------------------------
    // Phase 6r — multi-assignment
    // -------------------------------------------------------------------

    /// Lower a `multi_assignment` node (`a, b = 1, 2`) into one SIR
    /// statement per (LHS, RHS) pair.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// multi_assignment = mlhs_target COMMA mlhs_target { COMMA mlhs_target }
    ///                    EQUALS
    ///                    expression { COMMA expression } ;
    /// mlhs_target      = [ "*" ] NAME ;
    /// ```
    ///
    /// AST layout: each `mlhs_target` sub-node holds an optional `"*"`
    /// token followed by a `NAME` token.  After the EQUALS token the
    /// RHS `expression` nodes follow.  We walk the parent's children
    /// linearly: `mlhs_target` sub-nodes encountered *before* EQUALS
    /// form the LHS list (recording `(name, is_splat)` per target);
    /// `expression` nodes encountered *after* EQUALS form the RHS list.
    ///
    /// Lowering rules:
    ///
    /// - **Non-splat target** at position `i` binds the `i`-th RHS
    ///   value (from the start, or from the end if the splat sits to
    ///   its left), exactly like Phase 6r's pair-wise lowering:
    ///   `Stmt::LetBinding` on first sighting, `Stmt::Assign` on
    ///   re-bind.
    /// - **Splat target** (Phase 9b — at most one per LHS) binds an
    ///   `Expr::SeqLit` of the "middle" RHS values — those that aren't
    ///   claimed by the fixed-position non-splat targets to its left
    ///   or right.  Always requests `Feature::Sequences`.
    ///
    /// **Arity check** (Phase 9b refinement):
    ///
    /// - No splat present → LHS count must equal RHS count (Phase 6r
    ///   semantics).
    /// - Splat present → RHS count must be ≥ `non_splat_count`.  The
    ///   splat absorbs the difference (possibly zero) into a Sequence.
    ///
    /// **Still deferred to later phases**:
    ///
    /// - Single-RHS auto-unpack `a, b = arr` (Phase 9c).
    /// - Multi-assignment LHS inside a `modifier_statement`.
    fn lower_multi_assignment(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Walk children, partitioning at the EQUALS token.
        // Each LHS sub-node is an `mlhs_target` — `[ "*" ] NAME`.
        let mut saw_equals = false;
        let mut lhs_targets: Vec<(String, bool, Span)> = Vec::new();
        let mut rhs_exprs: Vec<&GrammarASTNode> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) => {
                    if t.type_ == TokenType::Equals {
                        saw_equals = true;
                    }
                    // COMMAs and any stray tokens are ignored at this
                    // level — actual LHS NAMEs now live inside
                    // `mlhs_target` sub-nodes.
                }
                ASTNodeOrToken::Node(n) => {
                    if !saw_equals && n.rule_name == "mlhs_target" {
                        // Pick out optional "*" + NAME token from this
                        // sub-node.  The lexer emits "*" as a
                        // `TokenType::Star` token (not a Name-typed
                        // token), so the splat detection is
                        // type-based; the target's Name lands on a
                        // `TokenType::Name`.  Defensive value-filter on
                        // Name covers the edge case where a future
                        // lexer change re-routes `*` through Name.
                        let mut is_splat = false;
                        let mut name_and_span: Option<(String, Span)> = None;
                        for sub in &n.children {
                            if let ASTNodeOrToken::Token(t) = sub {
                                if t.type_ == TokenType::Star
                                    || (t.type_ == TokenType::Name && t.value == "*")
                                {
                                    is_splat = true;
                                } else if t.type_ == TokenType::Name {
                                    name_and_span = Some((t.value.clone(), self.span_of_token(t)));
                                }
                            }
                        }
                        if let Some((nm, sp)) = name_and_span {
                            lhs_targets.push((nm, is_splat, sp));
                        }
                    } else if saw_equals && n.rule_name == "expression" {
                        rhs_exprs.push(n);
                    }
                }
            }
        }

        // Sanity: the grammar guarantees at least two LHS targets and
        // at least one RHS, plus an EQUALS token between them.  Defend
        // against pathological inputs anyway.
        if lhs_targets.len() < 2 {
            return Err(RubyLowerError {
                message: format!(
                    "multi_assignment expected ≥2 LHS targets, got {}",
                    lhs_targets.len()
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        if !saw_equals {
            return Err(RubyLowerError {
                message: "multi_assignment missing EQUALS token".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }

        // Locate splat position(s).  At most one splat is allowed per
        // LHS — Ruby's grammar enforces this and so does the lowerer.
        let splat_positions: Vec<usize> = lhs_targets
            .iter()
            .enumerate()
            .filter_map(|(i, (_, is_splat, _))| if *is_splat { Some(i) } else { None })
            .collect();
        if splat_positions.len() > 1 {
            return Err(RubyLowerError {
                message: format!(
                    "multi_assignment allows at most one splat (*) LHS, got {}",
                    splat_positions.len()
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        let splat_idx: Option<usize> = splat_positions.first().copied();
        let non_splat_count = lhs_targets.len() - if splat_idx.is_some() { 1 } else { 0 };

        // Arity check — different rule depending on whether a splat is
        // present.
        //
        // Phase 9c (FC) — single-RHS auto-unpack ("tuple destructure")
        // adds one new acceptable shape for the no-splat path: exactly
        // one RHS and ≥2 LHS.  We lower it by binding the single RHS to
        // a temp and reading `temp[0]`, `temp[1]`, … into each LHS via
        // `Expr::SeqIndex`.  The check below permits that shape; the
        // dispatch a few lines down routes it to the dedicated helper.
        if splat_idx.is_none() {
            let is_single_rhs_unpack = rhs_exprs.len() == 1 && lhs_targets.len() > 1;
            if lhs_targets.len() != rhs_exprs.len() && !is_single_rhs_unpack {
                return Err(RubyLowerError {
                    message: format!(
                        "multi_assignment requires LHS count == RHS count \
                         OR exactly 1 RHS (tuple destructure); \
                         got {} LHS, {} RHS",
                        lhs_targets.len(),
                        rhs_exprs.len(),
                    ),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                });
            }
        } else if rhs_exprs.len() < non_splat_count {
            return Err(RubyLowerError {
                message: format!(
                    "multi_assignment with splat needs RHS count ≥ \
                     non-splat LHS count (got {} RHS, {} non-splat LHS)",
                    rhs_exprs.len(),
                    non_splat_count,
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }

        // Lower each RHS first — this matches Ruby's evaluation order
        // (RHS is fully evaluated, *then* the LHS bindings happen).
        let lowered_rhs: Vec<Expr> = rhs_exprs
            .iter()
            .map(|e| self.lower_expression(e))
            .collect::<Result<_, _>>()?;

        // ── Phase 9b dispatch ───────────────────────────────────────
        //
        // When a splat is present, we route to a dedicated lowering
        // path so the Phase 6r/9a non-splat code below stays simple.
        // The splat path always uses the swap-safe temp pass (Phase 9a
        // pattern) because routing one LHS to a SeqLit makes the
        // bookkeeping easier when every RHS value sits in a named temp.
        if let Some(splat_idx_real) = splat_idx {
            return self.lower_multi_assignment_with_splat(
                node,
                &lhs_targets,
                splat_idx_real,
                lowered_rhs,
            );
        }

        // ── Phase 9c dispatch ───────────────────────────────────────
        //
        // Single-RHS tuple destructure (`a, b = arr` and friends).
        // The arity check above already guaranteed that this shape is
        // 1 RHS + ≥2 LHS and no splat.  Route to a dedicated helper
        // that binds the RHS to a temp once and reads `temp[i]` for
        // each LHS via `Expr::SeqIndex`.
        if lowered_rhs.len() == 1 && lhs_targets.len() > 1 {
            // Move the lone RHS out of the Vec.  This `unwrap` is
            // unreachable because `len() == 1` was just checked.
            let rhs_value = lowered_rhs.into_iter().next().expect(
                "lower_multi_assignment: single-RHS dispatch invariant \
                 violated — lowered_rhs.len() == 1 was just checked",
            );
            return self.lower_multi_assignment_single_rhs_destructure(
                node,
                lhs_targets,
                rhs_value,
            );
        }
        // For the (also re-mapped) compatibility with the Phase 9a
        // code, project to the old `lhs_names: Vec<(String, Span)>`
        // shape now that we know there are no splats.
        let lhs_names: Vec<(String, Span)> = lhs_targets
            .iter()
            .map(|(n, _, s)| (n.clone(), s.clone()))
            .collect();

        // ── Phase 9a (FC) — swap-safe parallel binding ───────────────
        //
        // Ruby's parallel assignment collects ALL RHS values *before*
        // writing ANY LHS.  Phase 6r's lowering emitted statements
        // sequentially: `Stmt(a := rhs0); Stmt(b := rhs1)`.  That's
        // observably correct only when no LHS name appears in any RHS
        // expression — the simple `a, b = 1, 2` case.  For the swap
        // (`a, b = b, a`), the second statement reads the *new* `a`
        // instead of the original, producing the wrong result.
        //
        // Phase 9a closes the gap with a "needs-temps" heuristic:
        //
        //   1. Build the set of LHS names.
        //   2. Scan each lowered RHS expression for any `VarRef` whose
        //      name is in that set.
        //   3. If found → emit two passes:
        //        a. `LetStarBinding(__multi_assign_t<N>_<i>, rhs[i])`
        //           for each i — captures the ORIGINAL RHS value before
        //           any LHS is rebound.  Uses `LetStarBinding` so each
        //           temp's name is visible to the LHS-binding pass that
        //           follows (LetBinding's parallel-let validator group
        //           rule would hide them).
        //        b. `Stmt::LetBinding`/`Assign` for each LHS, reading
        //           from the corresponding temp via `VarRef`.
        //   4. Otherwise → emit the sequential shape Phase 6r used
        //      (one stmt per pair, no temps) — cheaper SIR and matches
        //      the simple-case test expectations.
        //
        // The temp names use a monotonic counter (`multi_assign_counter`)
        // so nested or repeated multi-assignments don't collide.  Names
        // are double-underscore-prefixed so they can't collide with
        // user-typeable Ruby locals.
        let lhs_name_set: HashSet<String> = lhs_names.iter().map(|(n, _)| n.clone()).collect();
        let needs_temps = lowered_rhs
            .iter()
            .any(|e| expr_references_any_name(e, &lhs_name_set));

        let mut out: Vec<Stmt> = Vec::new();

        if needs_temps {
            let counter = self.multi_assign_counter;
            self.multi_assign_counter += 1;

            // Pass 1: bind each RHS to a fresh temp via LetStarBinding
            // so the LHS-binding pass below can read them by VarRef
            // without hitting the LetBinding parallel-let validator
            // visibility rule.
            let mut temp_refs: Vec<Expr> = Vec::with_capacity(lowered_rhs.len());
            for (i, (rhs_value, (_, name_span))) in
                lowered_rhs.into_iter().zip(lhs_names.iter()).enumerate()
            {
                let tmp_name = format!("__multi_assign_t{}_{}", counter, i);
                let tmp_span = name_span.clone();
                // Record the temp so later VarRef lookups treat it as
                // a local.  (It IS declared via LetStarBinding.)
                self.declared_locals.insert(tmp_name.clone());
                out.push(Stmt::LetStarBinding {
                    name: tmp_name.clone(),
                    sir_type: None,
                    value: rhs_value,
                    span: tmp_span.clone(),
                });
                temp_refs.push(Expr::VarRef {
                    name: tmp_name,
                    scope: Scope::Local,
                    span: tmp_span,
                });
            }

            // Pass 2: assign each LHS from its temp.
            for ((name, name_span), tmp_ref) in lhs_names.into_iter().zip(temp_refs.into_iter()) {
                let span = name_span.clone();
                let stmt = if self.declared_locals.contains(&name) {
                    self.features_used.insert(Feature::MutableBindings);
                    Stmt::Assign {
                        name: name.clone(),
                        scope: Scope::Local,
                        value: tmp_ref,
                        span,
                    }
                } else {
                    self.declared_locals.insert(name.clone());
                    Stmt::LetBinding {
                        name: name.clone(),
                        sir_type: None,
                        value: tmp_ref,
                        span,
                    }
                };
                out.push(stmt);
            }
        } else {
            // Fast path: no LHS appears in any RHS, so the sequential
            // lowering Phase 6r used is observably equivalent to the
            // truly-parallel form.  Emit one Stmt per pair.
            for ((name, name_span), value) in lhs_names.into_iter().zip(lowered_rhs.into_iter()) {
                let span = name_span.clone();
                let stmt = if self.declared_locals.contains(&name) {
                    self.features_used.insert(Feature::MutableBindings);
                    Stmt::Assign {
                        name: name.clone(),
                        scope: Scope::Local,
                        value,
                        span,
                    }
                } else {
                    self.declared_locals.insert(name.clone());
                    Stmt::LetBinding {
                        name: name.clone(),
                        sir_type: None,
                        value,
                        span,
                    }
                };
                out.push(stmt);
            }
        }
        Ok(out)
    }

    // -------------------------------------------------------------------
    // Phase 9c (FC) — multi-assignment single-RHS tuple destructure
    // -------------------------------------------------------------------

    /// Lower the single-RHS form of `multi_assignment`:
    ///
    /// ```text
    /// a, b    = arr           → a == arr[0]; b == arr[1]
    /// a, b, c = arr           → a == arr[0]; b == arr[1]; c == arr[2]
    /// a, b    = make_pair()   → make_pair() evaluated once into a temp
    /// ```
    ///
    /// Strategy:
    ///
    /// 1. Bind the (already-lowered) RHS to a fresh
    ///    `LetStarBinding(__multi_assign_t<N>_seq, rhs)`.  We use
    ///    `LetStarBinding` so the temp's name is visible to the
    ///    subsequent LHS-binding pass (the parallel-let validator
    ///    would otherwise hide names declared in the same LetBinding
    ///    group).  The single-temp evaluation also guarantees side
    ///    effects in the RHS fire exactly once — important for things
    ///    like `a, b = next_pair()` where the call may have observable
    ///    side effects.
    ///
    /// 2. For each LHS at position `i`, emit
    ///    `Stmt::LetBinding`/`Stmt::Assign` reading from
    ///    `Expr::SeqIndex { seq: VarRef(temp), index: IntLit(i) }`.
    ///    The first-sighting vs. re-bind decision uses the same logic
    ///    as the rest of the lowerer (`declared_locals`).
    ///
    /// Always requests `Feature::Sequences` (for `SeqIndex`).  The
    /// re-bind path also requests `Feature::MutableBindings`.
    ///
    /// Out-of-bounds semantics: target-language-defined per
    /// `Expr::SeqIndex`'s documentation.  Ruby itself would fill
    /// missing positions with `nil`; matching that exactly is left to
    /// the consuming backend (or a later phase if we want to make
    /// missing-index→nil explicit in the SIR).
    fn lower_multi_assignment_single_rhs_destructure(
        &mut self,
        _node: &GrammarASTNode,
        lhs_targets: Vec<(String, bool, Span)>,
        rhs_value: Expr,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Mint a unique temp name.
        let counter = self.multi_assign_counter;
        self.multi_assign_counter += 1;
        let tmp_name = format!("__multi_assign_t{}_seq", counter);

        // Span for the temp is the first LHS's span — close enough
        // for diagnostics; the actual RHS expression already carries
        // its own span in its sub-trees.
        let tmp_span = lhs_targets
            .first()
            .map(|(_, _, s)| s.clone())
            .expect("single-RHS destructure invariant: lhs_targets non-empty");

        let mut out: Vec<Stmt> = Vec::with_capacity(lhs_targets.len() + 1);

        // Pass 1: bind the RHS to the temp.
        self.declared_locals.insert(tmp_name.clone());
        out.push(Stmt::LetStarBinding {
            name: tmp_name.clone(),
            sir_type: None,
            value: rhs_value,
            span: tmp_span.clone(),
        });

        // Both SeqIndex and the temp's array shape rely on the
        // Sequences feature flag.
        self.features_used.insert(Feature::Sequences);

        // Pass 2: bind each LHS from `temp[i]`.
        for (i, (name, _is_splat, name_span)) in lhs_targets.into_iter().enumerate() {
            let span = name_span.clone();
            let index_expr = Expr::SeqIndex {
                seq: Box::new(Expr::VarRef {
                    name: tmp_name.clone(),
                    scope: Scope::Local,
                    span: span.clone(),
                }),
                index: Box::new(Expr::IntLit {
                    value: i as i64,
                    span: span.clone(),
                }),
                span: span.clone(),
            };
            let stmt = if self.declared_locals.contains(&name) {
                self.features_used.insert(Feature::MutableBindings);
                Stmt::Assign {
                    name: name.clone(),
                    scope: Scope::Local,
                    value: index_expr,
                    span,
                }
            } else {
                self.declared_locals.insert(name.clone());
                Stmt::LetBinding {
                    name: name.clone(),
                    sir_type: None,
                    value: index_expr,
                    span,
                }
            };
            out.push(stmt);
        }

        Ok(out)
    }

    // -------------------------------------------------------------------
    // Phase 9b (FC) — multi-assignment WITH splat LHS
    // -------------------------------------------------------------------

    /// Lower the splat-LHS form of `multi_assignment`:
    ///
    /// ```text
    /// a, *b = 1, 2, 3        → a == 1; b == [2, 3]
    /// *a, b = 1, 2, 3        → a == [1, 2]; b == 3
    /// a, *b, c = 1, 2, 3, 4  → a == 1; b == [2, 3]; c == 4
    /// ```
    ///
    /// Strategy: always go through the temp pass (same shape as Phase
    /// 9a's swap-safe path) — every RHS value is bound to a fresh
    /// `LetStarBinding(__multi_assign_t<N>_<i>, rhs[i])`.  Then we walk
    /// the LHS list:
    ///
    /// - Non-splat target at position `i` (before splat) → reads
    ///   `__multi_assign_t<N>_<i>`.
    /// - Non-splat target at position `i` (after splat)  → reads
    ///   `__multi_assign_t<N>_<rhs_len - (lhs_len - 1 - i)>`.
    /// - Splat target → binds `Expr::SeqLit` of the "middle" temps
    ///   `__multi_assign_t<N>_<splat_idx>..__multi_assign_t<N>_<end>`
    ///   where `end = rhs_len - (lhs_len - 1 - splat_idx)`.  May be
    ///   empty (the splat gets an empty Sequence).  Always requests
    ///   `Feature::Sequences`.
    fn lower_multi_assignment_with_splat(
        &mut self,
        node: &GrammarASTNode,
        lhs_targets: &[(String, bool, Span)],
        splat_idx: usize,
        lowered_rhs: Vec<Expr>,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Step 1: emit temp bindings for every RHS.  Reuses the same
        // counter-based naming scheme as Phase 9a so temps from
        // adjacent multi-assignments don't collide.
        let counter = self.multi_assign_counter;
        self.multi_assign_counter += 1;
        let stmt_span = self.span_of(node);
        let rhs_len = lowered_rhs.len();

        let mut out: Vec<Stmt> = Vec::with_capacity(rhs_len + lhs_targets.len());
        let mut temp_names: Vec<String> = Vec::with_capacity(rhs_len);
        for (i, rhs_value) in lowered_rhs.into_iter().enumerate() {
            let tmp_name = format!("__multi_assign_t{}_{}", counter, i);
            // Record so subsequent VarRef lookups treat the temp as
            // a local.  (It IS declared via LetStarBinding here.)
            self.declared_locals.insert(tmp_name.clone());
            out.push(Stmt::LetStarBinding {
                name: tmp_name.clone(),
                sir_type: None,
                value: rhs_value,
                span: stmt_span.clone(),
            });
            temp_names.push(tmp_name);
        }

        // Step 2: emit the LHS bindings.  The splat absorbs the
        // middle of the temp list (possibly an empty middle).
        let lhs_len = lhs_targets.len();
        // Indices of the temps that go into the splat's SeqLit.
        let splat_temps_start = splat_idx;
        let splat_temps_end = rhs_len - (lhs_len - 1 - splat_idx);

        for (i, (name, is_splat, name_span)) in lhs_targets.iter().enumerate() {
            let span = name_span.clone();
            let value = if *is_splat {
                // Build a Sequence from the middle slice of temps.
                self.features_used.insert(Feature::Sequences);
                let items: Vec<Expr> = (splat_temps_start..splat_temps_end)
                    .map(|j| Expr::VarRef {
                        name: temp_names[j].clone(),
                        scope: Scope::Local,
                        span: span.clone(),
                    })
                    .collect();
                Expr::SeqLit {
                    items,
                    span: span.clone(),
                }
            } else {
                // Pick the temp at i-mapped position.  Before the
                // splat, positions match directly (0..splat_idx).
                // After the splat, count from the end:
                //   The post-splat tail has `lhs_len - 1 - splat_idx`
                //   non-splat targets; they map to the last
                //   `lhs_len - 1 - splat_idx` temps.  Target at LHS
                //   position `i` is at temp index `rhs_len - lhs_len + i`.
                let temp_index = if i < splat_idx {
                    i
                } else {
                    // i > splat_idx (since splat itself handled above)
                    rhs_len - lhs_len + i
                };
                Expr::VarRef {
                    name: temp_names[temp_index].clone(),
                    scope: Scope::Local,
                    span: span.clone(),
                }
            };

            let stmt = if self.declared_locals.contains(name) {
                self.features_used.insert(Feature::MutableBindings);
                Stmt::Assign {
                    name: name.clone(),
                    scope: Scope::Local,
                    value,
                    span,
                }
            } else {
                self.declared_locals.insert(name.clone());
                Stmt::LetBinding {
                    name: name.clone(),
                    sir_type: None,
                    value,
                    span,
                }
            };
            out.push(stmt);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------
    // Phase 6v / 16a — `begin … rescue … ensure … end`
    // -------------------------------------------------------------------

    /// Lower a `begin_statement` node to a single `Stmt::TryCatch`.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// begin_statement = "begin"
    ///                   { !"rescue" !"ensure" !"end" statement }
    ///                   { rescue_clause }
    ///                   [ ensure_clause ]
    ///                   "end" ;
    /// rescue_clause   = "rescue" [ exception_list ] [ "=>" NAME ]
    ///                        { !"rescue" !"ensure" !"end" statement } ;
    /// exception_list  = NAME { COMMA NAME } ;
    /// ensure_clause   = "ensure" { !"end" statement } ;
    /// ```
    ///
    /// **Phase 16a (FC)** — the body, each `rescue_clause`, and the
    /// optional `ensure_clause` lower into a first-class
    /// `Stmt::TryCatch { body, rescues, ensure_body }` (semantic-ir
    /// 0.9.0), replacing the Phase 6v inline `__rescue_marker__` /
    /// `__ensure_marker__` placeholder builtins:
    ///
    /// ```text
    /// begin
    ///   body_stmts
    /// rescue StandardError, IOError => e
    ///   rescue_stmts
    /// ensure
    ///   ensure_stmts
    /// end
    /// ```
    ///
    /// →
    ///
    /// ```text
    /// TryCatch {
    ///   body:    [body_stmts…],
    ///   rescues: [RescueClause {
    ///       exception_types: ["StandardError", "IOError"],
    ///       binding: Some("e"),
    ///       body: [rescue_stmts…],
    ///   }],
    ///   ensure_body: Some([ensure_stmts…]),
    /// }
    /// ```
    ///
    /// Emitting it requests `Feature::Exceptions`.  The function returns
    /// a one-element `Vec<Stmt>` (the `TryCatch`) so its call site — which
    /// flattens statement lists — stays uniform with the other
    /// multi-statement lowerings.
    fn lower_begin_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Phase 16a (FC): `begin/rescue/ensure/end` lowers to a
        // first-class `Stmt::TryCatch`.  Phase 16e factored the body /
        // rescue / ensure extraction into shared helpers so method-level
        // rescue (`def … rescue … end`, no explicit `begin`) can reuse
        // them.
        let outer_span = self.span_of(node);
        self.features_used.insert(Feature::Exceptions);
        let body = self.lower_flat_statements(node)?;
        let (rescues, ensure_body) = self.lower_rescue_ensure_clauses(node)?;
        Ok(vec![Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            span: outer_span,
        }])
    }

    /// Lower the direct `statement` children of `node` into a flat
    /// `Vec<Stmt>` — no trailing-value extraction.  Used for
    /// `Stmt::TryCatch` body / rescue / ensure blocks (Phase 16a/16e),
    /// where the negative-lookahead grammar repetition guarantees the
    /// direct `statement` children are exactly that block's statements.
    fn lower_flat_statements(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        let mut body = Vec::new();
        for cc in &node.children {
            if let ASTNodeOrToken::Node(nn) = cc {
                if nn.rule_name == "statement" {
                    if let Some(inner) = self.first_node_child(nn) {
                        body.extend(self.lower_statement_inner_multi(inner)?);
                    }
                }
            }
        }
        Ok(body)
    }

    /// Scan `node`'s direct children for `rescue_clause` / `ensure_clause`
    /// and build the `(rescues, ensure_body)` for a `Stmt::TryCatch`.
    /// Shared by `begin … end` (Phase 16a) and method-level rescue on a
    /// `def` body (Phase 16e).  Callers are responsible for requesting
    /// `Feature::Exceptions`.
    fn lower_rescue_ensure_clauses(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<(Vec<RescueClause>, Option<Vec<Stmt>>), RubyLowerError> {
        // Each rescue_clause, in source order.
        let mut rescues: Vec<RescueClause> = Vec::new();
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "rescue_clause" {
                    // Exception class names (`rescue Foo, Bar`) — the
                    // `exception_list` node holds them as Name tokens.
                    let exception_types: Vec<String> = n
                        .children
                        .iter()
                        .find_map(|c| match c {
                            ASTNodeOrToken::Node(en) if en.rule_name == "exception_list" => {
                                Some(en)
                            }
                            _ => None,
                        })
                        .map(|en| {
                            en.children
                                .iter()
                                .filter_map(|cc| match cc {
                                    ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => {
                                        Some(t.value.clone())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    // Optional binding (`=> e`): the Name token after the
                    // `=>` token among the clause's direct children.
                    let binding: Option<String> = {
                        let mut saw_arrow = false;
                        let mut found: Option<String> = None;
                        for cc in &n.children {
                            if let ASTNodeOrToken::Token(t) = cc {
                                if t.value == "=>" {
                                    saw_arrow = true;
                                } else if saw_arrow && t.type_ == TokenType::Name {
                                    found = Some(t.value.clone());
                                    break;
                                }
                            }
                        }
                        found
                    };
                    let rescue_body = self.lower_flat_statements(n)?;
                    rescues.push(RescueClause {
                        exception_types,
                        binding,
                        body: rescue_body,
                        span: self.span_of(n),
                    });
                }
            }
        }

        // Optional ensure_clause.
        let mut ensure_body: Option<Vec<Stmt>> = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "ensure_clause" {
                    ensure_body = Some(self.lower_flat_statements(n)?);
                }
            }
        }

        Ok((rescues, ensure_body))
    }

    // -------------------------------------------------------------------
    // method_call → BuiltinCall (recognised names) or DirectCall
    // -------------------------------------------------------------------

    fn lower_method_call(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Shapes (Phase 6s-aware):
        //   method_call          = (NAME|KEYWORD) LPAREN [ call_arg
        //                          (COMMA call_arg)* ] RPAREN { dot_call }
        //   method_call_no_paren = (NAME|KEYWORD) expression
        //                          (COMMA expression)*
        //
        // The two shapes use *different* arg encodings: parenned calls
        // wrap each arg in a `call_arg` rule (which admits `*`/`**`
        // splat prefixes — Phase 6s); paren-less calls keep bare
        // `expression` children (the call_arg wrapper would create a
        // grammar ambiguity with binary `*` at expression-start
        // position — `a * b` as a statement would parse as `a(splat b)`,
        // which is wrong).  Paren-less splat (`puts *arr`) is therefore
        // a v0 deferred limitation; users who need it can fall back to
        // the parenned form `puts(*arr)`.
        //
        // Phase 6l: trailing `dot_call` repetitions chain method calls
        // onto the result.  Args before the first dot_call belong to
        // the head call; args inside each dot_call belong to that step.
        let (callee, _callee_span) = self.expect_first_name_token(node)?;
        let args: Vec<Expr> = if node.rule_name == "method_call_no_paren" {
            // Legacy shape: bare `expression` children directly.
            node.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                    _ => None,
                })
                .map(|n| self.lower_expression(n))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            // Phase 6s shape: `call_arg` wrappers (with optional splat
            // prefix), collected only from the head call's prefix
            // siblings.
            self.head_call_args(node)
                .into_iter()
                .map(|n| self.lower_call_arg(n))
                .collect::<Result<Vec<_>, _>>()?
        };

        let span = self.span_of(node);
        // Phase 16d (FC) — `raise Foo` / `raise Foo, "msg"` is exception
        // machinery; a module that uses it requests `Feature::Exceptions`
        // (aligning the manifest with begin/rescue from Phase 16a).
        if callee == "raise" {
            self.features_used.insert(Feature::Exceptions);
        }
        let head: Expr = if let Some(effects) = ruby_builtin_effects(&callee) {
            Expr::BuiltinCall {
                name: callee,
                args,
                effects,
                span,
            }
        } else {
            // Unrecognised name — fall back to DirectCall.  SIR
            // backends that can't resolve the name will surface a
            // diagnostic; this keeps the lowering total (no panics).
            Expr::DirectCall {
                fn_name: callee,
                args,
                effects: EffectSet::PURE,
                span,
            }
        };
        // Phase 6l — apply trailing `.method[(...)]` chain steps, if any.
        self.apply_dot_chain(head, node)
    }

    /// Phase 6l+6s helper — return the `call_arg` Node children of
    /// `method_call` that belong to the *head* call (i.e. those that
    /// come before any `dot_call` child).  Without this guard, args
    /// nested inside `dot_call` subtrees would leak into the head call.
    ///
    /// Phase 6s renamed the prior `head_call_expression_children`:
    /// `method_call`'s grammar now wraps each arg in a `call_arg` rule
    /// (so splat/double-splat prefixes have a slot).  Callers route
    /// each returned `call_arg` through [`lower_call_arg`] to unwrap
    /// the `*` / `**` envelope.
    fn head_call_args<'a>(&self, node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
        let mut out = Vec::new();
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "dot_call" {
                    break;
                }
                if n.rule_name == "call_arg" {
                    out.push(n);
                }
            }
        }
        out
    }

    /// Phase 6s / 22b — lower a single `call_arg` node.
    ///
    /// Grammar shape: `call_arg = [ "*" | "**" | "&" ] expression ;`
    ///
    /// Lowering:
    /// - No prefix → return the lowered `expression` as-is.
    /// - `*` prefix → wrap in `BuiltinCall("splat", [inner])` — a
    ///   semantic marker that downstream emitters can detect to expand
    ///   into target-language variadic forwarding.
    /// - `**` prefix → wrap in `BuiltinCall("double_splat", [inner])`
    ///   — same pattern, for keyword-argument spread.
    /// - `&` prefix → wrap in `BuiltinCall("block_pass", [inner])`
    ///   (Phase 22b) — the `&blk` block-pass argument, which converts
    ///   the operand (a Proc, a Symbol via `&:sym`, or any object
    ///   responding to `to_proc`) into the call's block.  Same marker
    ///   pattern: SIR has no first-class block-argument slot, so the
    ///   envelope lets downstream emitters reconstruct `&expr`.
    ///
    /// The BuiltinCall envelope preserves splat semantics through SIR
    /// (where the lossy v0 Param shape can't represent variadic
    /// parameters directly).  Callers downstream can pattern-match the
    /// builtin name to convert back to splat syntax in target source.
    fn lower_call_arg(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // KW7 — keyword argument `name: value` (`f(a: 1)`).  The grammar's
        // FIRST `call_arg` alternative is `NAME COLON expression`, so a
        // keyword arg node carries a COLON token child (the single `:`).
        // We detect that colon and, when present, produce the first-class
        // `Expr::KeywordArg { name, value }` — NOT a trailing hash literal.
        // (Real Ruby desugars `f(a: 1)` to a trailing implicit-hash keyword,
        // but SIR models the keyword as its own node so backends can bind it
        // to the callee's `Keyword` param by name.)  The COLON is matched by
        // value: the lexer emits both `:` and `::` as Colon-typed tokens, but
        // only the single `:` ever appears in this call_arg position (`::` is
        // scope-resolution, which lives inside the `expression` branch).  A
        // splat/block-pass prefix (`*`/`**`/`&`) never co-occurs with a
        // keyword colon, so this check is unambiguous.
        let has_kw_colon = node.children.iter().any(|c| match c {
            ASTNodeOrToken::Token(t) => matches!(t.type_, TokenType::Colon) && t.value == ":",
            _ => false,
        });
        if has_kw_colon {
            // The keyword name is the leading NAME token; the value is the
            // trailing `expression` child.
            let name_tok = node
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                    _ => None,
                })
                .ok_or_else(|| RubyLowerError {
                    message: "keyword call arg missing name token".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
            let value_node = node
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                    _ => None,
                })
                .ok_or_else(|| RubyLowerError {
                    message: "keyword call arg missing value expression".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
            let value = self.lower_expression(value_node)?;
            // Observe the feature so the SIR validator accepts the keyword
            // arg (mirrors the def-side `extract_params` KeywordParams gate).
            self.features_used.insert(Feature::KeywordParams);
            return Ok(Expr::KeywordArg {
                name: name_tok.value.clone(),
                value: Box::new(value),
                span: self.span_of(node),
            });
        }

        // Detect the leading `*` / `**` / `&` token (if present).  All
        // three land on Token children with their value preserved
        // (the 1.8-baseline state machine coalesces `**` into one
        // Name-typed Op token; `*` is a Star token; `&` is a Name-typed
        // Op token — the `&.` safe-nav fusion does not fire because no
        // `.` follows a block-pass `&`).
        let prefix = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.value.as_str(), "*" | "**" | "&") => {
                Some(t.value.clone())
            }
            _ => None,
        });
        let expr_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "call_arg missing expression child".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let inner = self.lower_expression(expr_node)?;
        let span = self.span_of(node);
        // Phase 22c — forward-all argument `...`.  The lexer fuses `...`
        // into a single Name-typed token, so `n(...)` parses with the
        // bare name `...` in the call_arg's expression slot, which lowers
        // to `VarRef { name: "..." }`.  Rewrite that into the nullary
        // marker `BuiltinCall("forward_args", [])`.  A beginless range
        // argument `m(...5)` instead lowers to a `range` BuiltinCall (the
        // `...` is the range operator, not a bare name), so it is left
        // untouched.  `...` is not a legal Ruby identifier, so a bare
        // `VarRef("...")` can only have come from argument forwarding —
        // the match is unambiguous.  A leading `*`/`**`/`&` prefix never
        // co-occurs with `...` (Ruby forbids `*...`), so this check runs
        // only on the no-prefix path.
        if prefix.is_none() {
            if let Expr::VarRef { name, .. } = &inner {
                if name == "..." {
                    return Ok(Expr::BuiltinCall {
                        name: "forward_args".to_string(),
                        args: vec![],
                        effects: EffectSet::PURE,
                        span,
                    });
                }
            }
        }
        Ok(match prefix.as_deref() {
            Some("*") => Expr::BuiltinCall {
                name: "splat".to_string(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span,
            },
            Some("**") => Expr::BuiltinCall {
                name: "double_splat".to_string(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span,
            },
            Some("&") => Expr::BuiltinCall {
                name: "block_pass".to_string(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span,
            },
            _ => inner,
        })
    }

    // -------------------------------------------------------------------
    // Phase 6g — method-with-block lowering
    // -------------------------------------------------------------------

    /// Lower a `method_with_block` node into the SIR shape:
    /// the call itself plus a synthesised `Expr::MakeClosure`
    /// appended as the call's last argument.  Block body becomes a
    /// new top-level `Function` named `__block_<n>` on
    /// `self.user_functions`.
    ///
    /// v0 simplification: block bodies see only their own params
    /// (no captures from the outer scope).  Bodies that reference
    /// outer locals will fail the SIR validator at the `VarRef` stage.
    /// Documented in the crate CHANGELOG as a known limitation.
    fn lower_method_with_block(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Shape:
        //   (NAME | KEYWORD) [LPAREN [expression (COMMA expression)*] RPAREN] block
        // The leading callee name comes first.
        let (callee, _callee_span) = self.expect_first_name_token(node)?;

        // Collect explicit argument expressions (direct `expression`
        // children of the method_with_block node — *not* inside the
        // block).  The block node holds its own `statement` children;
        // we route around it.
        let args: Vec<Expr> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_expression(n))
            .collect::<Result<Vec<_>, _>>()?;

        // Find the trailing `block` subnode and lower it to a hoisted
        // Function.  The Function's name is `__block_<n>` where `n`
        // monotonically counts every block we've lowered so far —
        // unique across the whole module.
        let block_node = self
            .find_node_child(node, "block")
            .ok_or_else(|| RubyLowerError {
                message: "method_with_block missing block subnode".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let (fn_name, capture_values) = self.hoist_block_to_function(block_node)?;

        // Append `MakeClosure` as the trailing arg.  Captures cover the RB2
        // enclosing-block capture (`__sir_block__`, when the block `yield`s)
        // and, per M4, any enclosing locals/params the block reads.
        let bspan = self.span_of(block_node);
        let make_closure = Expr::MakeClosure {
            fn_name: fn_name.clone(),
            captures: capture_values,
            span: bspan,
        };
        self.features_used.insert(Feature::Closures);

        let mut all_args = args;
        all_args.push(make_closure);

        let span = self.span_of(node);
        if let Some(effects) = ruby_builtin_effects(&callee) {
            Ok(Expr::BuiltinCall {
                name: callee,
                args: all_args,
                effects,
                span,
            })
        } else {
            Ok(Expr::DirectCall {
                fn_name: callee,
                args: all_args,
                effects: EffectSet::PURE,
                span,
            })
        }
    }

    /// Hoist a `block` (with one `do_block` or `brace_block` child)
    /// into a synthesised top-level Function on `user_functions`.
    /// Returns the synthesised function name so the caller can refer
    /// to it via `MakeClosure { fn_name }`.
    fn hoist_block_to_function(
        &mut self,
        block_node: &GrammarASTNode,
    ) -> Result<(String, Vec<CaptureValue>), RubyLowerError> {
        // Drill into the do_block / brace_block child.
        let inner = self
            .first_node_child(block_node)
            .ok_or_else(|| RubyLowerError {
                message: "block missing do_block/brace_block child".to_string(),
                line: block_node.start_line.unwrap_or(0),
                column: block_node.start_column.unwrap_or(0),
            })?;

        // Extract block parameters (the `|x, y|` pipe form).  Each
        // Name token *that isn't a `|`* is a parameter.  The lexer
        // classifies bare `|` ops as Name tokens (see
        // ruby-lexer/src/lib.rs::classify_op_token), so we filter
        // them out by value, not by type.
        // Phase 21a (FC) — block-local variables.  The `block_params`
        // pipe contents may contain a `;` (Semicolon token) that
        // separates the parameter names (before) from block-local
        // variable names (after): `{ |x; y, z| … }`.  Names before the
        // semicolon are parameters; names after it are fresh locals
        // scoped to the block body (declared so VarRefs resolve, but
        // NOT added to the parameter list).  We walk the pipe tokens in
        // order, flipping a flag when we hit the `;`.
        let params_node = self.find_node_child(inner, "block_params");
        let mut params: Vec<Param> = Vec::new();
        let mut block_locals: Vec<String> = Vec::new();
        if let Some(pn) = params_node {
            let mut seen_semicolon = false;
            for c in &pn.children {
                match c {
                    ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Semicolon) => {
                        seen_semicolon = true;
                    }
                    ASTNodeOrToken::Token(t)
                        if matches!(t.type_, TokenType::Name) && t.value != "|" =>
                    {
                        if seen_semicolon {
                            block_locals.push(t.value.clone());
                        } else {
                            params.push(Param {
                                name: t.value.clone(),
                                sir_type: None,
                                kind: ParamKind::Required,
                                default: None,
                                span: self.span_of_token(t),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        // Phase 21b (FC) — implicit numbered block parameters.  When a
        // block has NO explicit `|...|` header, Ruby allows the body to
        // reference `_1`..`_9` as positional parameters.  The arity is
        // the highest numbered param used (e.g. a body using `_2` gets
        // params `_1, _2`).  We scan the body tokens for `_1`..`_9`,
        // take the max, and synthesize params `_1`..`_<max>`.  This
        // only applies when no explicit params/block-locals were given
        // (an explicit header always wins; numbered params can't mix).
        if params.is_empty() && block_locals.is_empty() {
            let mut max_numbered: u8 = 0;
            collect_max_numbered_block_param(inner, &mut max_numbered);
            if max_numbered > 0 {
                for n in 1..=max_numbered {
                    params.push(Param {
                        name: format!("_{n}"),
                        sir_type: None,
                        kind: ParamKind::Required,
                        default: None,
                        span: self.span_of(inner),
                    });
                }
            } else if block_uses_implicit_it(inner) {
                // Phase 21c (FC) — implicit `it` parameter (Ruby 3.4).
                // A header-less block referencing bare `it` gets a single
                // synthesized parameter named `it`.  Mutually exclusive
                // with numbered params (`_N` takes precedence above; Ruby
                // forbids mixing them anyway).
                params.push(Param {
                    name: "it".to_string(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: self.span_of(inner),
                });
            }
        }

        // Block params are untyped → declare dynamic-typing.  Block-local
        // variables are likewise dynamically typed.
        if !params.is_empty() || !block_locals.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // Lower the body with a fresh locals+params scope so the
        // outer program's bindings don't leak in (same pattern as
        // `lower_def_statement`).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        // M4 — snapshot the *immediate* enclosing scope (the method or outer
        // block whose body we are lowering inside of) so that, after the body
        // is lowered, we can recognise free reads of its locals/params and
        // capture them.  Cloned now because `saved_*` are moved back into
        // `self` when the block scope is restored below.
        let enclosing_locals = saved_locals.clone();
        let enclosing_params = saved_params.clone();
        // Phase RB2 — whether THIS block is being hoisted while already
        // inside another block.  Captured before we mark ourselves as a
        // block body, so an inner block (hoisted during our body lowering)
        // sees `in_block_body == true` and skips the cross-level capture.
        let nested_in_block = self.in_block_body;
        let saved_in_block = self.in_block_body;
        self.in_block_body = true;
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }
        // Block-local variables (Phase 21a): fresh locals scoped to the
        // block body.  Declared so VarRefs resolve, but NOT params —
        // they shadow outer bindings and start unbound.
        for name in &block_locals {
            self.declared_locals.insert(name.clone());
        }

        // Body statements: every direct `statement` child of the
        // inner do_block / brace_block, in source order.  Tail-
        // expression promotion follows the same rule as
        // `lower_program` / `lower_clause_statements`.
        let body_stmts: Vec<&GrammarASTNode> = inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        if body_stmts.is_empty() {
            value = Some(Expr::NilLit {
                span: self.span_of(inner),
            });
        } else {
            let last_idx = body_stmts.len() - 1;
            for (i, s) in body_stmts.iter().enumerate() {
                let inner_stmt = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                    message: "statement node had no child rule".to_string(),
                    line: s.start_line.unwrap_or(0),
                    column: s.start_column.unwrap_or(0),
                })?;
                let is_tail = i == last_idx;
                let kind = inner_stmt.rule_name.as_str();
                if is_tail && matches!(kind, "expression_stmt" | "method_call") {
                    let v = match kind {
                        "expression_stmt" => {
                            let en = self.first_node_child(inner_stmt).ok_or_else(|| {
                                RubyLowerError {
                                    message: "expression_stmt had no expression child".to_string(),
                                    line: inner_stmt.start_line.unwrap_or(0),
                                    column: inner_stmt.start_column.unwrap_or(0),
                                }
                            })?;
                            self.lower_expression(en)?
                        }
                        "method_call" => self.lower_method_call(inner_stmt)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    // Phase 6r — multi-stmt fan-out for `multi_assignment`.
                    stmts_out.extend(self.lower_statement_inner_multi(inner_stmt)?);
                }
            }
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(inner),
        });

        // Phase 21a (FC) — materialize block-local variables.  Each
        // block-local needs an explicit `LetBinding <name> = nil` at the
        // top of the body so the SIR validator knows the name exists
        // (otherwise VarRefs to it report "references unknown name").
        // They initialize to nil (unbound) — Ruby block-locals start nil.
        if !block_locals.is_empty() {
            let mut prefix: Vec<Stmt> = block_locals
                .iter()
                .map(|name| Stmt::LetBinding {
                    name: name.clone(),
                    sir_type: None,
                    value: Expr::NilLit {
                        span: self.span_of(inner),
                    },
                    span: self.span_of(inner),
                })
                .collect();
            prefix.append(&mut stmts_out);
            stmts_out = prefix;
        }

        // Restore outer scope.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;
        self.in_block_body = saved_in_block;

        // Sequential-assignment fix-up for the block body (see
        // `sequentialize_let_bindings`).
        sequentialize_let_bindings(&mut stmts_out);

        // Assemble the block body so the RB2 yield-capture rewrite can run
        // over it as a unit.
        let mut body = Block {
            stmts: stmts_out,
            value,
            span: self.span_of(inner),
        };

        // Phase RB2 (FC) — a `yield` lexically inside this block belongs to
        // the *enclosing method*, not the block.  When we are lowering a
        // method body (`in_def_body`), rewrite each in-block `yield` to an
        // `IndirectCall` through a captured `__sir_block__` (scope
        // `Capture`, since inside the hoisted function it is a capture, not
        // a parameter), record it as a capture on the hoisted function, and
        // signal the enclosing `def` (via `block_captures_enclosing`) to
        // gain the trailing `__sir_block__` parameter and thread the
        // matching `CaptureValue` at the `MakeClosure`.  At the top level
        // there is no enclosing method, so we leave the raw `yield` (the
        // pre-RB2 behaviour) rather than synthesize a dangling capture.
        // v0 cut-line: only a block lowered *directly* in the method body
        // (not nested inside another block) threads the capture.  Capturing
        // across two block levels would require the intermediate block to
        // re-capture `__sir_block__`; until that chaining exists, a nested
        // block keeps its raw `yield` (valid SIR) rather than emit an
        // invalid cross-level `Param` reference.
        // `captures` lists the hoisted function's capture names; the parallel
        // `capture_values` are the values the caller threads at the
        // `MakeClosure` — kept in lockstep (same order) so capture[i] binds
        // value[i].  The backends prepend captures as leading parameters.
        let mut captures: Vec<Capture> = Vec::new();
        let mut capture_values: Vec<CaptureValue> = Vec::new();
        if self.in_def_body
            && !nested_in_block
            && Self::rewrite_yields_in_block(&mut body, Scope::Capture)
        {
            captures.push(Capture {
                name: BLOCK_PARAM_NAME.to_string(),
                sir_type: None,
            });
            capture_values.push(CaptureValue {
                name: BLOCK_PARAM_NAME.to_string(),
                value: Expr::VarRef {
                    name: BLOCK_PARAM_NAME.to_string(),
                    scope: Scope::Param,
                    span: self.span_of(block_node),
                },
            });
            self.block_captures_enclosing = true;
            self.features_used.insert(Feature::Closures);
            self.features_used.insert(Feature::DynamicTyping);
        }

        // M4 — capture free reads of the immediate enclosing scope's
        // locals/params.  Compute the block's own bound names (params,
        // block-locals, and anything assigned inside the body — those shadow
        // or rebind locally and are NOT captured), then rewrite every free
        // read to `Scope::Capture` and thread the enclosing value in.
        let mut block_bound: HashSet<String> = HashSet::new();
        for p in &params {
            block_bound.insert(p.name.clone());
        }
        for name in &block_locals {
            block_bound.insert(name.clone());
        }
        Self::collect_bound_names_in_block(&body, &mut block_bound);
        // The reserved block param is threaded by RB2, never via this path.
        block_bound.insert(BLOCK_PARAM_NAME.to_string());

        let is_free = |name: &str| {
            !block_bound.contains(name)
                && (enclosing_params.contains(name) || enclosing_locals.contains(name))
        };
        let mut free: Vec<String> = Vec::new();
        Self::recapture_reads_in_block(&mut body, &is_free, &mut free);
        if !free.is_empty() {
            self.features_used.insert(Feature::Closures);
            self.features_used.insert(Feature::DynamicTyping);
        }
        let cap_span = self.span_of(block_node);
        for name in free {
            // The enclosing value-ref scope: an enclosing *param* reads back
            // as `Param`, otherwise an enclosing *local* reads as `Local`.
            let outer_scope = if enclosing_params.contains(&name) {
                Scope::Param
            } else {
                Scope::Local
            };
            captures.push(Capture {
                name: name.clone(),
                sir_type: None,
            });
            capture_values.push(CaptureValue {
                name: name.clone(),
                value: Expr::VarRef {
                    name,
                    scope: outer_scope,
                    span: cap_span.clone(),
                },
            });
        }

        // Mint the synthetic function name and push the hoisted
        // Function onto user_functions.  Underscore-prefixed names
        // are conventionally treated as "compiler-generated" by SIR
        // backends — they should not collide with user-declared
        // identifiers.
        let n = self.block_counter;
        self.block_counter += 1;
        let fn_name = format!("__block_{n}");

        self.user_functions.push(Function {
            name: fn_name.clone(),
            params,
            return_type: None,
            captures,
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(block_node),
        });

        Ok((fn_name, capture_values))
    }

    // -------------------------------------------------------------------
    // Phase 6w — arrow-lambda literal `->(params){body}`
    // -------------------------------------------------------------------

    /// Lower a `lambda_literal` node (`->(params){body}`) into a
    /// `BuiltinCall("lambda", [MakeClosure])` expression.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// lambda_literal = "->" [ LPAREN [ params ] RPAREN ] block ;
    /// ```
    ///
    /// The body is hoisted to a top-level `Function` (named `__block_<n>`,
    /// reusing the same counter as `method_with_block` blocks).  Params
    /// are extracted from the leading `params` subnode (Phase 6s — splat
    /// supported) rather than from `block_params` (the `|x|` form inside
    /// the block), because in `->` syntax the parens-list IS the
    /// parameter list.
    ///
    /// **v0 deferred limitations**:
    /// - Block bodies that reference outer locals lose them — captures
    ///   are NOT computed for v0 (same limitation as Phase 6g blocks).
    /// - If the user writes both `->(x) { |y| … }` (params in parens
    ///   AND a block_params header), the latter is silently ignored;
    ///   only the parens-list is honoured.
    /// - `lambda { … }` and `proc { … }` continue to lower via
    ///   `method_with_block` — they're regular keyword-led calls.
    ///   The SIR builtin table tags both as `BuiltinCall("lambda", …)`
    ///   so downstream emitters see a single closure-construction shape.
    fn lower_lambda_literal(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // 1. Find the `block` subnode (mandatory).
        let block_node = self
            .find_node_child(node, "block")
            .ok_or_else(|| RubyLowerError {
                message: "lambda_literal missing block subnode".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 2. Extract arrow-lambda params from the optional `params`
        //    subnode (Phase 6s: param = [ "*"|"**" ] NAME).
        // Phase P7: extract arrow-lambda params with `name = expr`
        // defaults (see `extract_params`).  `->(a = 1) { ... }` is valid
        // Ruby; the parens-list IS the param list here.
        let params_node = self.find_node_child(node, "params");
        let params: Vec<Param> = self.extract_params(params_node)?;
        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // 3. Hoist the block body to a Function with these params.
        //    Reuse the same machinery as `hoist_block_to_function` but
        //    using OUR `params` (from the parens-list), not the inner
        //    block's `block_params` pipe form.
        let fn_name = self.hoist_lambda_body(block_node, params)?;

        // 4. Emit BuiltinCall("lambda", [MakeClosure]).  Closures
        //    feature auto-set so the validator accepts MakeClosure.
        self.features_used.insert(Feature::Closures);
        let span = self.span_of(node);
        Ok(Expr::BuiltinCall {
            name: "lambda".to_string(),
            args: vec![Expr::MakeClosure {
                fn_name,
                captures: Vec::new(),
                span: span.clone(),
            }],
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Phase 6w helper — hoist a `block` (do_block/brace_block) body
    /// to a top-level Function, taking the params list from the caller
    /// (the arrow lambda's parens-list) rather than from the block's
    /// own `|...|` `block_params` header.
    ///
    /// This is structurally parallel to `hoist_block_to_function` but
    /// with the params source swapped.  Returns the synthesised
    /// function name.
    fn hoist_lambda_body(
        &mut self,
        block_node: &GrammarASTNode,
        params: Vec<Param>,
    ) -> Result<String, RubyLowerError> {
        let inner = self
            .first_node_child(block_node)
            .ok_or_else(|| RubyLowerError {
                message: "block missing do_block/brace_block child".to_string(),
                line: block_node.start_line.unwrap_or(0),
                column: block_node.start_column.unwrap_or(0),
            })?;

        // Lower body with fresh scope (params pre-declared as locals
        // + tracked in current_params so VarRefs get Scope::Param).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // Body statements (same tail-expression promotion rule as the
        // method_with_block hoister).
        let body_stmts: Vec<&GrammarASTNode> = inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        if body_stmts.is_empty() {
            value = Some(Expr::NilLit {
                span: self.span_of(inner),
            });
        } else {
            let last_idx = body_stmts.len() - 1;
            for (i, s) in body_stmts.iter().enumerate() {
                let inner_stmt = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                    message: "statement node had no child rule".to_string(),
                    line: s.start_line.unwrap_or(0),
                    column: s.start_column.unwrap_or(0),
                })?;
                let is_tail = i == last_idx;
                let kind = inner_stmt.rule_name.as_str();
                if is_tail && matches!(kind, "expression_stmt" | "method_call") {
                    let v = match kind {
                        "expression_stmt" => {
                            let en = self.first_node_child(inner_stmt).ok_or_else(|| {
                                RubyLowerError {
                                    message: "expression_stmt had no expression child".to_string(),
                                    line: inner_stmt.start_line.unwrap_or(0),
                                    column: inner_stmt.start_column.unwrap_or(0),
                                }
                            })?;
                            self.lower_expression(en)?
                        }
                        "method_call" => self.lower_method_call(inner_stmt)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    stmts_out.extend(self.lower_statement_inner_multi(inner_stmt)?);
                }
            }
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(inner),
        });

        // Restore outer scope.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;

        // Sequential-assignment fix-up for this block body (see
        // `sequentialize_let_bindings`).
        sequentialize_let_bindings(&mut stmts_out);

        // Mint a synthetic function name (shares the same counter as
        // method_with_block-hoisted blocks).
        let n = self.block_counter;
        self.block_counter += 1;
        let fn_name = format!("__block_{n}");

        self.user_functions.push(Function {
            name: fn_name.clone(),
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: stmts_out,
                value,
                span: self.span_of(inner),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(block_node),
        });

        Ok(fn_name)
    }

    // -------------------------------------------------------------------
    // parameters
    // -------------------------------------------------------------------

    /// Phase P7 (Ruby 1.0) — extract a `def`/lambda parameter list from a
    /// `params` CST node, lowering any `name = expr` default value into
    /// `Param.default`.
    ///
    /// ## Why this is its own helper
    ///
    /// Three call sites (`def_statement`, `endless_def_statement`,
    /// `lambda_literal`) used to inline the same `filter_map` that pulled
    /// the param Name and the splat kind but *silently dropped* the
    /// `= <default>` subtree.  Now that the grammar parses the default
    /// (`param = [ "*" | "**" ] NAME [ EQUALS expression ]`), the default
    /// must be lowered through the normal expression path.  Centralising
    /// it here keeps the three sites in lock-step.
    ///
    /// ## CST shape
    ///
    /// A `param` node has one of these child layouts:
    ///
    /// ```text
    /// param ::= NAME                         # required:   default = None
    ///        |  ("*"|"**") NAME              # rest/kwrest: default = None
    ///        |  NAME EQUALS expression       # optional:   default = Some(expr)
    /// ```
    ///
    /// ## Param-scoped, call-time defaults
    ///
    /// Ruby defaults are evaluated at CALL time and may reference EARLIER
    /// parameters — `def f(a, b = a)` is legal and binds `b` to `a`'s
    /// runtime value.  This matches the SIR model exactly (the validator
    /// checks each default in a scope holding the params declared so far).
    /// To honour that, we lower each default with every *prior* param name
    /// already visible as a `Scope::Param`, so a `VarRef` to an earlier
    /// param resolves correctly.  We snapshot and restore the caller's
    /// `current_params` / `declared_locals` so this temporary visibility
    /// does not leak — the caller re-establishes the full param scope for
    /// the body afterwards.
    ///
    /// Splat (`*rest`) and double-splat (`**kwrest`) params never carry a
    /// default; the grammar is permissive but we attach `default` only to
    /// ordinary params.
    ///
    /// The walk is depth-bounded: it reuses the existing bounded
    /// `lower_expression` for the default subtree and introduces no new
    /// recursion over the CST.
    fn extract_params(
        &mut self,
        params_node: Option<&GrammarASTNode>,
    ) -> Result<Vec<Param>, RubyLowerError> {
        let Some(pn) = params_node else {
            return Ok(Vec::new());
        };

        // Snapshot the caller's scope so the incremental "earlier params
        // are visible" trick below is fully reverted on return.
        let saved_params = self.current_params.clone();
        let saved_locals = self.declared_locals.clone();

        let mut params: Vec<Param> = Vec::new();
        for c in &pn.children {
            let ASTNodeOrToken::Node(param_node) = c else {
                continue;
            };
            if param_node.rule_name != "param" {
                continue;
            }

            // Splat prefix → ParamKind (same detection as before).  The
            // KEYWORD kind (KW7) is NOT a prefix — it is signalled by a
            // trailing COLON token instead (detected just below), so it is
            // handled separately from the `*`/`**` prefix walk here.
            let splat_kind = param_node.children.iter().find_map(|cc| match cc {
                ASTNodeOrToken::Token(t) if t.value == "**" => Some(ParamKind::KwRest),
                ASTNodeOrToken::Token(t) if t.value == "*" => Some(ParamKind::Rest),
                _ => None,
            });

            // KW7 — keyword parameter discriminator.  The grammar's `param`
            // suffix is `[ COLON [ expression ] | EQUALS expression ]`; a
            // COLON token child means this is a *keyword* param (`a:` or
            // `a: 1`), bound by name at the call site rather than by
            // position.  A splat/double-splat never carries a colon, so a
            // COLON unambiguously means `ParamKind::Keyword`.  (The COLON is
            // matched from the token value — the lexer emits both `:` and
            // `::` as Colon-typed tokens, but a param suffix only ever holds
            // the single `:`.)
            let is_keyword = param_node.children.iter().any(|cc| match cc {
                ASTNodeOrToken::Token(t) => matches!(t.type_, TokenType::Colon) && t.value == ":",
                _ => false,
            });

            // The parameter Name token is the one that is not the
            // `*`/`**` prefix.
            let name_tok = param_node.children.iter().find_map(|cc| match cc {
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, TokenType::Name) && t.value != "*" && t.value != "**" =>
                {
                    Some(t)
                }
                _ => None,
            });
            let Some(name_tok) = name_tok else {
                continue;
            };

            // The final param kind: KEYWORD wins over the (absent) splat
            // prefix when a colon is present; otherwise it is the splat
            // kind, defaulting to positional `Required`.
            let kind = if is_keyword {
                ParamKind::Keyword
            } else {
                splat_kind.unwrap_or(ParamKind::Required)
            };

            // Optional default — the trailing `expression` child.  For a
            // positional param it is the `name = expr` default (P7); for a
            // keyword param it is the `name: expr` default (KW7, present ⇒
            // optional keyword, absent ⇒ required keyword).  In BOTH cases
            // the lowered expression becomes `Param.default`.  Rest/kwrest
            // splat params keep `default: None` (the grammar is permissive
            // but Ruby never defaults a splat).
            let default_node = param_node.children.iter().find_map(|cc| match cc {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            });
            let default = match (kind, default_node) {
                // Positional optional (`a = 1`) — the P7 path.
                (ParamKind::Required, Some(expr_node)) => {
                    let lowered = self.lower_expression(expr_node)?;
                    self.features_used.insert(Feature::DefaultParams);
                    Some(Box::new(lowered))
                }
                // Keyword optional (`a: 1`) — the KW7 path.  A keyword param
                // with a default is an OPTIONAL keyword; one without is a
                // REQUIRED keyword (the validator enforces that required
                // keywords are supplied at each call).
                (ParamKind::Keyword, Some(expr_node)) => {
                    let lowered = self.lower_expression(expr_node)?;
                    Some(Box::new(lowered))
                }
                _ => None,
            };

            // KW7 — any keyword param makes the module observe the
            // `KeywordParams` feature so the SIR validator accepts it
            // (mirrors how a positional default observes `DefaultParams`).
            if kind == ParamKind::Keyword {
                self.features_used.insert(Feature::KeywordParams);
            }

            // Make THIS param visible to LATER params' defaults
            // (`def f(a, b = a)`), then record it.
            self.current_params.insert(name_tok.value.clone());
            self.declared_locals.insert(name_tok.value.clone());

            params.push(Param {
                name: name_tok.value.clone(),
                sir_type: None,
                kind,
                default,
                span: self.span_of_token(name_tok),
            });
        }

        // Revert the temporary scope visibility — the caller owns the
        // real body scope.
        self.current_params = saved_params;
        self.declared_locals = saved_locals;

        Ok(params)
    }

    // -------------------------------------------------------------------
    // expression / term / factor
    // -------------------------------------------------------------------

    /// Issue #59 — lower a `super_expr` (or, historically, `super_statement`)
    /// node to the value-producing `__super__(method, class, …args)` builtin.
    ///
    /// Grammar shape (Phase 22d, now hosted in `factor` per #59):
    ///   super_expr = "super" [ super_args ] ;
    ///   super_args = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
    ///              | call_arg { COMMA call_arg } ;
    ///
    /// Two distinct lowerings keyed on whether a `super_args` node is PRESENT:
    ///   - present → explicit arg list (`super()`, `super(x)`, `super x`): the
    ///     lowered args are forwarded verbatim.
    ///   - absent  → bare `super` ("zsuper") forwards ALL of the enclosing
    ///     method's arguments implicitly.  Real Ruby re-passes the *current*
    ///     method's parameters; O2 makes that concrete by forwarding a `VarRef`
    ///     to each of the enclosing method's params (declaration order — well,
    ///     sorted for determinism, since `current_params` is a set), so
    ///     `def describe; super; end` inside a param-taking method forwards
    ///     them.  A method that takes no params forwards nothing.
    ///
    /// O2 (OOP production) — `super` is emitted as the OOP-runtime builtin
    /// `__super__(method_name, class_name, …args)`.  The runtime walks from
    /// `class_name`'s *parent* to find the first ancestor implementation of
    /// `method_name` and runs it with the *current* self still bound.  We
    /// thread the enclosing method + class names from the lowerer context
    /// (`current_method` / `current_class`).  Outside a class method (`super`
    /// at top level — not legal Ruby, but the parser admits it) both are empty
    /// strings; the runtime then finds no parent and returns nil (the honest
    /// floor).
    ///
    /// Effects: PURE, matching `yield` — `super` dispatches to a parent method
    /// whose own effects are accounted for at its definition/call site;
    /// modelling the marker as PURE keeps the effect lattice from
    /// double-counting.  Returning an `Expr` (not a `Stmt`) is what lets #59's
    /// `super + " tail"` / `x = super` slot `super` anywhere an expression goes.
    fn lower_super_expr(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let super_args_node = self.find_node_child(node, "super_args");
        let forwarded: Vec<Expr> = if let Some(sa) = super_args_node {
            let call_arg_nodes: Vec<&GrammarASTNode> = sa
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                    _ => None,
                })
                .collect();
            call_arg_nodes
                .into_iter()
                .map(|n| self.lower_call_arg(n))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut params: Vec<String> = self.current_params.iter().cloned().collect();
            params.sort();
            params
                .into_iter()
                .map(|p| Expr::VarRef {
                    name: p,
                    scope: Scope::Param,
                    span: self.span_of(node),
                })
                .collect()
        };
        let method_name = self.current_method.clone().unwrap_or_default();
        let class_name = self.current_class.clone().unwrap_or_default();
        self.features_used.insert(Feature::Classes);
        self.features_used.insert(Feature::Strings);
        let mut full_args: Vec<Expr> = Vec::with_capacity(forwarded.len() + 2);
        full_args.push(Expr::StrLit {
            value: method_name,
            span: self.span_of(node),
        });
        full_args.push(Expr::StrLit {
            value: class_name,
            span: self.span_of(node),
        });
        full_args.extend(forwarded);
        Ok(Expr::BuiltinCall {
            name: "__super__".to_string(),
            args: full_args,
            effects: EffectSet::PURE,
            span: self.span_of(node),
        })
    }

    fn lower_expression(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Pass through wrapper rules transparently — the parser
        // sometimes nests `expression → sum → term → factor → expression`.
        match node.rule_name.as_str() {
            // Phase 6m: `expression` is now the top of the logical
            // chain.  It contains exactly one child node — a
            // `logical_or`.  Pass through transparently.
            //
            // The comparison-op chain that used to live directly under
            // `expression` (pre-6m) has moved to the dedicated
            // `comparison` rule (lowered the same way the old
            // `expression` was — via `lower_comparison_chain`).
            "expression" => {
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression had no inner rule".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                self.lower_expression(inner)
            }
            // Phase 6m — logical-OR chain: `a || b || c || …`.
            // Folds left-associatively into nested
            // `BuiltinCall("or", [lhs, rhs])`.  Operator forms `||`
            // (symbol) and `or` (keyword) lower identically — see the
            // grammar comment for the v0 simplification.
            // Phase 6o — ternary `cond ? a : b`.  Either a bare
            // `range` pass-through or an `Expr::If` with single-expression
            // branch blocks.  Lowers identically to `if cond then a else b end`
            // so downstream emitters need no new code path.
            "ternary" => self.lower_ternary(node),
            // Phase 6n — range expressions `a..b` (inclusive) and
            // `a...b` (exclusive).  Either a bare `logical_or` pass-through
            // or a `BuiltinCall("range", [start, end, BoolLit(exclusive)])`.
            "range" => self.lower_range(node),
            "logical_or" => self.lower_logical_chain(node, &["||", "or"], "or"),
            // Phase 6m — logical-AND chain: same pattern as logical_or.
            "logical_and" => self.lower_logical_chain(node, &["&&", "and"], "and"),
            // Phase 6m — `logical_not`.  Two shapes:
            //   - prefix `!` or `not` → BuiltinCall("not", [inner])
            //   - bare passthrough to `comparison` (no leading op)
            "logical_not" => self.lower_logical_not(node),
            // Phase 6m — the comparison chain rule (renamed from the
            // old `expression`).  Same lowering as before.
            "comparison" => self.lower_comparison_chain(node),
            "sum" => self.lower_binary_chain(node, &["PLUS", "MINUS"]),
            "term" => self.lower_binary_chain(node, &["STAR", "SLASH"]),
            "factor" => self.lower_factor(node),
            "unary_minus" => {
                // Phase 6k — `-x` → BuiltinCall("neg", [x]).
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "unary_minus had no factor child".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                let operand = self.lower_expression(inner)?;
                Ok(Expr::BuiltinCall {
                    name: "neg".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of(node),
                })
            }
            // Phase 23b (FC) — `defined?(x)` in expression position
            // (assignment RHS, condition, …).
            "defined_expression" => self.lower_defined_expression(node),
            "array_literal" => self.lower_array_literal(node),
            "hash_literal" => self.lower_hash_literal(node),
            "symbol_literal" => self.lower_symbol_literal(node),
            // Phase 6l — `method_call` may now appear in expression
            // position because it's the first atom alternative inside
            // `factor`.  Reuse the statement-level lowerer; it handles
            // the trailing `{ dot_call }` chain transparently.
            "method_call" => self.lower_method_call(node),
            // Phase 6w — arrow-lambda literal `->(params){body}`.
            "lambda_literal" => self.lower_lambda_literal(node),
            // Issue #59 — `super` as an expression (`x = super`,
            // `super + " tail"`, `puts(super)`).  Lowers to the same
            // value-producing `__super__(method, class, …args)` builtin the
            // statement form uses.
            "super_expr" => self.lower_super_expr(node),
            // The parser sometimes wraps a bare token into an "expression_stmt"
            // when reached as the RHS of an assignment.  Recurse into it.
            "expression_stmt" => {
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression_stmt had no inner expression".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                self.lower_expression(inner)
            }
            other => Err(RubyLowerError {
                message: format!("unsupported expression shape `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    /// Lower a left-associative chain of binary operators.  Used for
    /// both `expression` (PLUS / MINUS) and `term` (STAR / SLASH) —
    /// the only difference is the operator set.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        ops: &[&str],
    ) -> Result<Expr, RubyLowerError> {
        // Walk children in order.  The first must be a sub-expression
        // node; subsequent pairs are (op-token, sub-expression node).
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<(String, Span)> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some((op_name, op_span))) => Expr::BuiltinCall {
                            name: op_name,
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(lhs), None) => {
                            // Two sibling sub-expressions with no operator
                            // between them — should not happen with the
                            // v0 grammar; treat as an internal error.
                            return Err(RubyLowerError {
                                message: "two consecutive expression children without an operator"
                                    .to_string(),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            }
                            .also(lhs));
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    if ops.iter().any(|op| token_type_name(tok.type_) == *op) {
                        pending_op = Some((
                            token_lexeme_for_op(tok.type_).to_string(),
                            self.span_of_token(tok),
                        ));
                    }
                    // Other tokens (whitespace, newline) are dropped.
                }
            }
        }

        acc.ok_or_else(|| RubyLowerError {
            message: "binary chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    /// Lower the `expression` rule's comparison-operator chain.
    /// Phase 6i — supports `==`, `!=`, `<`, `>`, `<=`, `>=` as
    /// left-associative BuiltinCalls.
    ///
    /// The lexer's `classify_op_token` reclassifies most comparison
    /// operators as `Name`-type tokens (its catch-all branch — only
    /// `==` gets a dedicated `EqualsEquals` type).  So we identify
    /// comparison operators by *value*, not by token type — the same
    /// trick used for `=>` in `hash_entry`.  This means the helper is
    /// resilient to the lexer's classifier changing in the future.
    fn lower_comparison_chain(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        const COMPARISON_OPS: &[&str] = &["==", "!=", "<", ">", "<=", ">="];
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<(String, Span)> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some((op_name, op_span))) => Expr::BuiltinCall {
                            name: op_name,
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(lhs), None) => {
                            return Err(RubyLowerError {
                                message:
                                    "two consecutive sum sub-expressions without a comparison \
                                     operator between them"
                                        .to_string(),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            }
                            .also(lhs));
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    // Match by lexeme — covers both EqualsEquals
                    // (`==`) and Name-classified operators (`<`, `>`,
                    // `<=`, `>=`, `!=`).
                    if COMPARISON_OPS.iter().any(|op| *op == tok.value) {
                        pending_op = Some((tok.value.clone(), self.span_of_token(tok)));
                    }
                    // Whitespace/newline tokens fall through silently.
                }
            }
        }
        acc.ok_or_else(|| RubyLowerError {
            message: "comparison chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6m — logical operators `&&`, `||`, `and`, `or`, `!`, `not`
    // -------------------------------------------------------------------

    /// Lower a left-associative logical chain (`logical_or` /
    /// `logical_and`).  `op_lexemes` is the set of accepted operator
    /// lexemes (e.g. `["||", "or"]`).  `builtin_name` is the SIR
    /// builtin name to emit (e.g. `"or"`).  Both the symbol form
    /// (`||`) and keyword form (`or`) collapse to the same builtin —
    /// see the grammar comment for why v0 doesn't distinguish them.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        op_lexemes: &[&str],
        builtin_name: &str,
    ) -> Result<Expr, RubyLowerError> {
        let mut acc: Option<Expr> = None;
        let mut pending_op_span: Option<Span> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op_span.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some(op_span)) => Expr::BuiltinCall {
                            name: builtin_name.to_string(),
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(_), None) => {
                            return Err(RubyLowerError {
                                message: format!(
                                    "logical chain had two consecutive operands without `{}`",
                                    op_lexemes.join("/")
                                ),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            });
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    // Match operator by lexeme — `||`/`&&` lex as Name
                    // tokens (catch-all in classify_op_token), `and`/`or`
                    // lex as Keyword tokens.  Both reach us by value.
                    if op_lexemes.iter().any(|l| *l == tok.value) {
                        pending_op_span = Some(self.span_of_token(tok));
                    }
                }
            }
        }
        acc.ok_or_else(|| RubyLowerError {
            message: "logical chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    /// Lower a `logical_not` node.  Shape: `{ "!" | "not" } comparison`.
    /// Each leading `!` or `not` wraps the inner expression in another
    /// `BuiltinCall("not", …)` layer — so `!!x` produces `not(not(x))`.
    fn lower_logical_not(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let not_count = node
            .children
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    ASTNodeOrToken::Token(t) if t.value == "!" || t.value == "not"
                )
            })
            .count();
        // The single Node child is the inner `comparison` expression.
        let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
            message: "logical_not had no inner expression".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let mut expr = self.lower_expression(inner)?;
        // Wrap once per leading `!` / `not` token.
        for _ in 0..not_count {
            expr = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![expr],
                effects: EffectSet::PURE,
                span: self.span_of(node),
            };
        }
        Ok(expr)
    }

    // -------------------------------------------------------------------
    // Phase 6n — range expressions `..` / `...`
    // -------------------------------------------------------------------

    /// Lower a `range` node.  Grammar shape (Phase 10c made the trailing
    /// operand optional):
    ///
    ///   range = logical_or [ ( "..." | ".." ) [ logical_or ] ]
    ///
    /// Three cases:
    ///   - One operand child, no `..`/`...` token → pass through (the
    ///     range rule is just a transparent wrapper in this case).
    ///   - Two operand children with a `..` or `...` token between them
    ///     → emit `BuiltinCall("range", [start, end, BoolLit(exclusive)])`.
    ///     The third argument carries the inclusive/exclusive flag so a
    ///     single builtin handles both forms without name multiplication.
    ///     `..` → exclusive=false; `...` → exclusive=true.
    ///   - One operand child WITH a `..`/`...` token → either a Phase 10c
    ///     endless range (`1..`, child order `[operand, op]`) or a Phase
    ///     10d beginless range (`..5`, child order `[op, operand]`),
    ///     disambiguated by the op token's position.  Endless emits
    ///     `[start, NilLit, excl]`; beginless emits `[NilLit, end, excl]`
    ///     — the missing endpoint is the nil one.
    ///
    /// Range is pure: building a range doesn't observe or mutate any
    /// state.  (Iterating over one *would* run code, but that's a
    /// separate call.)
    fn lower_range(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Collect operand sub-nodes (each a `logical_or`).
        let operands: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .collect();

        // Find the `..` or `...` operator token (if present).
        let op_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value == ".." || t.value == "..." => Some(t),
            _ => None,
        });

        match (operands.len(), op_tok) {
            // Bare logical_or pass-through — no range operator.
            (1, None) => self.lower_expression(operands[0]),
            // Two operands separated by `..` or `...`.
            (2, Some(tok)) => {
                let start = self.lower_expression(operands[0])?;
                let end = self.lower_expression(operands[1])?;
                let exclusive = tok.value == "...";
                let op_span = self.span_of_token(tok);
                Ok(Expr::BuiltinCall {
                    name: "range".to_string(),
                    args: vec![
                        start,
                        end,
                        // The third arg is a flag — `true` means
                        // exclusive (`...`), `false` means inclusive
                        // (`..`).  Carrying it as data keeps the
                        // builtin's signature uniform.
                        Expr::BoolLit { value: exclusive, span: op_span.clone() },
                    ],
                    effects: EffectSet::PURE,
                    span: op_span,
                })
            }
            // One operand plus a range op — this is EITHER a Phase 10c
            // endless range (`1..`, child order `[operand, op]`) OR a
            // Phase 10d beginless range (`..5`, child order `[op,
            // operand]`).  They have identical arity, so we disambiguate
            // by the position of the op token relative to the operand.
            // Both lower to the uniform `range` builtin with the missing
            // endpoint encoded as `NilLit`:
            //   endless  `1..`  → [start,  NilLit, exclusive]  (nil end)
            //   beginless `..5` → [NilLit, end,    exclusive]  (nil start)
            // The exclusive flag distinguishes `..` from `...` as always.
            (1, Some(tok)) => {
                let op_idx = node.children.iter().position(|c| {
                    matches!(c, ASTNodeOrToken::Token(t) if t.value == ".." || t.value == "...")
                });
                let operand_idx = node
                    .children
                    .iter()
                    .position(|c| matches!(c, ASTNodeOrToken::Node(_)));
                // Beginless when the op token comes BEFORE the operand.
                let beginless = matches!((op_idx, operand_idx), (Some(o), Some(p)) if o < p);

                let operand = self.lower_expression(operands[0])?;
                let exclusive = tok.value == "...";
                let op_span = self.span_of_token(tok);
                let args = if beginless {
                    // `..5` — nil (unbounded) lower bound, `operand` is the end.
                    vec![
                        Expr::NilLit { span: op_span.clone() },
                        operand,
                        Expr::BoolLit { value: exclusive, span: op_span.clone() },
                    ]
                } else {
                    // `1..` — `operand` is the start, nil (unbounded) upper bound.
                    vec![
                        operand,
                        Expr::NilLit { span: op_span.clone() },
                        Expr::BoolLit { value: exclusive, span: op_span.clone() },
                    ]
                };
                Ok(Expr::BuiltinCall {
                    name: "range".to_string(),
                    args,
                    effects: EffectSet::PURE,
                    span: op_span,
                })
            }
            // Shouldn't happen given the grammar shape — but be
            // defensive: a missing operator with two operands or any
            // other operand/op combination points at a grammar
            // regeneration gone awry.
            (n, _) => Err(RubyLowerError {
                message: format!(
                    "range node had {n} operand(s) and op={:?} — expected (1, None), (1, Some(..|...)), or (2, Some(..|...))",
                    op_tok.map(|t| t.value.clone()),
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6o — ternary `cond ? a : b`
    // -------------------------------------------------------------------

    /// Lower a `ternary` node.  Grammar shape:
    ///
    ///   ternary = range [ "?" expression ":" expression ]
    ///
    /// Two cases:
    ///   - One operand sub-node (just a `range`, no `?`) → pass through.
    ///   - Three operand sub-nodes (`range "?" expression ":" expression`)
    ///     → emit `Expr::If` wrapping each branch in a single-expression
    ///     `Block`.  Lowers identically to `if cond then a else b end`
    ///     so all downstream emitters (semantic-ir-to-python, etc.) reuse
    ///     existing if-lowering code paths.
    ///
    /// Right-associativity (`a ? b : c ? d : e` → `a ? b : (c ? d : e)`)
    /// is enforced by the grammar's recursion into `expression` for the
    /// false branch.  Each `expression` recursion bottoms back out at
    /// `ternary` at the top of the precedence pyramid, so the inner
    /// ternary appears as the else-branch's value.
    fn lower_ternary(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Collect operand sub-nodes (each is an `expression`-shaped
        // subtree: the first is `range`, the trailing two are
        // `expression`).
        let operands: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .collect();

        match operands.len() {
            // Bare range pass-through — no `?` operator.
            1 => self.lower_expression(operands[0]),
            // cond ? then : else — three operand sub-nodes.
            3 => {
                let cond = self.lower_expression(operands[0])?;
                let then_value = self.lower_expression(operands[1])?;
                let else_value = self.lower_expression(operands[2])?;
                let span = self.span_of(node);
                let then_block = Block {
                    stmts: Vec::new(),
                    value: then_value,
                    span: span.clone(),
                };
                let else_block = Block {
                    stmts: Vec::new(),
                    value: else_value,
                    span: span.clone(),
                };
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_block),
                    else_branch: Box::new(else_block),
                    span,
                })
            }
            n => Err(RubyLowerError {
                message: format!(
                    "ternary node had {n} operand sub-node(s) — expected 1 (pass-through) or 3 (cond/then/else)",
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    /// Lower a `factor` node — the leaves of the expression tree.
    /// Phase 6e — `:foo` / `:"bar"` → `Expr::SymLit`.  The leading
    /// COLON is the syntactic marker; the symbol's *name* is the
    /// Name/Keyword/String token that follows.  For quoted forms
    /// (`:"hello world"`) the String token's value already strips
    /// the surrounding quotes, so we use it verbatim.
    fn lower_symbol_literal(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let name_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if matches!(
                    t.type_,
                    TokenType::Name | TokenType::Keyword | TokenType::String
                ) =>
            {
                Some(t)
            }
            _ => None,
        });
        let name_tok = name_tok.ok_or_else(|| RubyLowerError {
            message: "symbol_literal missing payload token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        self.features_used.insert(Feature::Symbols);
        Ok(Expr::SymLit {
            name: name_tok.value.clone(),
            span: self.span_of(node),
        })
    }

    /// Phase 6d — `[a, b, c]` → `Expr::SeqLit`.
    fn lower_array_literal(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let items: Vec<Expr> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_expression(n))
            .collect::<Result<Vec<_>, _>>()?;
        // SIR's SeqLit allocates a runtime list — declare the
        // `sequences` feature so the validator accepts it.
        self.features_used.insert(Feature::Sequences);
        Ok(Expr::SeqLit {
            items,
            span: self.span_of(node),
        })
    }

    /// Phase 6d — `{a: 1, b => 2}` → `Expr::MapLit`.  Both the
    /// `NAME COLON expression` shorthand and the `expression => expression`
    /// hash-rocket form lower to the same node — the key becomes a
    /// `SymLit` for the shorthand (since `a:` is sugar for `:a =>`)
    /// or whatever the LHS expression evaluates to for the rocket
    /// form.
    fn lower_hash_literal(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let entry_nodes: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry" => Some(n),
                _ => None,
            })
            .collect();
        let mut entries: Vec<semantic_ir::nodes::MapEntry> = Vec::with_capacity(entry_nodes.len());
        for ent in &entry_nodes {
            entries.push(self.lower_hash_entry(ent)?);
        }
        self.features_used.insert(Feature::Maps);
        Ok(Expr::MapLit {
            entries,
            span: self.span_of(node),
        })
    }

    fn lower_hash_entry(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<semantic_ir::nodes::MapEntry, RubyLowerError> {
        // Three shapes are possible:
        //   1. `NAME COLON expression` — keyword-style shorthand.  The
        //      Name token is the symbol key; the trailing expression
        //      is the value.
        //   2. `NAME COLON` — Ruby 3.1 value-omitted shorthand (Phase
        //      7f).  The Name token is the symbol key AND the value is
        //      a `VarRef` to a same-named local variable in scope.
        //   3. `expression "=>" expression` — hash-rocket.  Two
        //      `expression` rule children.
        let expression_subnodes: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .collect();
        if expression_subnodes.len() == 2 {
            // Rocket form.
            let key = self.lower_expression(expression_subnodes[0])?;
            let value = self.lower_expression(expression_subnodes[1])?;
            return Ok(semantic_ir::nodes::MapEntry { key, value });
        }
        // Shorthand form (cases 1 and 2) — find the leading Name token.
        let key_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let key_tok = key_tok.ok_or_else(|| RubyLowerError {
            message: "hash_entry missing key Name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let key_span = self.span_of_token(key_tok);
        let key = Expr::SymLit {
            name: key_tok.value.clone(),
            span: key_span.clone(),
        };
        self.features_used.insert(Feature::Symbols);
        let value = if let Some(value_node) = expression_subnodes.first() {
            // Case 1 — explicit value expression follows the colon.
            self.lower_expression(value_node)?
        } else {
            // Case 2 — Ruby 3.1 value-omitted shorthand `{name:}`.
            // Value is a `VarRef` to the same-named local variable.
            // The scope follows the same Param-vs-Local dispatch used
            // by the bare-name factor lowering above: if the binding
            // exists in `current_params`, mark it `Param`; otherwise
            // mark it `Local` and let the validator catch any unbound
            // reference.
            let scope = if self.current_params.contains(&key_tok.value) {
                Scope::Param
            } else {
                Scope::Local
            };
            Expr::VarRef {
                name: key_tok.value.clone(),
                scope,
                span: key_span,
            }
        };
        Ok(semantic_ir::nodes::MapEntry { key, value })
    }

    // -------------------------------------------------------------------
    // Phase 7b — heredoc literal lowering
    // -------------------------------------------------------------------

    /// Lower a Ruby heredoc literal (Phase 7b).
    ///
    /// The lexer's Phase-3c body capture and Phase-4o opener-variant
    /// handling finalise every heredoc into a single `TokenType::String`
    /// token whose value is the verbatim canonical form:
    ///
    /// ```text
    /// <<TAG\n<body>TAG     (Plain)
    /// <<-TAG\n<body>TAG    (DashIndent — closing tag may be indented)
    /// <<~TAG\n<body>TAG    (TildeIndent — common indent already stripped)
    /// ```
    ///
    /// Note: the `<<~TAG` indent-stripping is already applied by the
    /// lexer before we see the token, so we don't repeat it here.
    ///
    /// ## SIR shape
    ///
    /// `StrLit(body)` — the inner body (opener line + tag suffix
    /// stripped) is emitted as a plain string literal.  This matches
    /// Ruby's surface semantics: a heredoc *is* a string literal, just
    /// with a different on-source representation.  Triggers
    /// `Feature::Strings`.
    ///
    /// ## v0 deferred limitations
    ///
    /// - Interpolation inside the body (`#{name}`) is NOT split.  The
    ///   body is emitted as a single `StrLit` with any `#{...}` markers
    ///   preserved verbatim.  Follow-up will reuse the Phase 6y
    ///   interpolation splitter.
    /// - Non-interpolating heredocs (`<<'TAG'`) and the `<<"TAG"` form
    ///   are not yet distinguished — the lexer doesn't carry the quote
    ///   state, so we treat every heredoc the same.
    /// - Escape sequences inside the body are kept literal (the lexer's
    ///   heredoc capture does not unescape; same v0 stance as backticks).
    fn lower_heredoc_literal(
        &mut self,
        raw: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Expr {
        // Step 1: strip the opener prefix.  Order matters — `<<~` and
        // `<<-` must be tried *before* `<<`, because `<<~`/`<<-` both
        // start with `<<`.
        let after_prefix = raw
            .strip_prefix("<<~")
            .or_else(|| raw.strip_prefix("<<-"))
            .or_else(|| raw.strip_prefix("<<"))
            .unwrap_or(raw);

        // Step 2: split off the tag (everything up to the first `\n`).
        // The remainder is `<body><tag>` (the trailing tag was appended
        // by the lexer's `finalize_heredoc`).
        let body: String = if let Some(nl_idx) = after_prefix.find('\n') {
            let tag = &after_prefix[..nl_idx];
            let body_plus_tag = &after_prefix[nl_idx + 1..];
            // Strip the closing tag from the end.  Defensive `or`:
            // if the suffix isn't present (a lexer bug), keep the
            // body intact rather than panicking.
            body_plus_tag
                .strip_suffix(tag)
                .unwrap_or(body_plus_tag)
                .to_string()
        } else {
            // Pathological: opener with no newline.  Shouldn't be
            // possible given the lexer's finalise path, but handle it
            // by treating the whole thing as the body.
            after_prefix.to_string()
        };

        // Step 3 (Phase 17a FC): a plain / `<<-` / `<<~` heredoc
        // interpolates like a double-quoted string, so route the
        // extracted body through the shared interpolation splitter
        // rather than treating it as fully literal.  Bodies with no
        // `#{…}` come back as a single `StrLit` (the prior behaviour);
        // bodies that interpolate become a `StrConcat` whose segments
        // are the literal runs and the lowered `#{…}` expressions —
        // exactly as `"a#{x}b"` lowers.  A lowering error (malformed
        // interpolation body) falls back to the verbatim body as a
        // plain `StrLit`, keeping heredoc lowering infallible.
        match self.lower_string_literal_with_interp(&body, span.clone(), err_line, err_column) {
            Ok(expr) => expr,
            Err(_) => {
                self.features_used.insert(Feature::Strings);
                Expr::StrLit { value: body, span }
            }
        }
    }

    // -------------------------------------------------------------------
    // Phase 7a — backtick command literal lowering
    // -------------------------------------------------------------------

    /// Lower a Ruby backtick command literal `` `cmd args` `` (Phase 7a).
    ///
    /// The lexer's Phase-4m `backtick_body` state emits the entire
    /// literal — including the surrounding backticks — as a single
    /// `TokenType::String` token whose `value` is `` `<body>` `` (the
    /// inner body wrapped back up).  This sentinel-by-prefix trick
    /// lets the parser route both plain strings and backtick literals
    /// through the same NUMBER/STRING/NAME factor alternation while
    /// preserving the distinction for the lowerer.
    ///
    /// ## SIR shape
    ///
    /// `BuiltinCall { name: "backtick", args: [StrLit(body)] }` — the
    /// inner body (backticks stripped) is carried as a `StrLit` arg.
    /// Effects are `MayBlock | MayPrint | MayThrow`: command execution
    /// can block (waiting for the child process), print (stdout/stderr
    /// from the child), and throw (`Errno::ENOENT` and friends).  Same
    /// marker-builtin pattern as Phase 6v's `__rescue_marker__`,
    /// Phase 6w's lambda construction, and Phase 6y's `__interp__`.
    ///
    /// ## v0 deferred limitations
    ///
    /// - Interpolation inside the body (`` `echo #{name}` ``) is NOT
    ///   split.  The body is emitted as a single `StrLit` with any
    ///   `#{...}` markers preserved verbatim.  A future phase will
    ///   reuse the Phase 6y interpolation splitter inside the body.
    /// - Escape sequences inside the body (`` \` ``, `\n`, etc.) are
    ///   resolved by the lexer (Phase 4m's body state) before reaching
    ///   us — we don't re-process them here.
    /// - Triggers `Feature::Strings` because we emit a `StrLit`.
    fn lower_backtick_command_literal(&mut self, raw: &str, span: Span) -> Expr {
        // Strip the surrounding backticks.  The lexer guarantees the
        // value is `` `<body>` ``, so the first and last bytes are
        // always ASCII `` ` `` (single-byte) — we can slice on bytes.
        // Defensive fallback: if either delimiter is missing (which
        // would be a lexer bug), treat the whole value as the body so
        // we don't panic on a malformed input.
        let body = if raw.len() >= 2 && raw.starts_with('`') && raw.ends_with('`') {
            &raw[1..raw.len() - 1]
        } else {
            raw
        };
        self.features_used.insert(Feature::Strings);
        Expr::BuiltinCall {
            name: "backtick".to_string(),
            args: vec![Expr::StrLit {
                value: body.to_string(),
                span: span.clone(),
            }],
            // Backtick execution can block on the child process, print
            // its output, and throw if the command can't be invoked
            // (`Errno::ENOENT`, etc).
            effects: EffectSet::PURE
                .with(Effect::MayBlock)
                .with(Effect::MayPrint)
                .with(Effect::MayThrow),
            span,
        }
    }

    // -------------------------------------------------------------------
    // Phase 19a (FC) — regex literal lowering (`/pattern/flags`)
    // -------------------------------------------------------------------

    /// Lower a Ruby regex literal `/pattern/flags` (Phase 19a).
    ///
    /// The lexer's `regex_body` sub-machine (entered via `should_open_regex`,
    /// which resolves the classic `/`-is-regex-vs-division ambiguity from the
    /// lex state) emits the whole literal as a single `TokenType::String`
    /// token whose value is the verbatim `` /pattern/flags `` (leading slash
    /// included).  This is the same lexeme-prefix sentinel trick the percent
    /// literals, heredocs (`<<`), and backticks (`` ` ``) use — the parser
    /// routes a regex through the ordinary string-literal slot.
    ///
    /// We split the verbatim lexeme into `pattern` and `flags` (see
    /// [`regex_pattern_flags`]) and emit
    /// `BuiltinCall("regex", [<pattern-expr>, StrLit(flags)])`.  Carrying
    /// the flags as a separate `StrLit` arg keeps the builtin uniform
    /// across all flag combinations (`/x/`, `/x/i`, `/x/im`, …) without
    /// name multiplication — mirroring the `range` builtin's flag arg.
    ///
    /// ## Phase 19c — interpolation
    ///
    /// The `regex_body` lexer state does not special-case `#{...}` — it
    /// accumulates the markers verbatim into the pattern — so an
    /// interpolated regex `` /a#{b}c/ `` arrives here with the pattern
    /// `a#{b}c`.  We therefore lower the pattern through the SAME
    /// interpolation splitter string literals use
    /// ([`lower_string_literal_with_interp`]):
    ///   - no `#{...}`  → `args[0]` is a plain `StrLit(pattern)` (the
    ///     Phase-19a/19b shape, unchanged);
    ///   - with `#{...}` → `args[0]` is a `string_concat` (or a single
    ///     `VarRef` for `` /#{x}/ ``) over the literal + interpolated
    ///     segments — exactly how `"a#{b}c"` lowers.
    ///
    /// Building a regex object is pure (no I/O, no mutation).  We emit
    /// `StrLit`s, so the literal requests `Feature::Strings` — the same
    /// stance as backticks and heredocs.  (v0 does NOT unescape the body:
    /// `\/` etc. survive verbatim in the pattern, matching the heredoc /
    /// backtick capture stance.)
    fn lower_regex_literal(
        &mut self,
        pattern: &str,
        flags: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Result<Expr, RubyLowerError> {
        self.features_used.insert(Feature::Strings);
        // Run the pattern through the string interpolation splitter so
        // `#{...}` markers inside the regex become real sub-expressions.
        // For a marker-free pattern this returns a plain `StrLit`.
        let pattern_expr =
            self.lower_string_literal_with_interp(pattern, span.clone(), err_line, err_column)?;
        Ok(Expr::BuiltinCall {
            name: "regex".to_string(),
            args: vec![
                pattern_expr,
                Expr::StrLit {
                    value: flags.to_string(),
                    span: span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span,
        })
    }

    // -------------------------------------------------------------------
    // Phase 6z — numeric literal lowering (float / hex / bin / oct / dec)
    // -------------------------------------------------------------------

    /// Lower a Ruby numeric literal token (Phase 6z).
    ///
    /// The lexer's Phase-4k / Phase-4l post-passes fuse the source-level
    /// shapes below into a single `TokenType::Number` token whose value
    /// is the verbatim source text (with underscore separators preserved).
    /// This routine dispatches on the shape:
    ///
    /// | Source       | SIR shape                                |
    /// |--------------|------------------------------------------|
    /// | `42`         | `IntLit { value: 42 }`                   |
    /// | `1_000_000`  | `IntLit { value: 1000000 }`              |
    /// | `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
    /// | `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
    /// | `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
    /// | `0o17`       | `IntLit { value: 15 }` (radix 8)         |
    /// | `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
    /// | `1.5`        | `FloatLit { value: 1.5 }`                |
    /// | `1e10`       | `FloatLit { value: 1e10 }`               |
    /// | `1.5e-3`     | `FloatLit { value: 0.0015 }`             |
    ///
    /// Float detection is a single pass over the cleaned (underscore-
    /// stripped) value: if `.` or `e` / `E` is present **anywhere**,
    /// the literal is a float; otherwise it's an integer.  Radix
    /// detection requires both the leading `0` *and* a radix-prefix
    /// letter as the second character.  These two checks are mutually
    /// exclusive in the Ruby grammar (radix prefixes start with a
    /// letter, floats start with a digit run + `.` / `e`), so the
    /// dispatch order doesn't matter — we test radix first because
    /// it's the cheaper check.
    ///
    /// ## v0 deferred
    ///
    /// - Ruby's `r` / `i` numeric suffixes (Rational / Complex, lexed
    ///   by Phase 4f) are still kept on the token as a trailing letter;
    ///   the lowerer currently rejects those, since SIR has no
    ///   Rational / Complex types.  A future phase will route those
    ///   into `BuiltinCall("rational", [...])` / `BuiltinCall("complex", [...])`
    ///   markers.
    /// - Negative literals are still handled by the unary-minus path
    ///   (Phase 6k); this routine sees only the magnitude.
    fn lower_numeric_literal(
        &mut self,
        raw: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Result<Expr, RubyLowerError> {
        // Step 1: strip Ruby's `_` digit separators.  They're purely
        // cosmetic (Ruby allows `1_000_000` to mean `1000000`).  We
        // do this *before* the shape dispatch so both the float-parse
        // and the radix-parse see clean digit strings.
        let cleaned: String = raw.chars().filter(|c| *c != '_').collect();

        // Step 2: radix-prefix detection (Phase 4l).  A Ruby radix
        // literal is `0` followed by a radix letter then the digits:
        //   0x | 0X  -> base 16
        //   0b | 0B  -> base  2
        //   0o | 0O  -> base  8
        //   0d | 0D  -> base 10 (explicit decimal)
        // Anything else starting with `0` is plain decimal (e.g. `0`,
        // `017` would be Ruby's legacy octal — not supported in v0).
        let bytes = cleaned.as_bytes();
        if bytes.len() >= 3 && bytes[0] == b'0' {
            let (radix, body_start): (u32, usize) = match bytes[1] {
                b'x' | b'X' => (16, 2),
                b'b' | b'B' => (2, 2),
                b'o' | b'O' => (8, 2),
                b'd' | b'D' => (10, 2),
                _ => (0, 0),
            };
            if radix != 0 {
                let body = &cleaned[body_start..];
                // `i64::from_str_radix` rejects empty strings and bad
                // digits — both of which would already be lexer bugs
                // here, so propagate the error rather than panicking.
                let v = i64::from_str_radix(body, radix).map_err(|_| RubyLowerError {
                    message: format!("invalid radix-{} integer literal `{}`", radix, raw),
                    line: err_line,
                    column: err_column,
                })?;
                return Ok(Expr::IntLit { value: v, span });
            }
        }

        // Step 3: float detection (Phase 4k).  A Ruby float literal has
        // either a fractional part (`.` followed by digit) OR an
        // exponent (`e` / `E`).  Both can appear together (`1.5e-3`).
        // We use `contains` rather than `starts_with` because the dot
        // / exponent can appear anywhere in the body.
        //
        // Note we cannot use a bare `.` check because the lexer's
        // float fusion already guarantees the dot is between digits;
        // we don't need to re-validate that here.
        let has_fraction = cleaned.contains('.');
        let has_exponent = cleaned.contains(['e', 'E']);
        if has_fraction || has_exponent {
            self.features_used.insert(Feature::Floats);
            let v: f64 = cleaned.parse().map_err(|_| RubyLowerError {
                message: format!("invalid float literal `{}`", raw),
                line: err_line,
                column: err_column,
            })?;
            return Ok(Expr::FloatLit { value: v, span });
        }

        // Step 4: plain decimal integer (pre-Phase-6z behaviour).
        let v: i64 = cleaned.parse().map_err(|_| RubyLowerError {
            message: format!("invalid integer literal `{}`", raw),
            line: err_line,
            column: err_column,
        })?;
        Ok(Expr::IntLit { value: v, span })
    }

    // -------------------------------------------------------------------
    // Phase 6y — string interpolation lowering
    // -------------------------------------------------------------------

    /// Lower a Ruby string literal whose raw content may contain
    /// `#{...}` interpolation markers (Phase 6y).
    ///
    /// The lexer's Phase-3b state machine captures `"foo#{x}bar"` as a
    /// single `TokenType::String` token whose `value` is the inner
    /// content with the `#{...}` markers preserved verbatim and any
    /// `{` / `}` inside the interpolation already brace-balanced by
    /// the lexer's `interp_brace_depth` tracking.
    ///
    /// ## Split strategy
    ///
    /// Walk the content char-by-char.  When we hit `#{`, flush the
    /// accumulated literal text as a `StrLit` segment, then scan the
    /// interpolation body up to the matching `}` (tracking brace depth
    /// so `#{ {a: 1} }` works).  Each interpolation body lowers via
    /// [`lower_interp_expression`] — bare identifiers route to
    /// `VarRef`, anything else lowers as a `BuiltinCall("__interp__",
    /// [StrLit(raw)])` marker.  This matches the marker pattern used
    /// by Phase 6v rescue/ensure.
    ///
    /// ## Output shapes
    ///
    /// | Source              | Lowered SIR shape                                                              |
    /// |---------------------|--------------------------------------------------------------------------------|
    /// | `"plain"`           | `StrLit("plain")`                                                              |
    /// | `"#{x}"`            | `VarRef("x")` — single non-literal segment, no wrapper                         |
    /// | `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                |
    /// | `"#{a}#{b}"`        | `BuiltinCall("string_concat", [VarRef("a"), VarRef("b")])`                     |
    /// | `"sum is #{1+2}"`   | `BuiltinCall("string_concat", [StrLit("sum is "), BuiltinCall("__interp__", [StrLit("1+2")])])` |
    ///
    /// ## v0 deferred
    ///
    /// - Complex interpolation expressions are kept as the `__interp__`
    ///   marker carrying the raw source text rather than being recursively
    ///   parsed; downstream Ruby emitters can still reconstruct the
    ///   original literal verbatim from the marker.  A future phase
    ///   will recursively invoke the Ruby parser/lowerer on the body so
    ///   the SIR carries proper semantic info.
    /// - Escape sequences inside the literal (`\n`, `\t`, `\\`, `\"`)
    ///   pass through unchanged — the lexer hasn't unescaped them yet.
    fn lower_string_literal_with_interp(
        &mut self,
        raw: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Result<Expr, RubyLowerError> {
        let mut segments: Vec<Expr> = Vec::new();
        let mut text_buf = String::new();
        let mut chars = raw.char_indices().peekable();

        while let Some((_, ch)) = chars.next() {
            // Detect the `#{` interpolation opener — only when `#` is
            // immediately followed by `{`.  Bare `#` inside a string
            // (e.g. `"a#b"`) is just a literal character.
            if ch == '#' {
                if let Some(&(_, '{')) = chars.peek() {
                    // Consume the `{`.
                    chars.next();
                    // Flush whatever literal text we've accumulated so
                    // far as its own `StrLit` segment.  We do not push
                    // empty segments (saves allocations and keeps the
                    // emitted SIR clean for `"#{a}"`-style strings).
                    if !text_buf.is_empty() {
                        segments.push(Expr::StrLit {
                            value: std::mem::take(&mut text_buf),
                            span: span.clone(),
                        });
                    }
                    // Scan up to the matching closing `}`, tracking
                    // brace depth so nested `{...}` (e.g. inline hash
                    // or block in the interp) is balanced correctly.
                    let mut depth: usize = 1;
                    let mut interp = String::new();
                    let mut terminated = false;
                    for (_, c) in chars.by_ref() {
                        match c {
                            '{' => {
                                depth += 1;
                                interp.push(c);
                            }
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    terminated = true;
                                    break;
                                }
                                interp.push(c);
                            }
                            other => interp.push(other),
                        }
                    }
                    if !terminated {
                        // Defensive: the lexer's Phase-3b state machine
                        // would have rejected an unterminated `#{...`,
                        // but propagate as a lower-error rather than
                        // panicking if it ever slips through.
                        return Err(RubyLowerError {
                            message: format!(
                                "unterminated `#{{...` interpolation in string literal `\"{}\"`",
                                raw
                            ),
                            line: err_line,
                            column: err_column,
                        });
                    }
                    segments.push(self.lower_interp_expression(&interp, span.clone()));
                    continue;
                }
            }
            // Ordinary literal character.  push() copies one full
            // UTF-8 char (not a byte) so multi-byte content stays
            // intact.
            text_buf.push(ch);
        }
        // Flush any trailing literal text after the last interp.
        if !text_buf.is_empty() {
            segments.push(Expr::StrLit {
                value: text_buf,
                span: span.clone(),
            });
        }

        // Result-shape selection:
        // - Empty string literal (`""`): emit a single empty `StrLit`.
        // - Exactly one segment: hand it back directly (no concat
        //   wrapper needed — keeps `"plain"` and `"#{x}"` lean).
        // - Two or more segments: wrap in a first-class `StrConcat`
        //   node (Phase 20b).  This replaces the v0
        //   `BuiltinCall("string_concat", …)` marker so backends can
        //   emit native string building and the validator can track
        //   interpolation usage via `Feature::StringInterpolation`.
        //
        // Any path that emits one or more segments needs the `Strings`
        // feature flag because we're producing `StrLit` data.
        if segments.is_empty() {
            return Ok(Expr::StrLit {
                value: String::new(),
                span,
            });
        }
        self.features_used.insert(Feature::Strings);
        if segments.len() == 1 {
            return Ok(segments.into_iter().next().unwrap());
        }
        // 2+ segments → first-class concatenation node.  Declare the
        // new feature so the manifest matches what the validator
        // observes for the `StrConcat` node.
        self.features_used.insert(Feature::StringInterpolation);
        Ok(Expr::StrConcat {
            parts: segments,
            span,
        })
    }

    /// Lower the body of a single `#{...}` interpolation segment
    /// (Phase 6y).
    ///
    /// v0 fast path: a bare identifier (no whitespace, no operators,
    /// no sigils) routes to `VarRef` with the same `Scope::Param` /
    /// `Scope::Local` dispatch as the regular factor-atom Name case.
    /// This covers the overwhelmingly common shape `"hello #{name}"`.
    ///
    /// v0 fallback: anything else — arithmetic, method calls, nested
    /// strings, sigil vars, etc. — lowers as a single marker
    /// `BuiltinCall("__interp__", [StrLit(raw_body)])`.  Downstream
    /// emitters that target Ruby can re-emit the marker as `#{<raw>}`
    /// verbatim; emitters that target other languages can flag the
    /// marker as a TODO for a future phase that re-parses the body.
    ///
    /// Same marker pattern as Phase 6v's `__rescue_marker__` /
    /// `__ensure_marker__` — a known-name `BuiltinCall` whose arg
    /// list carries the verbatim source text.
    fn lower_interp_expression(&mut self, raw: &str, span: Span) -> Expr {
        let trimmed = raw.trim();
        // Bare-identifier check: starts with `_` or ASCII letter,
        // and every following char is `_`/letter/digit.  We
        // intentionally reject sigil vars (`@x`, `$x`, `@@x`) here
        // because the Phase 6x routing happens at lex time, not at
        // interp-split time — those would need their own special
        // handling in a follow-up phase.
        let mut chars_iter = trimmed.chars();
        let is_bare_name = match chars_iter.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                chars_iter.all(|c| c.is_ascii_alphanumeric() || c == '_')
            }
            _ => false,
        };
        if is_bare_name {
            let scope = if self.current_params.contains(&trimmed.to_string()) {
                Scope::Param
            } else {
                Scope::Local
            };
            return Expr::VarRef {
                name: trimmed.to_string(),
                scope,
                span,
            };
        }
        // Phase 20a (FC) — recursively parse+lower the interpolation
        // body so `#{1 + 2}`, `#{foo.bar}`, `#{a * b}` etc. become real
        // SIR (a BuiltinCall/DirectCall/BinaryOp tree) instead of an
        // opaque marker.  We re-invoke the Ruby parser on the trimmed
        // body and lower its single tail expression in the CURRENT
        // scope (so VarRefs to enclosing params/locals resolve
        // correctly).  Anything that doesn't cleanly parse to exactly
        // one expression/method-call statement falls back to the
        // `__interp__` marker below (kept for one phase per the Tier-3
        // marker-replacement convention).
        if let Some(expr) = self.try_lower_interp_body(trimmed) {
            return expr;
        }
        // Fallback marker.  Triggers `Strings` because we embed the
        // raw text as a `StrLit`.
        self.features_used.insert(Feature::Strings);
        Expr::BuiltinCall {
            name: "__interp__".to_string(),
            args: vec![Expr::StrLit {
                value: raw.to_string(),
                span: span.clone(),
            }],
            effects: EffectSet::PURE,
            span,
        }
    }

    /// Phase 20a (FC) — try to lower an interpolation body by
    /// re-invoking the Ruby parser on it and lowering the resulting
    /// single tail expression.  Returns `None` (→ marker fallback) if
    /// the body is empty, doesn't parse to exactly one statement, or
    /// that statement isn't a plain expression / method call.  Lowering
    /// runs in the current scope so `#{x + 1}` resolves `x` the same way
    /// the surrounding code would.
    fn try_lower_interp_body(&mut self, body: &str) -> Option<Expr> {
        if body.is_empty() {
            return None;
        }
        // DoS guard (Phase 20a): a nested interpolated literal
        // (`"#{ "#{x}" }"`) recurses back through this path on every
        // level.  Beyond MAX_INTERP_DEPTH we stop re-parsing and let the
        // caller fall back to the `__interp__` marker, bounding stack
        // growth no matter how deeply an adversarial input nests.
        if self.interp_depth >= MAX_INTERP_DEPTH {
            return None;
        }
        self.interp_depth += 1;
        // Run the parse+lower in a closure so a single `?`/early-return
        // can't skip the depth decrement below.
        let result = (|| {
            let ast = coding_adventures_ruby_parser::parse_ruby(body);
            if ast.rule_name != "program" {
                return None;
            }
            let stmts: Vec<&GrammarASTNode> = ast
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                    _ => None,
                })
                .collect();
            // Exactly one statement; multi-statement bodies (`#{a; b}`)
            // are rare and kept as the verbatim marker.
            if stmts.len() != 1 {
                return None;
            }
            let inner = self.first_node_child(stmts[0])?;
            match inner.rule_name.as_str() {
                "expression_stmt" => {
                    let expr_node = self.first_node_child(inner)?;
                    self.lower_expression(expr_node).ok()
                }
                "method_call" | "method_call_no_paren" => self.lower_method_call(inner).ok(),
                _ => None,
            }
        })();
        self.interp_depth -= 1;
        result
    }

    /// Phase 23b (FC) — lower a `defined_expression` node
    /// (`defined? <operand>` / `defined?(<operand>)`) to
    /// `BuiltinCall("defined?", [operand])`.
    ///
    /// Effects are `PURE`: `defined?` inspects whether its operand is
    /// defined and never raises (even on an undefined name) and has no
    /// side effects.  The operand is carried as a lowered argument so a
    /// downstream emitter can reconstruct the source; a faithful backend
    /// does NOT evaluate it (Ruby's `defined?(foo.bar)` does not call
    /// `foo.bar`).
    ///
    /// v0 limitation: the operand is lowered like any expression, so
    /// `defined?(undefined_local)` lowers to a `VarRef` the SIR validator
    /// will reject as an unknown name — `defined?` on a never-bound bare
    /// local is not representable yet.  In practice the operand is a
    /// bound name, a method call, or a literal.
    fn lower_defined_expression(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        let operand_node = self.first_node_child(node).ok_or_else(|| RubyLowerError {
            message: "defined_expression missing operand".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let operand = self.lower_expression(operand_node)?;
        Ok(Expr::BuiltinCall {
            name: "defined?".to_string(),
            args: vec![operand],
            effects: EffectSet::PURE,
            span: self.span_of(node),
        })
    }

    fn lower_factor(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // factor ::= ( atom ) { dot_call }
        //
        // Phase 6l — method receiver chains.  The atom is followed by
        // zero or more `dot_call` Node children (`.method[(args)]`).
        // We extract the atom first, then wrap it once per dot_call.
        //
        // Atom alternatives: NUMBER | STRING | NAME | KEYWORD |
        //   symbol_literal | array_literal | hash_literal |
        //   LPAREN expression RPAREN | unary_minus
        let atom = self.lower_factor_atom(node)?;
        self.apply_dot_chain(atom, node)
    }

    /// Extract the atom expression from a `factor` node, ignoring
    /// trailing `dot_call` Node children.  This is the pre-Phase-6l
    /// lowering logic, refactored into its own helper so that
    /// `lower_factor` can apply the dot-chain postfix on top.
    fn lower_factor_atom(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let span = self.span_of_token(tok);
                    match tok.type_ {
                        TokenType::Number => {
                            // Phase 6z — float / hex / bin / oct / decimal-explicit
                            // integer literal parsing.  The lexer (Phase 4k / 4l)
                            // fuses these into a single `Number` token whose
                            // value carries the verbatim source text.  The
                            // parser sees them all uniformly at the factor
                            // atom position — no grammar changes needed.
                            return self
                                .lower_numeric_literal(&tok.value, span, tok.line, tok.column);
                        }
                        TokenType::String => {
                            // Phase 7a — backtick command literal dispatch.
                            // The lexer (Phase 4m) emits `` `cmd args` `` as
                            // a `String` token whose value is the verbatim
                            // source *including* the surrounding backticks
                            // — same lexeme-prefix sentinel trick the
                            // percent literals and heredocs use.  Detect by
                            // checking the leading byte.
                            if tok.value.starts_with('`') {
                                return Ok(self.lower_backtick_command_literal(&tok.value, span));
                            }
                            // Phase 7b — heredoc dispatch.  The lexer (Phase
                            // 3c / 4o) finalises every heredoc into a single
                            // `String` token whose value is the verbatim
                            // source: opener line (`<<TAG`, `<<-TAG`, or
                            // `<<~TAG`), a `\n`, the body, then the closing
                            // tag.  We detect by the `<<` prefix.
                            if tok.value.starts_with("<<") {
                                return Ok(self.lower_heredoc_literal(
                                    &tok.value, span, tok.line, tok.column,
                                ));
                            }
                            // Phase 19d (FC) — `%r{...}` regex literal
                            // dispatch.  The lexer emits `%r{pat}flags` as
                            // a `String` token carrying the verbatim source
                            // (the `%r`, delimiters, and flags all present).
                            // We split it and reuse `lower_regex_literal`,
                            // so interpolation inside `%r{...}` is handled
                            // for free by the shared pattern splitter.
                            if let Some((pattern, flags)) = percent_r_pattern_flags(&tok.value) {
                                let (pattern, flags) = (pattern.to_string(), flags.to_string());
                                return self.lower_regex_literal(
                                    &pattern, &flags, span, tok.line, tok.column,
                                );
                            }
                            // Phase 19a (FC) — regex literal dispatch.  The
                            // lexer's `regex_body` machine emits `/p/flags`
                            // as a `String` token carrying the verbatim
                            // source (slashes included).  `regex_pattern_flags`
                            // recognises that shape (and rejects path-shaped
                            // strings like `/usr/bin` via the flag-letter
                            // check), splitting it into pattern + flags.
                            if let Some((pattern, flags)) = regex_pattern_flags(&tok.value) {
                                let (pattern, flags) = (pattern.to_string(), flags.to_string());
                                return self.lower_regex_literal(
                                    &pattern, &flags, span, tok.line, tok.column,
                                );
                            }
                            // Phase 6y — string interpolation expression
                            // lowering.  The lexer (Phase 3b) emits the
                            // entire `"foo#{x}bar"` literal as a single
                            // `String` token whose `value` holds the inner
                            // content with `#{...}` markers preserved
                            // verbatim.  When markers are present we split
                            // into segments and emit a concat builtin;
                            // when absent we fall through to a plain
                            // `StrLit` (zero-cost fast path).
                            return self.lower_string_literal_with_interp(
                                &tok.value, span, tok.line, tok.column,
                            );
                        }
                        TokenType::Name => {
                            // Phase 16d (FC) — a bare `raise` (no args:
                            // re-raise the current exception) lowers to
                            // the same `BuiltinCall("raise")` as
                            // `raise Foo` — `MayThrow` + `Divergent` —
                            // rather than a plain local read, and
                            // requests `Feature::Exceptions`.  A `raise`
                            // shadowed by a local binding (`raise = 1`)
                            // keeps the local.
                            if tok.value == "raise" && !self.declared_locals.contains("raise") {
                                self.features_used.insert(Feature::Exceptions);
                                return Ok(Expr::BuiltinCall {
                                    name: "raise".to_string(),
                                    args: vec![],
                                    effects: EffectSet::PURE
                                        .with(Effect::MayThrow)
                                        .with(Effect::Divergent),
                                    span,
                                });
                            }
                            // Phase 23a (FC) — `__FILE__` is Ruby's pseudo-
                            // variable for the path of the current source
                            // file.  The lexer does NOT classify it as a
                            // keyword (it starts with `_`, so it arrives as
                            // an ordinary `Name` token) and the grammar
                            // already matches it via `factor`'s bare-NAME
                            // alternative — so NO grammar/lexer change is
                            // needed; we intercept it here at lowering time,
                            // exactly like the bare-`raise` case above.
                            //
                            // It lowers to a compile-time `StrLit` carrying
                            // the lowerer's `file_name` (the SIR module
                            // identifier the source was compiled under) —
                            // the closest fixed value we have for "the
                            // current file" without a runtime filesystem.
                            // Emitting a `StrLit` means the module uses the
                            // `strings` feature, so we declare it (already
                            // permitted by the manifest builder allowlist).
                            //
                            // A `__FILE__` shadowed by a local binding
                            // (`__FILE__ = 1`) keeps the local, mirroring
                            // the `raise` shadow guard.
                            if tok.value == "__FILE__" && !self.declared_locals.contains("__FILE__")
                            {
                                self.features_used.insert(Feature::Strings);
                                return Ok(Expr::StrLit {
                                    value: self.file_name.clone(),
                                    span,
                                });
                            }
                            // Phase 23c (FC) — `__LINE__` is Ruby's pseudo-
                            // variable for the (1-based) line number of the
                            // source line on which it appears.  Like
                            // `__FILE__` it is NOT a lexer keyword (it
                            // arrives as an ordinary `Name` token) and the
                            // grammar already matches it via `factor`'s
                            // bare-NAME alternative — so NO grammar/lexer
                            // change is needed; we intercept it here at
                            // lowering time, exactly like `__FILE__` /
                            // bare-`raise`.
                            //
                            // It lowers to a compile-time `IntLit` carrying
                            // the token's own line number (`tok.line`).
                            // Integers are a baseline SIR capability, so —
                            // unlike `__FILE__`'s StrLit — no `Feature`
                            // declaration is required.
                            //
                            // A `__LINE__` shadowed by a local binding
                            // (`__LINE__ = 1`) keeps the local, mirroring
                            // the `__FILE__` / `raise` shadow guards.
                            if tok.value == "__LINE__" && !self.declared_locals.contains("__LINE__")
                            {
                                return Ok(Expr::IntLit {
                                    value: tok.line as i64,
                                    span,
                                });
                            }
                            // Phase 23d (FC) — `__dir__` is Ruby's
                            // pseudo-variable for the directory name of the
                            // current source file (`File.dirname(__FILE__)`,
                            // expanded).  Like `__FILE__` / `__LINE__` it is
                            // NOT a lexer keyword (it arrives as an ordinary
                            // `Name` token) and the grammar already matches
                            // it via `factor`'s bare-NAME alternative — so NO
                            // grammar/lexer change is needed; we intercept it
                            // here at lowering time, exactly like the sibling
                            // pseudo-variables.
                            //
                            // It lowers to a compile-time `StrLit` carrying
                            // the directory portion of the lowerer's
                            // `file_name`: the substring before the final
                            // path separator (`/` or `\`), or `"."` when the
                            // name has no directory component (the closest
                            // fixed value for "the current directory" without
                            // a runtime filesystem — consistent with how
                            // `__FILE__` surfaces the bare module name).
                            // Emitting a `StrLit` means the module uses the
                            // `strings` feature, so we declare it.
                            //
                            // A `__dir__` shadowed by a local binding
                            // (`__dir__ = 1`) keeps the local, mirroring the
                            // sibling shadow guards.  Scope: the bare form;
                            // the explicit-call form `__dir__()` is a
                            // deliberate follow-up slice.
                            if tok.value == "__dir__" && !self.declared_locals.contains("__dir__") {
                                let dir = match self.file_name.rfind(['/', '\\']) {
                                    Some(i) => self.file_name[..i].to_string(),
                                    None => ".".to_string(),
                                };
                                self.features_used.insert(Feature::Strings);
                                return Ok(Expr::StrLit { value: dir, span });
                            }
                            // Inside a function body, parameter
                            // names lex as `VarRef` with
                            // `Scope::Param` so the SIR validator
                            // can verify they bind to a `Param`
                            // declaration.  At the top level
                            // (main) the params set is empty and
                            // every name falls through to
                            // `Scope::Local`.
                            //
                            // Phase 6x — Ruby sigil-prefixed variable refs
                            // (`@x` ivar, `@@x` cvar, `$x` gvar) come through
                            // as Name-typed tokens with the sigil preserved
                            // in `value` (the lexer's Phase-4i/4j states
                            // build a single-token form).
                            //
                            // v0 SIR limitation: there is no dedicated IVar /
                            // CVar / GVar scope.  Using `Scope::Global` for
                            // `$x` would require a matching `Global` decl on
                            // the module (the validator enforces this); we
                            // skip the auto-declaration and put all sigil
                            // vars on `Scope::Local` instead.  The leading
                            // sigil stays in the bound name, so downstream
                            // emitters that target Ruby (or any language
                            // with similar lookup) can detect the sigil and
                            // route the assignment / read appropriately.
                            //
                            // Documented as a deferred limitation; a follow-
                            // up phase will (a) add IVar/CVar scopes to SIR
                            // and/or (b) auto-emit `Global` declarations for
                            // `$x`-prefixed names so the validator-true
                            // mapping `$x` → `Scope::Global` becomes usable.
                            // Phase 15a/15b/15c (FC) — names lower to
                            // dedicated scopes (no declaration needed):
                            // `@@x` (double `@`) → class var, `@x`
                            // (single `@`) → instance var, and any
                            // uppercase-initial name (`FOO`, `MyClass`)
                            // → constant.  `$x` (global) keeps the
                            // pre-15a `Scope::Local` fallback for now.
                            // Class var is checked first since `@@x`
                            // also starts with `@`.
                            let scope = if is_class_var_name(&tok.value) {
                                self.features_used.insert(Feature::ClassVars);
                                Scope::ClassVar
                            } else if is_instance_var_name(&tok.value) {
                                self.features_used.insert(Feature::InstanceVars);
                                Scope::Instance
                            } else if is_constant_name(&tok.value) {
                                self.features_used.insert(Feature::Constants);
                                Scope::Const
                            } else if self.current_params.contains(&tok.value) {
                                Scope::Param
                            } else {
                                Scope::Local
                            };
                            return Ok(Expr::VarRef {
                                name: tok.value.clone(),
                                scope,
                                span,
                            });
                        }
                        TokenType::Keyword => match tok.value.as_str() {
                            "nil" => return Ok(Expr::NilLit { span }),
                            "true" => return Ok(Expr::BoolLit { value: true, span }),
                            "false" => return Ok(Expr::BoolLit { value: false, span }),
                            // O2 (OOP production) — a bare `self` is the current
                            // receiver.  The Ruby frontend hoists methods to
                            // detached top-level functions (no `self` parameter),
                            // so `self` cannot be a plain local: it must ask the
                            // OOP runtime for the receiver on top of its self-stack.
                            // Lower to `__self__()`, which the backend routes to
                            // `_sir_oop_current_self()`.  As a dot-chain receiver
                            // (`self.count`) this `__self__()` becomes the receiver
                            // arg of the enclosing `__method__` fold — so
                            // `self.foo` and a self-returning method (`c.inc.inc`,
                            // where `inc` ends in `self`) both work.
                            "self" => {
                                self.features_used.insert(Feature::Classes);
                                return Ok(Expr::BuiltinCall {
                                    name: "__self__".to_string(),
                                    args: Vec::new(),
                                    effects: EffectSet::PURE,
                                    span,
                                });
                            }
                            _ => {
                                // Any other keyword used in factor position
                                // is an error in v0 — but the parser
                                // accepts NAME|KEYWORD as a fallback to
                                // method-call shapes.  Treat as a local.
                                return Ok(Expr::VarRef {
                                    name: tok.value.clone(),
                                    scope: Scope::Local,
                                    span,
                                });
                            }
                        },
                        _ => {
                            // Parens — skip the LPAREN/RPAREN tokens
                            // and recurse into the inner expression
                            // (which is a sibling Node child).
                        }
                    }
                }
                ASTNodeOrToken::Node(sub) => {
                    // Skip dot_call children — those are postfix-applied
                    // by `apply_dot_chain` after the atom is extracted.
                    if sub.rule_name == "dot_call" {
                        continue;
                    }
                    return self.lower_expression(sub);
                }
            }
        }
        Err(RubyLowerError {
            message: "factor node had no recognisable leaf".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6l — dot-call chain postfix
    // -------------------------------------------------------------------

    /// Walk every `dot_call` Node child of `node` (in source order) and
    /// fold each one into a method-call expression with the running
    /// `recv` as receiver.  `foo.bar.baz` becomes:
    ///
    /// ```text
    /// __method__(recv = foo, "bar")    →  inner
    /// __method__(recv = inner, "baz")  →  outer
    /// ```
    ///
    /// The chosen SIR encoding is `Expr::BuiltinCall { name:
    /// "__method__", args: [receiver, StrLit(method_name), ...args] }`.
    /// This keeps the receiver as a first-class expression (preserving
    /// arbitrary nesting), the method name as data (so backends can
    /// dispatch by string), and avoids growing the shared SIR Expr enum.
    ///
    /// BuiltinCall (not DirectCall) is chosen because the validator
    /// checks DirectCall.fn_name against the module's function table,
    /// and our synthetic `__method__` envelope intentionally isn't a
    /// declared function — it's a wire-format tag for backends.
    ///
    /// Effects default to PURE — receiver-dispatched calls are
    /// type-erased at this layer; a later receiver-type analysis pass
    /// can widen as needed.  Callers wrapping I/O-flavored chains
    /// (e.g. `STDOUT.puts(...)`) can post-process.
    fn apply_dot_chain(
        &mut self,
        atom: Expr,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let mut recv = atom;
        for child in &node.children {
            if let ASTNodeOrToken::Node(sub) = child {
                if sub.rule_name == "dot_call" {
                    recv = self.fold_one_dot_call(recv, sub)?;
                } else if sub.rule_name == "scope_resolution" {
                    // Phase 15d (FC) — `::Name` step.
                    recv = self.fold_one_scope_resolution(recv, sub)?;
                }
            }
        }
        Ok(recv)
    }

    /// Lower a single `scope_resolution` step (`::Name`).  Grammar shape
    /// (Phase 15d):
    ///     scope_resolution = "::" ( NAME | KEYWORD ) ;
    ///
    /// A scoped constant lookup `Foo::Bar` is, semantically, a single
    /// constant resolved against a namespace.  The common case — a
    /// constant path whose base is itself a `Scope::Const` ref — folds
    /// into one qualified-name `Scope::Const` ref (`"Foo::Bar"`), so
    /// `A::B::C` collapses to `VarRef { scope: Const, name: "A::B::C" }`.
    /// A non-constant base (`expr::Bar`, uncommon) is preserved
    /// structurally via a `__scope__` BuiltinCall marker so no structure
    /// is silently dropped.  Either way `Feature::Constants` is
    /// requested (the result is a constant reference).
    fn fold_one_scope_resolution(
        &mut self,
        base: Expr,
        sr_node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let (rhs_name, rhs_span) = self.expect_first_name_token(sr_node)?;
        self.features_used.insert(Feature::Constants);
        match base {
            Expr::VarRef {
                name,
                scope: Scope::Const,
                span,
            } => Ok(Expr::VarRef {
                name: format!("{name}::{rhs_name}"),
                scope: Scope::Const,
                span,
            }),
            other => {
                // Fallback: `expr::Bar` where the base is not a bare
                // constant.  Keep the step explicit; the synthetic
                // StrLit triggers the Strings feature.
                self.features_used.insert(Feature::Strings);
                let span = self.span_of(sr_node);
                Ok(Expr::BuiltinCall {
                    name: "__scope__".to_string(),
                    args: vec![
                        other,
                        Expr::StrLit {
                            value: rhs_name,
                            span: rhs_span,
                        },
                    ],
                    effects: EffectSet::PURE,
                    span,
                })
            }
        }
    }

    /// Lower a single `dot_call` step.  Grammar shape (Phase 6s):
    ///     dot_call = "." ( NAME | KEYWORD ) [ LPAREN [ call_arg
    ///                  { COMMA call_arg } ] RPAREN ] ;
    fn fold_one_dot_call(
        &mut self,
        receiver: Expr,
        dot_node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // First Name/Keyword token under dot_node is the method name.
        let (method_name, name_span) = self.expect_first_name_token(dot_node)?;
        // Optional argument list — each arg is wrapped in `call_arg`
        // (Phase 6s) so the optional splat prefix has a slot.
        let args: Vec<Expr> = dot_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_call_arg(n))
            .collect::<Result<Vec<_>, _>>()?;

        let span = self.span_of(dot_node);

        // O2 (OOP production) — `Foo.new(args)` on a *constant* receiver is
        // object construction, not an ordinary method call.  It lowers to the
        // OOP-runtime builtin `__new__(StrLit("Foo"), …args)` (→
        // `_sir_oop_call_new`), which allocates an instance, pushes it as the
        // current self, runs the inherited `initialize` with `args`, pops self,
        // and returns the object.  We special-case it *here*, before the generic
        // `__method__` envelope, and only when the receiver is a bare constant
        // ref (`Scope::Const`) — so `arr.new`/`obj.new` (a `.new` on a
        // non-constant, which Ruby would dispatch normally) is untouched.
        //
        // Chaining falls out for free: in `Foo.new(x).meth`, `apply_dot_chain`
        // folds `.new` first (this arm → `__new__("Foo", x)`) and then folds
        // `.meth` with that `__new__` call as the receiver of the outer
        // `__method__`.  So `__method__(__new__("Foo", x), "meth")` is produced
        // exactly as required, and a longer chain (`c.inc.inc`) nests the same
        // way.
        if method_name == "new" {
            if let Expr::VarRef {
                name: class_name,
                scope: Scope::Const,
                ..
            } = &receiver
            {
                self.features_used.insert(Feature::Classes);
                self.features_used.insert(Feature::Strings);
                let mut new_args: Vec<Expr> = Vec::with_capacity(args.len() + 1);
                new_args.push(Expr::StrLit {
                    value: class_name.clone(),
                    span: name_span,
                });
                new_args.extend(args);
                return Ok(Expr::BuiltinCall {
                    name: "__new__".to_string(),
                    args: new_args,
                    effects: EffectSet::PURE,
                    span,
                });
            }
        }

        // Issue #59 — a NON-`new` method call on a *constant* receiver
        // (`Counter.zero`, `Foo.bar(x)`) is a CLASS-METHOD dispatch.  It lowers
        // to `__class_method__(StrLit("Counter"), StrLit("zero"), …args)`,
        // which the backend routes to the OOP runtime's `call_class_method`
        // (ancestry-walking lookup in the `def self.m` table registered by
        // `__def_class_method__`).  We special-case it here, before the generic
        // `__method__` envelope, and ONLY when the receiver is a bare constant
        // ref — so an instance method call on a non-constant (`obj.meth`,
        // `arr.zero`) still routes through `__method__` unchanged.  `.new` was
        // handled just above (it routes to `__new__`, the implicit constructor
        // class method), so it never reaches here.  Chaining is preserved: in
        // `Foo.bar.baz`, this folds `.bar` into a `__class_method__` call and
        // `apply_dot_chain` then folds `.baz` with that call as the receiver of
        // an outer `__method__` — exactly the nesting `.new(...).meth` uses.
        if let Expr::VarRef {
            name: class_name,
            scope: Scope::Const,
            ..
        } = &receiver
        {
            self.features_used.insert(Feature::Classes);
            self.features_used.insert(Feature::Strings);
            let mut cm_args: Vec<Expr> = Vec::with_capacity(args.len() + 2);
            cm_args.push(Expr::StrLit {
                value: class_name.clone(),
                span: name_span.clone(),
            });
            cm_args.push(Expr::StrLit {
                value: method_name.clone(),
                span: name_span,
            });
            cm_args.extend(args);
            return Ok(Expr::BuiltinCall {
                name: "__class_method__".to_string(),
                args: cm_args,
                effects: EffectSet::PURE,
                span,
            });
        }

        // Pack as BuiltinCall("__method__", [receiver, StrLit(method),
        // ...args]) — see apply_dot_chain doc for rationale.
        // The synthetic StrLit triggers the Strings feature, which the
        // post-pass adds to the manifest unconditionally.
        self.features_used.insert(Feature::Strings);
        let mut full_args = Vec::with_capacity(args.len() + 3);
        full_args.push(receiver);
        full_args.push(Expr::StrLit {
            value: method_name,
            span: name_span,
        });
        full_args.extend(args);

        // RB1 (FC) — a trailing block on a receiver/dotted method call
        // (`recv.each { … }` / `recv.each do … end`).  The grammar now
        // admits an optional `block` after the dot_call's argument list;
        // hoist it to a top-level Function and append the resulting
        // `MakeClosure` as the call's trailing argument, exactly as
        // `method_with_block` does for bare-name calls.  Without this the
        // block would be silently dropped.  Captures cover the RB2
        // enclosing-block capture and, per M4, any enclosing locals/params
        // the block reads (shared with `method_with_block`).
        if let Some(block_node) = self.find_node_child(dot_node, "block") {
            let (fn_name, capture_values) = self.hoist_block_to_function(block_node)?;
            let bspan = self.span_of(block_node);
            full_args.push(Expr::MakeClosure {
                fn_name,
                captures: capture_values,
                span: bspan,
            });
            self.features_used.insert(Feature::Closures);
        }

        Ok(Expr::BuiltinCall {
            name: "__method__".to_string(),
            args: full_args,
            effects: EffectSet::PURE,
            span,
        })
    }

    // -------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------

    fn first_node_child<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        })
    }

    fn find_node_child<'a>(
        &self,
        node: &'a GrammarASTNode,
        rule_name: &str,
    ) -> Option<&'a GrammarASTNode> {
        node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == rule_name => Some(n),
            _ => None,
        })
    }

    /// Return the lexeme of the first `Name` or `Keyword` token
    /// directly under `node` along with its span.
    fn expect_first_name_token(
        &self,
        node: &GrammarASTNode,
    ) -> Result<(String, Span), RubyLowerError> {
        for child in &node.children {
            if let ASTNodeOrToken::Token(t) = child {
                if matches!(t.type_, TokenType::Name | TokenType::Keyword) {
                    return Ok((t.value.clone(), self.span_of_token(t)));
                }
            }
        }
        Err(RubyLowerError {
            message: format!(
                "expected first Name/Keyword token under `{}`",
                node.rule_name
            ),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }
}

// ---------------------------------------------------------------------------
// Token plumbing
// ---------------------------------------------------------------------------

fn token_type_name(t: TokenType) -> &'static str {
    // We only need names for the operator tokens that appear in
    // expression chains.  Anything else returns a placeholder; the
    // caller never compares it against the operator list.
    match t {
        TokenType::Plus => "PLUS",
        TokenType::Minus => "MINUS",
        TokenType::Star => "STAR",
        TokenType::Slash => "SLASH",
        _ => "OTHER",
    }
}

fn token_lexeme_for_op(t: TokenType) -> &'static str {
    match t {
        TokenType::Plus => "+",
        TokenType::Minus => "-",
        TokenType::Star => "*",
        TokenType::Slash => "/",
        _ => "?",
    }
}

// ---------------------------------------------------------------------------
// Ruby builtins
// ---------------------------------------------------------------------------

/// Effect set for a recognised Ruby builtin.  Returns `None` for any
/// name we don't know — the caller falls back to `DirectCall`.
///
/// The v0 list is intentionally tiny: just the I/O and error-raising
/// builtins that nearly every Ruby program touches.  Later phases
/// will grow this as the lowering matures.
fn ruby_builtin_effects(name: &str) -> Option<EffectSet> {
    match name {
        "puts" | "print" | "p" => Some(EffectSet::PURE.with(Effect::MayPrint)),
        "gets" => {
            // Reads from stdin — modelled as a blocking effect.  Not
            // strictly pure, but `MayBlock` is the closest tag we
            // have in the SIR v0 effect lattice.
            Some(EffectSet::PURE.with(Effect::MayBlock))
        }
        "raise" => {
            // `raise` is divergent (the call doesn't return) and
            // also throws — backends use both tags to suppress
            // unreachable-code warnings and to emit `throw`/`return`
            // shapes correctly.
            Some(
                EffectSet::PURE
                    .with(Effect::MayThrow)
                    .with(Effect::Divergent),
            )
        }
        // Phase 6g: block-taking iterators.  These all accept a
        // trailing block (closure) as their last argument and invoke
        // it zero or more times.  v0 models them as pure builtins —
        // their effect set is the *closure's* effect set lifted, but
        // SIR's effect inference handles that at the call site, so
        // we just declare PURE here.  Adding them to the builtin
        // table makes `each { … }` lower cleanly without forcing
        // every consumer to declare `each` as a user function.
        // Phase 6w — explicit closure-construction builtins.  `lambda { ... }`
        // and `proc { ... }` go through `method_with_block` and pass their
        // hoisted closure as the trailing arg.  Tagging both as known
        // builtins (PURE) gives downstream emitters a single
        // closure-construction shape — same as Phase 6w's arrow-lambda.
        "lambda" | "proc" => Some(EffectSet::PURE),
        "each" | "map" | "select" | "reject" | "filter" | "each_with_index"
        | "each_with_object" | "times" | "tap" | "then" | "yield_self" | "loop" | "collect"
        | "find" | "detect" | "any?" | "all?" | "none?" | "count" | "reduce" | "inject"
        | "sort_by" | "group_by" | "min_by" | "max_by" | "flat_map" | "partition"
        | "each_slice" | "each_cons" => Some(EffectSet::PURE),
        // Phase 26a (FC) — `using Mod` activates a refinement module in
        // the current lexical scope.  It is an ordinary method call
        // (`using` is a `Kernel`/`Module` method, not a keyword), so it
        // arrives here as a `method_call_no_paren` with callee `using`
        // and the refinement module as its sole argument.  Without this
        // entry it would fall through to a `DirectCall("using", …)`,
        // which the SIR validator rejects as an undeclared callee.
        // Tagging it a PURE builtin — like the other declaration-style
        // forms (`alias` / `undef`) — lets it lower and validate as a
        // first-class call whose argument (the module, e.g. a `Const`
        // ref) is lowered through the normal expression path.  In this
        // model the activation carries no runtime data effect.
        "using" => Some(EffectSet::PURE),
        // Phase 26b (FC) — `refine Class do ... end` reopens `Class`
        // within a refinement module, defining methods that are only
        // visible where the enclosing module is later `using`-activated.
        // `refine` is an ordinary `Module` method that takes the target
        // class as its argument and a block holding the refinement body,
        // so it arrives as a `method_with_block` with callee `refine`.
        // Without this entry it would fall through to a
        // `DirectCall("refine", …)`, which the SIR validator rejects as
        // an undeclared callee.  Tagging it a PURE builtin — like the
        // block-taking `lambda`/`proc` and the `using` companion form —
        // lets it lower and validate as a first-class call: the target
        // class is lowered through the normal expression path and the
        // refinement block is hoisted to a `MakeClosure` trailing arg by
        // `lower_method_with_block`.  This completes the Ruby 3.4
        // refinement surface (`using` + `refine`).
        "refine" => Some(EffectSet::PURE),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

impl RubyLowerError {
    /// Chain a value onto the error (used inside `?`-style fallbacks
    /// where we need to "consume" a value without otherwise using it).
    fn also<T>(self, _: T) -> Self {
        self
    }
}
