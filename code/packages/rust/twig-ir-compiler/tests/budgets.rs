//! Compile-time resource budgets (#18474), each tested exactly at its limit
//! and one past it.
//!
//! Before #18474 every "past the limit" program here compiled: the source
//! and the tree had no size limit at all, and a function too big for a VM
//! frame failed only when twig-vm or vm-core tried to run it. Each
//! at-limit test also pins the budget's arithmetic: it asserts that the
//! program really sits *on* the limit, so a test that drifted below the
//! limit would fail instead of passing vacuously.
//!
//! The programs are big (up to 2²⁰ tree items), so each one is built
//! once, from a shape whose cost is linear and measured below:
//!
//! | Shape                      | Tree items | Registers | Instructions |
//! |----------------------------|------------|-----------|--------------|
//! | top-level `1`              | 4          | 1         | 1            |
//! | `x` in a `let` body        | 3          | 0         | 0            |
//! | `(if x x x)` in a `let`    | 15         | 1         | 6            |
//! | record field *i*           | 7          | (own fn)  | ≈ i + 5      |

use interpreter_ir::{IIRFunction, IIRModule};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use twig_ir_compiler::{
    compile_ast, compile_source, compile_typed_source, TwigCompileError, MAX_CST_ITEMS,
    MAX_IIR_INSTRUCTIONS_PER_FUNCTION, MAX_IIR_INSTRUCTIONS_PER_MODULE,
    MAX_REGISTERS_PER_FUNCTION, MAX_SOURCE_BYTES,
};
use vm_core::frame::VMFrame;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `src` must compile; returns the module.
fn compiles(src: &str) -> IIRModule {
    compile_source(src, "m").unwrap_or_else(|e| panic!("must compile: {e}"))
}

/// `src` must be refused by the named budget; returns the error.
fn refused_by(src: &str, which: &str) -> TwigCompileError {
    match compile_source(src, "m") {
        Ok(_) => panic!("a program past the {which} budget compiled"),
        Err(e) => {
            assert!(e.message.starts_with(&format!("{which} exceeds the limit of ")), "{e}");
            e
        }
    }
}

/// Registers as vm-core counts them: parameters + distinct non-parameter
/// `dest`s (the count `crate::budgets` documents).
fn registers(function: &IIRFunction) -> usize {
    let params: std::collections::HashSet<&str> =
        function.params.iter().map(|(p, _)| p.as_str()).collect();
    let dests: std::collections::HashSet<&str> = function
        .instructions
        .iter()
        .filter_map(|i| i.dest.as_deref())
        .filter(|d| !params.contains(d))
        .collect();
    function.params.len() + dests.len()
}

/// Every node and token of the tree, the root included.
fn items(root: &GrammarASTNode) -> usize {
    let mut count = 1;
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        for child in &node.children {
            count += 1;
            if let ASTNodeOrToken::Node(inner) = child {
                pending.push(inner);
            }
        }
    }
    count
}

fn main_of(module: &IIRModule) -> &IIRFunction {
    module.get_function("main").expect("every module has a main")
}

/// `count` top-level `1`s, one per line.
fn ones(count: usize) -> String {
    "1\n".repeat(count)
}

// ---------------------------------------------------------------------------
// The constants
// ---------------------------------------------------------------------------

/// The register budget is vm-core's frame limit, not a number that merely
/// happens to match it. (twig-vm's own tests pin `MAX_REGISTERS_PER_FRAME`
/// to the same constant; this crate cannot depend on twig-vm, which
/// depends on it.)
#[test]
fn the_register_budget_is_the_vm_frame_limit() {
    assert_eq!(MAX_REGISTERS_PER_FUNCTION, VMFrame::MAX_FRAME_REGISTERS);
    // The other budgets keep the register budget reachable: a function at
    // the register limit fits in both instruction budgets and its tree in
    // the item budget, so `registers per function` is a live limit.
    const {
        assert!(MAX_IIR_INSTRUCTIONS_PER_FUNCTION > MAX_REGISTERS_PER_FUNCTION);
        assert!(MAX_IIR_INSTRUCTIONS_PER_MODULE >= MAX_IIR_INSTRUCTIONS_PER_FUNCTION);
        assert!(MAX_CST_ITEMS > 4 * MAX_REGISTERS_PER_FUNCTION);
    }
}

