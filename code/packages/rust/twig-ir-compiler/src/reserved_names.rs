//! `reserved_names` — the identifier prefixes a Twig program may not use.
//!
//! ## Why a source language needs reserved names at all
//!
//! The IIR this crate emits does not mangle names: `(define (sq x) …)`
//! becomes an IIR function called `sq`, and on the native backends a
//! machine symbol called `sq`. The lowering passes that run *after* this
//! compiler add functions of their own to the same module, and recognise
//! them **by name**:
//!
//! | prefix      | owner                                   | examples                                          |
//! |-------------|-----------------------------------------|---------------------------------------------------|
//! | `__lisp_`   | `iir-builtin-lowering::lisp_semantics`  | `__lisp_equal`, `__lisp_exit_code`, and the inline-rule markers `__lisp_truthy` / `__lisp_not` |
//! | `__dyn_`    | the dynamic-value lowering and runtime  | `__dyn_list_length` …, `__dyn_call_closure`, and the C runtime's `__dyn_pair_new` … |
//! | `__lambda_` | this compiler                           | `__lambda_0`, `__lambda_1`, … — the functions a `(lambda …)` is lifted into |
//! | `__twig_`   | the native runtime (`twig_runtime.c`, gc-core-capi) | `__twig_print_i64`, `__twig_trap`, `__twig_gc_alloc` … |
//!
//! Twig's `NAME` token (`[A-Za-z+\-*/=<>!?_$][…]*`) can spell every one of
//! them, so before this check a program could stand in for the compiler:
//!
//! ```text
//! (define (__lisp_equal a b) 42) (equal? 1 2)   ; equal? called the program's function
//! (define (__lisp_not x) 0) (__lisp_not 5)      ; the call was expanded as Lisp's `not`
//! (define (__dyn_list_length l) 99) (length …)  ; `length` called the program's function
//! ```
//!
//! and a `__dyn_*` definition could collide with a symbol of the linked
//! runtime. The passes now refuse such a module themselves (`lower_dyn_repr`
//! returns a `ReservedNameError`), but a user deserves a source-level error
//! with a line and column, raised before any IIR exists — so the frontend
//! reserves both prefixes.
//!
//! The same review found two more names a program could take over:
//!
//! ```text
//! (define (__twig_print_i64 n) 0) (begin (print 5) 0)
//!     ; native x86-64 linked `print`'s runtime call to the program's
//!     ; function, so nothing was printed
//! (define (__lambda_0 y) (+ y 50)) (define (g f) (+ 0 (f 5)))
//! (+ (__lambda_0 1) (g (lambda (y) (+ y 1))))
//!     ; the lambda is lifted into a second function named `__lambda_0`;
//!     ; the closure called the program's one and the result was 120, not 57
//! ```
//!
//! so `__lambda_` and `__twig_` are reserved as well.
//!
//! ## Why reserve a prefix rather than pick an unspellable name
//!
//! The other fix for `__lambda_N` would be a generated name that contains a
//! character Twig's `NAME` token cannot (`NAME` is
//! `[A-Za-z+\-*/=<>!?_$][A-Za-z+\-*/=<>!?_$0-9]*`, so `.`, `:`, `#`, `@`,
//! `%` … are out). But that name reaches every backend's symbol table, and
//! no such character is legal unquoted on all of them: the JVM forbids `.`,
//! `;`, `[` and `/` in a method name; LLVM quotes anything outside
//! `[-a-zA-Z$._0-9]`, so `.` is the only candidate it takes bare; ilasm's
//! plain identifiers take `@` and `` ` `` but not `.` or `:` in a method name
//! without quoting. A reserved prefix changes no generated name, so no
//! backend can mangle it differently — it only removes the program's way of
//! spelling it.
//!
//! ## The rule
//!
//! Every `NAME` token in the program — a definition, a parameter, a `let`
//! binding, a reference, a record or union name, a module name — is
//! refused if any of its `/`-separated segments begins, ignoring ASCII case,
//! with a reserved prefix. Case is ignored because a record's accessors are
//! derived from its *lower-cased* name (`(record __DYN_X …)` would define
//! `__dyn_x-…`); segments are checked because module-qualified references
//! are written `module/name`.
//!
//! Quoted symbols (`'__lisp_x`, `(quote __lisp_x)`) are data, not names —
//! they become interned symbol constants and never name a function — so
//! they are allowed.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::errors::TwigCompileError;

/// The identifier prefixes reserved for compiler-generated functions and
/// runtime symbols (see the module docs for who owns each).
pub const RESERVED_PREFIXES: [&str; 4] = ["__lisp_", "__dyn_", "__lambda_", "__twig_"];

/// The reserved prefix `name` uses, if any.
///
/// ```text
/// reserved_prefix("__lisp_equal")     → Some("__lisp_")
/// reserved_prefix("__DYN_x")          → Some("__dyn_")    (case ignored)
/// reserved_prefix("host/__dyn_x")     → Some("__dyn_")    (a segment)
/// reserved_prefix("__lispy")          → None              (no underscore after lisp)
/// reserved_prefix("my__lisp_x")       → None              (not at the start)
/// ```
pub fn reserved_prefix(name: &str) -> Option<&'static str> {
    name.split('/').find_map(|segment| {
        let lower = segment.to_ascii_lowercase();
        RESERVED_PREFIXES.iter().copied().find(|p| lower.starts_with(p))
    })
}

/// Refuse the first `NAME` token in `program` that uses a reserved prefix.
///
/// # Errors
///
/// A [`TwigCompileError`] at the offending token's position.
pub fn check_reserved_names(program: &GrammarASTNode) -> Result<(), TwigCompileError> {
    // A depth-first walk in source order with an explicit stack (children
    // pushed in reverse, so the leftmost is popped first). `check_cst` has
    // already bounded the tree's depth, but a loop needs no such argument.
    let mut stack: Vec<&ASTNodeOrToken> = program.children.iter().rev().collect();
    while let Some(item) = stack.pop() {
        match item {
            // A symbol literal is data, not a name.
            ASTNodeOrToken::Node(n) if matches!(n.rule_name.as_str(), "quoted" | "quote_form") => {}
            ASTNodeOrToken::Node(n) => stack.extend(n.children.iter().rev()),
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                if let Some(prefix) = reserved_prefix(&t.value) {
                    return Err(TwigCompileError {
                        message: format!(
                            "the name {:?} is reserved: names beginning with {prefix:?} \
                             belong to the compiler and its runtime",
                            t.value
                        ),
                        line: t.line,
                        column: t.column,
                    });
                }
            }
            ASTNodeOrToken::Token(_) => {}
        }
    }
    Ok(())
}
