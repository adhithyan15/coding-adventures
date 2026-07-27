//! # c-to-semantic-ir — C (integer-core subset) → Semantic IR.
//!
//! The mirror of the Ruby/Python frontends for a **strict, typed** source
//! language, and the last piece of the C → SIR → Ruby initiative.  It parses C
//! with [`coding_adventures_c_parser`], then walks the CST assigning a concrete
//! `IntSpec` to every expression and inserting `Expr::Convert` nodes per C's
//! integer promotions / usual-arithmetic-conversions — so a C program's
//! width/wrap/truncate semantics survive the narrow waist and every backend
//! reproduces the program's results.
//!
//! Implements [SIR27](../../../specs/SIR27-c-to-semantic-ir.md):
//!
//! - **Milestone 1** — functions, typed `+`/`-`/`*` arithmetic (with `Convert`
//!   insertion), casts, declarations & assignments, `printf`, `return`.
//! - **Milestone 2** — comparisons and control flow (`if`/`else`, `while`,
//!   `for`, re-assignment), bridging the C-vs-SIR truthiness mismatch.
//! - **Milestone 3** — early `return`, lifted into value-producing `If`s, which
//!   is what makes guard clauses and idiomatic recursion translatable.
//! - **Milestone 4** — the short-circuiting logical operators `&&`, `||`, `!`,
//!   reusing the truthiness bridge (`and`/`or`/`not` builtins).
//! - **Milestone 5** — bitwise `& | ^ ~` and shifts `<< >>` (shifts take the
//!   promoted left operand's type, not the usual common type).
//! - **Milestone 6** — division/modulo `/ %`, which *truncate toward zero* in C
//!   (unlike SIR/Ruby floor), via dedicated `tdiv`/`tmod` builtins.
//! - **Milestone 7** — per-block scoping (a scope stack with unique SIR names),
//!   so shadowing and re-used names (e.g. two sequential `for (int i …)` loops)
//!   compile instead of being rejected.

mod lower;

pub use lower::{compile, CLowerError};