// ---------------------------------------------------------------------------
// Source bytes
// ---------------------------------------------------------------------------

/// A 1 MiB program: a comment that fills the budget, then `1`.
fn source_of_len(len: usize) -> String {
    let src = format!(";{}\n1", "a".repeat(len - 3));
    assert_eq!(src.len(), len);
    src
}

#[test]
fn source_bytes_at_the_limit_compile() {
    let src = source_of_len(MAX_SOURCE_BYTES);
    compiles(&src);
    compile_typed_source(&src, "m").expect("the typed path takes the same source");
}

#[test]
fn source_bytes_past_the_limit_are_refused_before_lexing() {
    let src = source_of_len(MAX_SOURCE_BYTES + 1);
    let e = refused_by(&src, "source bytes");
    assert_eq!((e.line, e.column), (1, 1));
    assert_eq!(e.message, format!("source bytes exceeds the limit of {MAX_SOURCE_BYTES}"));
    let e = compile_typed_source(&src, "m").expect_err("the typed path checks first too");
    assert!(e.message.starts_with("source bytes exceeds"), "{e}");
}

// ---------------------------------------------------------------------------
// CST items
// ---------------------------------------------------------------------------

/// `(let ((x 1)) x x … x)` with `refs` body references, then `ones` lines
/// of `1`: 17 + 3·refs + 4·ones tree items, and three registers or fewer
/// per `1` — a tree as large as the budget allows that stays far below
/// every other budget.
fn wide_let(refs: usize, ones_after: usize) -> String {
    format!("(let ((x 1)) {})\n{}", "x ".repeat(refs), ones(ones_after))
}

#[test]
fn cst_items_at_the_limit_compile() {
    // 17 + 3 × 349,517 + 4 × 2 = 1,048,576 = 2²⁰.
    let src = wide_let(349_517, 2);
    let cst = twig_parser::parse_to_ast(&src).unwrap();
    assert_eq!(items(&cst), MAX_CST_ITEMS, "the program must sit on the limit");
    compile_ast(&cst, "m").expect("a tree of exactly MAX_CST_ITEMS compiles");
}

#[test]
fn cst_items_past_the_limit_are_refused() {
    // 17 + 3 × 349,520 = 1,048,577: one item over.
    let src = wide_let(349_520, 0);
    let cst = twig_parser::parse_to_ast(&src).unwrap();
    assert_eq!(items(&cst), MAX_CST_ITEMS + 1);
    let e = compile_ast(&cst, "m").expect_err("one item over the budget");
    assert_eq!(e.message, format!("CST items exceeds the limit of {MAX_CST_ITEMS}"));
    // Through the source entry point too, and through `Compiler::compile`,
    // which is public and so measures the tree itself.
    refused_by(&src, "CST items");
    let e = twig_ir_compiler::Compiler::new().compile(&cst, "m").unwrap_err();
    assert!(e.message.starts_with("CST items exceeds"), "{e}");
}

// ---------------------------------------------------------------------------
// Registers per function
// ---------------------------------------------------------------------------

#[test]
fn registers_at_the_limit_compile_and_fit_a_vm_core_frame() {
    // Each top-level `1` is one `const` into a fresh register, and `main`
    // returns the last one, so 65,536 of them use exactly 65,536.
    let module = compiles(&ones(MAX_REGISTERS_PER_FUNCTION));
    assert_eq!(registers(main_of(&module)), MAX_REGISTERS_PER_FUNCTION);
    for function in &module.functions {
        VMFrame::check_register_limit(function).expect("vm-core loads every function");
    }
}