/// Parse C `source` and lower it to a [`semantic_ir::Module`].
pub fn compile_source(source: &str, module_name: &str) -> Result<semantic_ir::Module, CLowerError> {
    let tree = coding_adventures_c_parser::try_parse_c(source).map_err(|msg| CLowerError {
        message: format!("C parse error: {msg}"),
        line: 0,
        column: 0,
    })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    /// A per-(process, call) unique stem so parallel tests never share a file.
    fn uniq(ext: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "c2sir_{}_{}{ext}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("C lowering succeeded")
    }

    #[test]
    fn source_language_is_c_and_validates() {
        let m = lower("int main(void) { return 0; }");
        assert_eq!(m.metadata.source_language.as_deref(), Some("c"));
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn uint8_overflow_inserts_convert_chain() {
        // uint8_t c = 200 + 100  →  the assignment narrows to u8 and the add
        // promotes its u8 operands to i32, so the SIR text contains both a u8
        // and an i32 convert.
        let m = lower("int main(void) { uint8_t c = 200 + 100; return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(convert (int u8 wrap)"),
            "no u8 convert:\n{text}"
        );
        assert!(
            text.contains("(convert (int i32 ub)"),
            "no i32 promote:\n{text}"
        );
    }

    // ── milestone 2: control flow & comparisons ─────────────────────────────

    #[test]
    fn while_loop_lowers_to_sir_while_with_bool_cond() {
        let m = lower("int main(void) { int32_t i = 0; while (i < 3) { i = i + 1; } return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(while"), "no while stmt:\n{text}");
        // The condition is the comparison builtin directly (already a bool),
        // not wrapped in a `!= 0`.
        assert!(text.contains("(builtin-call <"), "no `<` cond:\n{text}");
        assert!(text.contains("(assign i local"), "no reassign:\n{text}");
    }

    #[test]
    fn for_loop_desugars_to_while_with_trailing_step() {
        let m = lower(
            "int main(void) { uint32_t s = 0; for (int i = 1; i <= 3; i = i + 1) { s = s + i; } return 0; }",
        );
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(while"),
            "for did not desugar to while:\n{text}"
        );
        // init `i` is bound before the loop; the step reassigns it inside.
        assert!(text.contains("(let* i "), "no loop-var init:\n{text}");
        assert!(text.contains("(assign i local"), "no step assign:\n{text}");
    }

    #[test]
    fn bare_int_condition_gets_ne_zero() {
        // `while (n)` — a non-comparison condition — must become `!= 0`, since
        // SIR treats 0 as truthy (C treats it as false).
        let m = lower("int main(void) { int32_t n = 2; while (n) { n = n - 1; } return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(builtin-call != (effects pure) (var-ref n local) (int 0))"),
            "condition not bridged via != 0:\n{text}"
        );
    }

    #[test]
    fn comparison_as_value_is_if_one_else_zero() {
        // `int c = a > b;` — the comparison is used as a value, so it is
        // `If(cmp, 1, 0)` (C's int-typed 0/1 result).
        let m = lower("int main(void) { int a = 5; int b = 3; int c = a > b; return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(if (builtin-call >"), "no if-int:\n{text}");
    }

    // ── milestone 4: logical operators ──────────────────────────────────────

    #[test]
    fn logical_and_condition_short_circuits_via_and_builtin() {
        // `if (a && b)` → `and(cond(a), cond(b))`, each operand a SIR bool.
        let m = lower("int f(int x) { if (x >= 0 && x < 10) { return 1; } return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(builtin-call and"),
            "no short-circuit and:\n{text}"
        );
        // Its operands are the comparison builtins directly (bools), not `!= 0`.
        assert!(text.contains("(builtin-call >="), "no >= operand:\n{text}");
        assert!(text.contains("(builtin-call <"), "no < operand:\n{text}");
    }

    #[test]
    fn logical_or_uses_or_builtin() {
        let m = lower("int f(int x) { if (x < 0 || x > 9) { return 1; } return 0; }");
        assert!(semantic_ir::print_module(&m).contains("(builtin-call or"));
    }

    #[test]
    fn logical_not_condition_uses_not_builtin() {
        let m = lower("int f(int x) { if (!(x > 5)) { return 1; } return 0; }");
        assert!(semantic_ir::print_module(&m).contains("(builtin-call not"));
    }

    #[test]
    fn bare_variable_operand_is_bridged_via_ne_zero() {
        // `a && b` with plain int operands: each is `!= 0` (SIR treats 0 truthy).
        let m = lower("int f(int a, int b) { if (a && b) { return 1; } return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(builtin-call and"), "no and:\n{text}");
        assert!(
            text.contains("(builtin-call != (effects pure) (var-ref a param) (int 0))"),
            "operand a not bridged via != 0:\n{text}"
        );
    }

    #[test]
    fn logical_as_value_is_if_one_else_zero() {
        // `int r = a && b;` — a logical operator used as a value is int 0/1.
        let m = lower("int f(int a, int b) { int r = a && b; return r; }");
        assert!(semantic_ir::print_module(&m).contains("(if (builtin-call and"));
    }

    #[test]
    fn long_logical_chain_is_a_clean_error_not_a_crash() {
        // A `&&` chain folds into a tree as deep as it is wide, so its width is
        // charged against the shared depth budget.
        let mut cond = String::from("x > 0");
        for i in 0..200 {
            cond.push_str(&format!(" && x != {i}"));
        }
        let err = compile_source(
            &format!("int f(int x) {{ if ({cond}) {{ return 1; }} return 0; }}"),
            "t",
        )
        .unwrap_err();
        assert!(
            err.message.contains("limit"),
            "wrong error: {}",
            err.message
        );
    }

    // ── milestone 5: bitwise & shifts ───────────────────────────────────────

    #[test]
    fn bitwise_operators_emit_their_builtins() {
        let m = lower("int f(int a, int b) { return (a & b) | (a ^ b); }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(builtin-call &"), "no &:\n{text}");
        assert!(text.contains("(builtin-call |"), "no |:\n{text}");
        assert!(text.contains("(builtin-call ^"), "no ^:\n{text}");
    }

    #[test]
    fn bitwise_not_is_a_unary_builtin() {
        let m = lower("int f(int x) { return ~x; }");
        assert!(semantic_ir::print_module(&m).contains("(builtin-call ~"));
    }

    #[test]
    fn shift_result_takes_the_left_operands_type_not_the_common_type() {
        // `x << c` with x:uint8 → promoted to i32; the result is i32 (the
        // promoted LEFT type), NOT common-typed with the count.  So the whole
        // expression narrows only where C says: at the u8 assignment.
        let m = lower("int main(void) { uint8_t x = 1 << 3; return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(builtin-call <<"), "no shift:\n{text}");
        // shift performed at i32, then the declaration narrows to u8.
        assert!(
            text.contains("(convert (int i32 ub)"),
            "shift not at i32:\n{text}"
        );
        assert!(
            text.contains("(convert (int u8 wrap)"),
            "no u8 narrow:\n{text}"
        );
    }

    #[test]
    fn division_and_modulo_lower_to_truncating_builtins() {
        // C `/` truncates toward zero (unlike SIR/Ruby floor), so it must lower
        // to the dedicated `tdiv`/`tmod` builtins, not the flooring `/`/`%`.
        let m = lower("int f(int a, int b) { return a / b; }");
        assert!(semantic_ir::print_module(&m).contains("(builtin-call tdiv"));
        let m = lower("int f(int a, int b) { return a % b; }");
        assert!(semantic_ir::print_module(&m).contains("(builtin-call tmod"));
    }

    #[test]
    fn int_min_div_is_guarded() {
        // INT_MIN / -1 is signed-overflow UB (and traps on x86).  Our emitted C
        // must *not* trap — the `_sir_itdiv` INT64_MIN/-1 guard returns the
        // two's-complement wrap.  Compile+run the emitted C and confirm it exits
        // cleanly with the wrapped value rather than a SIGFPE.
        if let Some(out) = run_c(&lower(
            "int main(void) { int32_t a = -2147483647 - 1; int32_t b = -1; \
             printf(\"%d\\n\", a / b); return 0; }",
        )) {
            assert_eq!(out, "-2147483648", "guarded INT_MIN/-1");
        }
    }

    // ── milestone 3: early return ───────────────────────────────────────────

    #[test]
    fn guard_clause_lifts_into_a_value_producing_if() {
        // `if (x == 0) { return 1; } return 0;` has no early-exit form in SIR,
        // so it becomes the block's *value*: If(x==0, {1}, {0}).
        let m = lower("int f(int x) { if (x == 0) { return 1; } return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(block (if (builtin-call =="),
            "guard clause not lifted into the block value:\n{text}"
        );
    }

    #[test]
    fn guard_clause_does_not_duplicate_the_continuation() {
        // The tail attaches only to the branch that falls through, so it must
        // appear exactly once — this is what keeps the transformation linear.
        let m = lower("int f(int x) { if (x == 0) { return 1; } return 12345; }");
        let text = semantic_ir::print_module(&m);
        assert_eq!(
            text.matches("12345").count(),
            1,
            "continuation duplicated:\n{text}"
        );
    }

    #[test]
    fn recursive_function_with_a_guard_clause_lowers() {
        // The canonical shape that early return unlocks.
        let m =
            lower("int fib(int n) { if (n < 2) { return n; } return fib(n - 1) + fib(n - 2); }");
        assert!(semantic_ir::validate(&m).is_ok());
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(direct-call fib"), "no recursion:\n{text}");
    }

    #[test]
    fn long_statement_sequence_does_not_overflow_the_stack() {
        // Regression: the sequence walk must be ITERATIVE.  Recursing once per
        // statement overflowed the stack at ~350 statements — an ordinary
        // function size, so this was a plain functionality bug as well as a
        // DoS on untrusted input.
        let mut src = String::from("int main(void) { int x = 0;");
        for _ in 0..5000 {
            src.push_str(" x = x + 1;");
        }
        src.push_str(" return x; }");
        let m = lower(&src);
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn continuation_duplicating_if_is_rejected() {
        // `if (c) { if (d) { return 1; } }` — neither branch returns on all
        // paths, so lifting would copy the tail into BOTH.  Chained, that is
        // 4^N IR nodes: <1 KB of C emitted hundreds of MB.  Refuse it.
        let err = compile_source(
            "int f(int x) { if (x > 0) { if (x > 5) { return 1; } } return 0; }",
            "test",
        )
        .unwrap_err();
        assert!(
            err.message.contains("duplicate the rest"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn shadowing_and_reuse_now_lower_with_per_block_scopes() {
        // Milestone 7: per-block scopes make all of these VALID C compile (they
        // were rejected before).  Each binds a distinct SIR name, so the inner
        // `v` shadows without clobbering the outer one.
        for src in [
            "int f(int x) { int v = 1; { uint8_t v = 250; v = v + 6; } return v + 1000; }",
            "int f(int x) { int v = 1; if (x > 0) { return 5; } \
             else { uint8_t v = 250; } return v + 1000; }",
            "int f(int x){ int v=1; if(x>0){return 5;} \
             else { for(uint8_t v=250; v>249; v=v+1){ x=x+1; } } return v+1000; }",
            "int f(int x) { int v = 1; if (x > 0) { uint8_t v = 250; return v; } \
             return v + 1000; }",
            // Two sequential `for (int i = …)` loops re-using `i` — the everyday
            // case this milestone unlocks.
            "int f(int x) { for (int i = 0; i < 3; i = i + 1) { x = x + 1; } \
             for (int i = 0; i < 3; i = i + 1) { x = x + 1; } return x; }",
            // Shadowing a parameter.
            "int f(int v) { if (v > 0) { return 5; } else { uint8_t v = 250; } return v; }",
        ] {
            let m = compile_source(src, "test")
                .unwrap_or_else(|e| panic!("should compile now: {src}: {}", e.message));
            assert!(semantic_ir::validate(&m).is_ok(), "invalid SIR for {src}");
        }
    }

    #[test]
    fn shadowing_binds_a_distinct_sir_name() {
        // The inner `v` must lower to a *different* SIR name (`v__2`) so it can't
        // clobber the outer `v` in SIR's flat namespace.
        let m = lower("int f(int x) { int v = 1; { uint8_t v = 250; } return v + 1000; }");
        let text = semantic_ir::print_module(&m);
        assert!(text.contains("(let* v "), "no outer v:\n{text}");
        assert!(
            text.contains("(let* v__2 "),
            "inner v not uniquified:\n{text}"
        );
    }

    #[test]
    fn redeclaration_in_the_same_block_is_still_an_error() {
        // Per-block scoping accepts shadowing across blocks, but re-declaring the
        // same name in one block is a C error.
        let err =
            compile_source("int f(void) { int v = 1; int v = 2; return v; }", "test").unwrap_err();
        assert!(
            err.message.contains("redeclaration of `v`"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn a_variable_out_of_scope_is_undeclared() {
        // A name declared in an inner block is not visible after the block ends.
        let err =
            compile_source("int f(int x) { { int a = 1; x = a; } return a; }", "test").unwrap_err();
        assert!(
            err.message.contains("undeclared variable `a`"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn distinct_names_in_sibling_branches_still_compile() {
        // Sibling branches each open their own scope, so re-using a name is fine.
        let m = lower(
            "int f(int x) { if (x > 0) { int a = 1; return a; } \
             else { int a = 2; return a; } }",
        );
        assert!(semantic_ir::validate(&m).is_ok());
    }

    /// `if (x > 0) return 0; if (x > 1) return 1; …` — the `sign()` idiom, a
    /// *flat* sequence of sibling guards.  Recursing once per guard overflowed
    /// the stack at ~150, so this shape must stay iterative.
    fn chained_guards(n: usize) -> String {
        let mut s = String::from("int f(int x) {");
        for i in 0..n {
            s.push_str(&format!(" if (x > {i}) return {i};"));
        }
        s.push_str(" return 0; }");
        s
    }

    #[test]
    fn many_sibling_guard_clauses_lower_without_recursing_per_guard() {
        let m = lower(&chained_guards(40));
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn too_many_chained_guards_is_a_clean_error_not_a_crash() {
        // Each lifted guard nests the emitted IR one level deeper, and every
        // consumer of that IR (validator, backends, printer, Drop) walks it
        // recursively — 250 aborted the process inside the validator.  Past the
        // cap it must be a positioned error instead.
        // Guards and expression nesting share one budget (they add in the
        // emitted tree), so either cap may fire first — both are the point.
        let err = compile_source(&chained_guards(250), "test").unwrap_err();
        assert!(
            err.message.contains("too many early returns") || err.message.contains("limit"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn guards_plus_deep_expressions_share_one_budget() {
        // Lifted guards and expression depth ADD in the emitted tree.  64 guards
        // each returning a 50-term chain passed both caps independently and
        // still overflowed the stack, so the budget must be joint.
        let mut src = String::from("int f(int x) {");
        for i in 0..64 {
            src.push_str(&format!(" if (x > {i}) return x"));
            for _ in 0..50 {
                src.push_str(" + 1");
            }
            src.push(';');
        }
        src.push_str(" return 0; }");
        let err = compile_source(&src, "test").unwrap_err();
        assert!(
            err.message.contains("limit"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn nested_chains_do_not_multiply_past_the_cap() {
        // `((((x + 1 ×22) + 1 ×22) …)` — checking a chain's width without
        // *spending* it let each nesting level restart from the same low base,
        // reaching ~14× the cap and aborting the process on a 369-byte input.
        let mut e = String::from("x");
        for _ in 0..4 {
            let mut inner = format!("({e}");
            for _ in 0..22 {
                inner.push_str(" + 1");
            }
            inner.push(')');
            e = inner;
        }
        let err = compile_source(&format!("int f(int x){{ return {e}; }}"), "test").unwrap_err();
        assert!(
            err.message.contains("limit"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn deep_operator_chain_is_a_clean_error_not_a_crash() {
        // A *flat* operator chain folds left into a tree as deep as it is wide,
        // and the validator recurses over it — a 428-byte file of `x + 1 + 1 …`
        // aborted the process (debug builds die around 80 terms).  Chain width
        // is charged against the expression-depth budget, so this errors.
        let mut src = String::from("int f(int x) { return x");
        for _ in 0..200 {
            src.push_str(" + 1");
        }
        src.push_str("; }");
        let err = compile_source(&src, "test").unwrap_err();
        assert!(
            err.message.contains("limit"),
            "wrong error: {}",
            err.message
        );
    }

    #[test]
    fn ordinary_operator_chains_still_lower() {
        let m = lower("int f(int x) { return x + 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8; }");
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn return_inside_a_loop_is_a_clean_error() {
        // Leaving a loop early needs a break-with-value, which SIR has no node
        // for — so this must be a positioned error, not a miscompile.
        let err = compile_source(
            "int f(int x) { while (x > 0) { return 1; } return 0; }",
            "test",
        )
        .unwrap_err();
        assert!(
            err.message.contains("`return` inside a loop"),
            "wrong error: {}",
            err.message
        );
    }

    // ── end-to-end helpers ──────────────────────────────────────────────────

    fn run_ruby(m: &semantic_ir::Module) -> Option<String> {
        use std::io::Write;
        let src = semantic_ir_to_ruby::compile(m).ok()?.source;
        let path = uniq(".rb");
        std::fs::File::create(&path)
            .ok()?
            .write_all(src.as_bytes())
            .ok()?;
        let out = std::process::Command::new("ruby")
            .arg(&path)
            .output()
            .ok()?;
        let _ = std::fs::remove_file(&path);
        if !out.status.success() {
            panic!(
                "emitted ruby failed:\n{}",
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

    fn find_cc() -> Option<String> {
        if let Ok(cc) = std::env::var("SIR_CC") {
            if !cc.trim().is_empty() {
                return Some(cc);
            }
        }
        for c in ["cc", "clang", "gcc"] {
            if std::process::Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(c.to_string());
            }
        }
        None
    }

    fn run_c(m: &semantic_ir::Module) -> Option<String> {
        use std::io::Write;
        let cc = find_cc()?;
        let src = semantic_ir_to_c::compile(m).ok()?.source;
        let cpath = uniq(".c");
        let exe = uniq(std::env::consts::EXE_SUFFIX);
        std::fs::File::create(&cpath)
            .ok()?
            .write_all(src.as_bytes())
            .ok()?;
        let ok = std::process::Command::new(&cc)
            .arg("-std=c99")
            .arg("-o")
            .arg(&exe)
            .arg(&cpath)
            .output()
            .ok()?;
        assert!(
            ok.status.success(),
            "emitted C failed to compile:\n{}\n{src}",
            String::from_utf8_lossy(&ok.stderr)
        );
        let run = std::process::Command::new(&exe).output().ok()?;
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&exe);
        Some(
            String::from_utf8_lossy(&run.stdout)
                .replace("\r\n", "\n")
                .trim_end()
                .to_string(),
        )
    }

    /// The headline case: a C program and BOTH its translations agree on the
    /// uint8 overflow — 200 + 100 == 44 (mod 256).
    #[test]
    fn uint8_overflow_roundtrips_through_ruby_and_c() {
        let m = lower("int main(void) { uint8_t c = 200 + 100; printf(\"%d\\n\", c); return 0; }");
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "44", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "44", "c");
        }
    }

    #[test]
    fn int32_overflow_roundtrips() {
        // (int32_t)(2000000000 + 2000000000): the operands are int (i32) so the
        // add is at i32 and wraps: 4000000000 - 2^32 = -294967296.
        let m = lower(
            "int main(void) { int32_t y = (int32_t)(2000000000 + 2000000000); printf(\"%d\\n\", y); return 0; }",
        );
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "-294967296", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "-294967296", "c");
        }
    }

    #[test]
    fn function_calls_and_params_roundtrip() {
        let m = lower(
            "int add(int a, int b) { return a + b; }\n\
             int main(void) { printf(\"%d\\n\", add(2, 3)); return 0; }",
        );
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "5", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "5", "c");
        }
    }
}