#[test]
fn registers_past_the_limit_are_refused_at_the_statement_that_went_over() {
    let e = refused_by(&ones(MAX_REGISTERS_PER_FUNCTION + 1), "registers per function");
    assert_eq!(
        e.message,
        format!("registers per function exceeds the limit of {MAX_REGISTERS_PER_FUNCTION} in function `main`")
    );
    // Line 65,537 is the `1` with no register left for it.
    assert_eq!((e.line, e.column), (MAX_REGISTERS_PER_FUNCTION + 1, 1));
}

#[test]
fn registers_are_budgeted_in_every_function_not_only_main() {
    let body = |n: usize| format!("(define (f)\n{})\n(f)", ones(n));
    let module = compiles(&body(MAX_REGISTERS_PER_FUNCTION));
    let f = module.get_function("f").unwrap();
    assert_eq!(registers(f), MAX_REGISTERS_PER_FUNCTION);
    VMFrame::check_register_limit(f).expect("vm-core loads f");

    let e = refused_by(&body(MAX_REGISTERS_PER_FUNCTION + 1), "registers per function");
    assert!(e.message.ends_with("in function `f`"), "{e}");
    assert_eq!((e.line, e.column), (MAX_REGISTERS_PER_FUNCTION + 2, 1));
}

/// Parameters are positional slots and count even with no instruction
/// writing them (lesson: *a register cap that counts distinct names must
/// also count positional parameter slots*).
#[test]
fn parameters_count_as_registers() {
    let params = |n: usize| (0..n).map(|i| format!("a{i}")).collect::<Vec<_>>().join(" ");
    // The body reads a parameter, so it adds no register of its own.
    let module = compiles(&format!("(define (f {}) a0)", params(MAX_REGISTERS_PER_FUNCTION)));
    let f = module.get_function("f").unwrap();
    assert_eq!(f.params.len(), MAX_REGISTERS_PER_FUNCTION);
    assert_eq!(registers(f), MAX_REGISTERS_PER_FUNCTION);

    let src = format!("\n  (define (f {}) a0)", params(MAX_REGISTERS_PER_FUNCTION + 1));
    let e = refused_by(&src, "registers per function");
    assert!(e.message.ends_with("in function `f`"), "{e}");
    // No instruction is to blame, so the declaration is: the function's
    // name, where the compiler's other errors about a function point.
    assert_eq!((e.line, e.column), (2, 12));
}

// ---------------------------------------------------------------------------
// IIR instructions per function
// ---------------------------------------------------------------------------

/// `(let ((x 1)) (if x x x) …)` with `ifs` copies, then `ones` lines of
/// `1`. An `if` on a bound name costs six instructions and one register,
/// so this reaches the instruction budget with ≈ 43,700 registers.
fn branchy(ifs: usize, ones_after: usize) -> String {
    format!("(let ((x 1))\n{})\n{}", "(if x x x)\n".repeat(ifs), ones(ones_after))
}

/// `main`'s length for [`branchy`], as `base + per_if·ifs + per_one·ones`,
/// measured on small programs (and checked linear) rather than assumed.
fn branchy_len(ifs: usize, ones_after: usize) -> usize {
    let len = |i, o| main_of(&compiles(&branchy(i, o))).instructions.len();
    let per_if = len(2, 1) - len(1, 1);
    let per_one = len(1, 2) - len(1, 1);
    let base = len(1, 1) - per_if - per_one;
    assert_eq!(len(3, 3), base + 3 * per_if + 3 * per_one, "linear");
    base + per_if * ifs + per_one * ones_after
}

/// The ([`branchy`] `ifs`, `ones`) whose `main` holds exactly `target`
/// instructions, with at least one trailing `1`.
fn branchy_shape(target: usize) -> (usize, usize) {
    let per_if = branchy_len(2, 1) - branchy_len(1, 1);
    let at_one = branchy_len(0, 1);
    let ifs = (target - at_one) / per_if;
    let ones_after = 1 + (target - branchy_len(ifs, 1));
    assert_eq!(branchy_len(ifs, ones_after), target);
    (ifs, ones_after)
}

#[test]
fn instructions_per_function_at_the_limit_compile() {
    let (ifs, ones_after) = branchy_shape(MAX_IIR_INSTRUCTIONS_PER_FUNCTION);
    let module = compiles(&branchy(ifs, ones_after));
    let main = main_of(&module);
    assert_eq!(main.instructions.len(), MAX_IIR_INSTRUCTIONS_PER_FUNCTION);
    assert!(registers(main) < MAX_REGISTERS_PER_FUNCTION, "only the instruction budget is at stake");
}

#[test]
fn instructions_per_function_past_the_limit_are_refused() {
    let (ifs, ones_after) = branchy_shape(MAX_IIR_INSTRUCTIONS_PER_FUNCTION + 1);
    let e = refused_by(&branchy(ifs, ones_after), "IIR instructions per function");
    assert!(e.message.ends_with("in function `main`"), "{e}");
    // The instruction over the limit is `main`'s synthetic `ret`, so the
    // error points at the last statement: the final `1`.
    assert_eq!((e.line, e.column), (ifs + 2 + ones_after, 1));
}

// ---------------------------------------------------------------------------
// IIR instructions per module
// ---------------------------------------------------------------------------

/// `(record P (f0 : int) …)` with `fields` fields, then `ones` lines of `1`.
///
/// A record of n fields lowers to a constructor (3n + 2 instructions), n
/// accessors (the i-th walks i `cdr`s: i + 2) and a predicate (2):
///
/// ```text
///   n(n − 1)/2 + 5n + 4   instructions, from about 12n bytes of source
/// ```
///
/// That quadratic growth is why the module budget exists: on main a
/// 25 KB record of 2,000 fields compiled to 2,009,006 instructions, and
/// a 250 KB one would ask for 200 million.
fn record(fields: usize, ones_after: usize) -> String {
    let fields: String = (0..fields).map(|i| format!(" (f{i} : int)")).collect();
    format!("(record P{fields})\n{}", ones(ones_after))
}

fn record_instructions(n: usize) -> usize {
    n * (n - 1) / 2 + 5 * n + 4
}

fn total(module: &IIRModule) -> usize {
    module.functions.iter().map(|f| f.instructions.len()).sum()
}

#[test]
fn the_record_expansion_formula_holds() {
    for n in [1, 2, 7, 30] {
        // `main` with no `1`s is `const nil; ret`: two instructions.
        assert_eq!(total(&compiles(&record(n, 0))), record_instructions(n) + 2, "n = {n}");
    }
}

/// 1,440 fields and 5,291 `1`s: 1,043,284 + 5,292 (`main`: the `1`s and
/// its `ret`) = 1,048,576 = 2²⁰.
#[test]
fn instructions_per_module_at_the_limit_compile() {
    let ones_after = MAX_IIR_INSTRUCTIONS_PER_MODULE - record_instructions(1_440) - 1;
    assert_eq!(ones_after, 5_291);
    let module = compiles(&record(1_440, ones_after));
    assert_eq!(total(&module), MAX_IIR_INSTRUCTIONS_PER_MODULE);
}

#[test]
fn instructions_per_module_past_the_limit_are_refused() {
    // One more `1`: `main`, admitted last, takes the module one over.
    let ones_after = MAX_IIR_INSTRUCTIONS_PER_MODULE - record_instructions(1_440);
    let e = refused_by(&record(1_440, ones_after), "IIR instructions per module");
    assert!(e.message.ends_with("in function `main`"), "{e}");
    assert_eq!((e.line, e.column), (1 + ones_after, 1));
}

/// A record alone can pass the module budget: 1,445 fields need
/// 1,050,519 instructions. The refusal names the accessor that went over
/// and points at the `(record …)` form, because every accessor
/// instruction is compiler-written.
#[test]
fn a_record_whose_accessors_pass_the_module_budget_is_refused() {
    assert!(record_instructions(1_445) > MAX_IIR_INSTRUCTIONS_PER_MODULE);
    let e = refused_by(&format!("\n\n {}", record(1_445, 0)), "IIR instructions per module");
    assert!(e.message.contains("in function `p-f"), "{e}");
    assert_eq!((e.line, e.column), (3, 2));
}
