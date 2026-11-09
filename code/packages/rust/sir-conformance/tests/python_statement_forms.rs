//! Python **statement forms** on every backend: assignment, and the
//! function-body and suite fixes found with it (#18482).
//!
//! Python spells five statement forms with one grammar rule:
//!
//! ```text
//!   assign_stmt = expression_list [ assign_suffix | augmented_assign | annotation ] ;
//! ```
//!
//! The frontend used to look only for `assign_suffix` and lowered *any other*
//! tail as the bare expression on the left.  Valid programs then computed the
//! wrong value without any error:
//!
//! | source            | used to compile to | so it printed |
//! |-------------------|--------------------|---------------|
//! | `x += 2`          | `x`                | the old `x`   |
//! | `x: int = 5`      | `x`                | (refused: `x` unbound) |
//! | `a = b = 0`       | `a = b`            | (`a` got `b`) |
//!
//! Testing those fixes end to end turned up more silent miscompiles in the
//! statement lowerer, which this file also pins down:
//!
//! - assigning a **parameter** inside a loop declared a new local that
//!   shadowed it until the end of the loop body, so `while n > 0: n -= 1`
//!   never changed the `n` the condition reads (an endless loop);
//! - a function that **falls off its end** returned its last expression
//!   statement's value instead of `None`;
//! - an expression statement **just before a tail `return`** (a call, a
//!   `print`, an `if`) was dropped;
//! - a **one-line body** (`def f(x): return x + 1`, `if c: y = 1`) was
//!   dropped whole;
//! - a `for` loop over an **already-bound** variable declared a fresh one,
//!   so the variable did not keep the last item;
//! - a user **`def main`** clashed with the synthesised module body, which
//!   the validator refused.
//!
//! Each program's expected stdout is the value CPython prints.  The run
//! test re-checks that against a real `python3` (or `python`) when one is on `PATH`, so a
//! wrong expectation cannot hide a wrong lowering.  Booleans and `None`
//! follow Python's spelling (`True`, `False`, `None`) via the Python
//! display convention (#18496 item 1).
//!
//! `while` loops are bounded by a guard counter, so a regression of the
//! parameter-shadowing bug fails with a wrong number instead of hanging.
//! The guard is a local declared before the loop and updated with a plain
//! `guard = guard + 1`, which the old lowering already got right: a guard
//! written `guard += 1` was dropped with the rest, and the old lowering
//! looped forever.

use std::io::Write;
use std::process::{Command, Stdio};

use sir_conformance::{lower_source_via, run_source_via, Frontend, RunOutcome, Target};

/// `(name, Python source, CPython's stdout)`.
const PROGRAMS: &[(&str, &str, &str)] = &[
    // ── display convention: booleans and None (#18496 item 1) ──────────
    (
        "py_display_booleans_and_none",
        "print(True)\nprint(False)\nprint(None)\nprint(True, False, None)\n",
        "True\nFalse\nNone\nTrue False None",
    ),
    (
        "py_display_booleans_and_none_in_list",
        "print([True, False, None])\n",
        "[True, False, None]",
    ),
    // ── augmented assignment ──────────────────────────────────────────
    // The issue's program, extended over the lowered operators.  (`%=` is
    // lowered too, but the binary `%` itself does not run on every backend
    // yet, so it is pinned by the frontend's unit tests only.)
    // 1 + 2 = 3; 3 - 5 = -2; -2 * -3 = 6.
    (
        "py_aug_int_ops",
        "x = 1\nx += 2\nx -= 5\nx *= -3\nprint(x)\n",
        "6",
    ),
    // `/=` true-divides, exactly like `/`.
    ("py_aug_true_div", "x = 7\nx /= 2\nprint(x)\n", "3.5"),
    // `+=` on a string concatenates.
    ("py_aug_str", "s = \"ab\"\ns += \"cd\"\ns += s\nprint(s)\n", "abcdabcd"),
    // The target is read *before* the right-hand side runs, and here the
    // right-hand side changes the target: `bump` sets `self.n` to 100 and
    // returns 5.  CPython reads 1 first: 1 + 5 = 6 (not 105).  The C
    // backend used to render both operands inline as C call arguments,
    // whose order C leaves unspecified, and gcc ran `bump()` first.
    (
        "py_aug_reads_target_first",
        "class K:\n    def __init__(self):\n        self.n = 1\n    def bump(self):\n        self.n = 100\n        return 5\n    def go(self):\n        self.n += self.bump()\n        return self.n\n\nprint(K().go())\n",
        "6",
    ),
    // Inside a function, on a local and on a parameter, in a loop.  The
    // guard keeps a regression from looping forever.
    // 4 + 3 + 2 + 1 = 10.
    (
        "py_aug_param_in_loop",
        "def total(n):\n    t = 0\n    guard = 0\n    while n > 0 and guard < 100:\n        t += n\n        n -= 1\n        guard = guard + 1\n    return t\n\nprint(total(4))\n",
        "10",
    ),
    // The same with plain `n = n - 1`, which had the same shadowing bug.
    (
        "py_assign_param_in_loop",
        "def total(n):\n    t = 0\n    guard = 0\n    while n > 0 and guard < 100:\n        t = t + n\n        n = n - 1\n        guard = guard + 1\n    return t * 100 + n\n\nprint(total(4))\n",
        "1000",
    ),
    // A nested def captures the parameter's *current* value (its copy).
    (
        "py_param_assigned_then_captured",
        "def outer(n):\n    n += 1\n    def g():\n        return n * 10\n    return g()\n\nprint(outer(5))\n",
        "60",
    ),
    // A parameter assigned in an `if` branch keeps the new value after it.
    (
        "py_assign_param_in_if",
        "def clamp(n):\n    if n > 10:\n        n = 10\n    return n\n\nprint(clamp(50) + clamp(3))\n",
        "13",
    ),
    // `self.n += 1` inside a method reads and writes the instance variable.
    (
        "py_aug_instance_var",
        "class Counter:\n    def __init__(self):\n        self.n = 0\n    def bump(self, k):\n        self.n += k\n        return self.n\n\nc = Counter()\nc.bump(2)\nprint(c.bump(5))\n",
        "7",
    ),
    // ── annotated assignment ──────────────────────────────────────────
    // `x: int = 5` binds x; `y: int` binds nothing.
    (
        "py_annotated",
        "x: int = 5\ny: int\nx += 1\nprint(x)\n",
        "6",
    ),
    (
        "py_annotated_in_function",
        "def f(a):\n    b: int = a * 2\n    c: str\n    return b + 1\n\nprint(f(20))\n",
        "41",
    ),
    // ── chained assignment ────────────────────────────────────────────
    // Both targets get the value; they are separate variables afterwards.
    (
        "py_chained",
        "a = b = 7\nb += 1\nprint(a * 100 + b)\n",
        "708",
    ),
    // The value is evaluated once: both names hold the *same* list.
    (
        "py_chained_shares_one_list",
        "a = b = [0]\na[0] = 5\nprint(b[0])\n",
        "5",
    ),
    // Three targets, one of them a subscript, assigned left to right.
    (
        "py_chained_three_targets",
        "xs = [0, 0]\nn = xs[1] = m = 4\nprint(n + xs[1] + m)\n",
        "12",
    ),
    // ── list concatenation on binary `+` (#18496 item 3) ──────────────
    (
        "py_list_concat_plus",
        "print([1] + [2])\n",
        "[1, 2]",
    ),
    (
        "py_list_concat_multiple",
        "print([1, 2] + [3, 4] + [5])\n",
        "[1, 2, 3, 4, 5]",
    ),
    // ── `for` over an already-bound variable ──────────────────────────
    // The variable keeps the last item after the loop.
    (
        "py_for_rebinds_local",
        "i = 10\nfor i in range(3):\n    t = i\nprint(i)\n",
        "2",
    ),
    (
        "py_for_each_rebinds_local",
        "x = 0\nfor x in [5, 6, 7]:\n    t = x\nprint(x)\n",
        "7",
    ),
    // A loop that runs zero times leaves the variable alone (j = 10); the
    // next loop then rebinds it (i = 2).
    (
        "py_for_empty_then_rebinds",
        "i = 10\nfor i in range(0):\n    t = i\nj = i\nfor i in range(3):\n    t = i\nprint(j * 10 + i)\n",
        "102",
    ),
    // The body assigns the loop variable; the range still decides the
    // next value.  The JavaScript backend counted with the variable
    // itself, so the inner loop's last store (2) ended the outer loop
    // after one pass, and `i += 10` ended the loop after one pass.
    (
        "py_for_nested_rebinds_outer_var",
        "for i in range(2):\n    for i in range(3):\n        pass\n    print(i)\n",
        "2\n2",
    ),
    (
        "py_for_body_assigns_loop_var",
        "for i in range(3):\n    i += 10\n    print(i)\n",
        "10\n11\n12",
    ),
    // A loop body's last statement, when it is an expression statement
    // (a `print`, a trailing `if`), became the body block's *value*, which
    // the C and Ruby backends never emit: nothing was printed there.
    (
        "py_expression_last_in_loop_body",
        "for i in range(3):\n    print(i)\nx = 0\nwhile x < 2:\n    x += 1\n    if x == 2:\n        print(x * 10)\n",
        "0\n1\n2\n20",
    ),
    // Two code-running arguments of ONE method call run left to right
    // (#18768): `c.get()` reads 1 before `c.bump()` adds 100.  CPython: 1005.
    (
        "py_dispatch_args_left_to_right",
        "class C:\n    def __init__(self):\n        self.n = 1\n    def bump(self):\n        self.n = self.n + 100\n        return 5\n    def get(self):\n        return self.n\n    def pair(self, a, b):\n        return a * 1000 + b\n\nc = C()\nprint(c.pair(c.get(), c.bump()))\n",
        "1005",
    ),
    // A constructor and a method call whose argument is compound.  After the
    // first ordering fix the C backend emitted `_sir_unknown_builtin("__new__")`
    // for the first and refused the second.
    (
        "py_dispatch_with_compound_arg",
        "class P:\n    def __init__(self, v):\n        self.v = v\n    def get(self):\n        return self.v\n    def add(self, k):\n        return self.v + k\n\na = P(2)\nb = P(3)\nc = P(a.get() + b.get())\nprint(a.add(c.get() + b.get()) * 100 + c.get())\n",
        "1005",
    ),
    // An indirect call evaluates its callee before its argument.  The C
    // backend rendered both inline, and gcc ran `side()` first.
    (
        "py_indirect_call_callee_first",
        "def show(s):\n    print(s)\n    return 0\n\ndef make():\n    show(\"callee\")\n    return lambda x: x\n\ndef side():\n    show(\"arg\")\n    return 1\n\nr = make()(side())\n",
        "callee\narg",
    ),
    // A local spelled like a synthesised lambda (`__lambda_0`) must not
    // shadow it.  JavaScript failed with `__lambda_0 is not a function`.
    (
        "py_local_spelled_like_a_lambda",
        "def g():\n    __lambda_0 = 5\n    f = lambda: 1\n    return f() + __lambda_0\n\nprint(g())\n",
        "6",
    ),
    // Two names the *frontend* invents must not coincide either (#18771).
    // The assigned parameter `a__b` is copied into the temporary
    // `__param_a__b_0`, and the method `b_0` of class `__param_a` is hoisted
    // to the function `__param_a__b_0`: the same spelling.  Each generator
    // skipped source names, and the method skipped other functions, but
    // neither skipped the other's inventions.  The method's closure is built
    // inside `f`, where the copy shadowed it, and JavaScript failed with
    // `TypeError: __param_a__b_0 is not a function`.  2 + 40 = 42.
    (
        "py_param_copy_spelled_like_a_hoisted_method",
        "def f(a__b):\n    a__b += 1\n    class __param_a:\n        def b_0(self):\n            return 40\n    return a__b + __param_a().b_0()\nprint(f(1))\n",
        "42",
    ),
    // The other order: the hoisted method's name is invented *first* (the
    // class is lowered at top level), and a later function's parameter copy
    // must not take it.  `g`'s copy of `a__b` lives in `g` only, so this
    // program never failed, but it pins that the two generators share one
    // set of used names in both directions.  2 + 40 = 42.
    (
        "py_hoisted_method_then_param_copy",
        "class __param_a:\n    def b_0(self):\n        return 40\ndef g(a__b):\n    a__b += 1\n    return a__b + __param_a().b_0()\nprint(g(1))\n",
        "42",
    ),
    // ── `pass` ────────────────────────────────────────────────────────
    (
        "py_pass_everywhere",
        "def f():\n    pass\n\ndef g(): pass\n\nx = 0\nfor i in range(3): pass\nwhile x > 0: pass\nif x == 0:\n    pass\nelse:\n    x = 9\nif f() == None:\n    if g() == None:\n        print(x)\n",
        "0",
    ),
    // ── falling off the end of a function returns None ────────────────
    (
        "py_implicit_none",
        "def f(x):\n    x + 1\n\nr = f(1)\nif r == None:\n    print(\"none\")\nelse:\n    print(r)\n",
        "none",
    ),
    (
        "py_docstring_only_returns_none",
        "def f():\n    \"doc\"\n\nr = f()\nif r == None:\n    print(\"none\")\nelse:\n    print(r)\n",
        "none",
    ),
    // ── a statement just before a tail `return` runs ─────────────────
    // `assemble_block` made it the block's value, which the `return`
    // then replaced: the call to `bump`, and the `print`, were dropped.
    (
        "py_call_before_return",
        "def bump(xs):\n    xs[0] = xs[0] + 1\n    return 0\n\ndef f(xs):\n    bump(xs)\n    return xs[0]\n\nprint(f([5]))\n",
        "6",
    ),
    (
        "py_print_before_return",
        "def f():\n    print(\"ran\")\n    return 1\n\nr = f()\n",
        "ran",
    ),
    // ── one-line bodies ───────────────────────────────────────────────
    // A body on the header's line is a bare `simple_stmt`, which the suite
    // lowerers skipped: `f` returned None and the `if`/`for` did nothing.
    // f(4) = 5, then the loop takes 2 off.
    (
        "py_one_line_bodies",
        "def f(x): return x + 1\n\nx = 0\nif x == 0: x = f(4)\nfor i in range(2): x -= 1\nprint(x)\n",
        "3",
    ),
    // ── a user `def main` ─────────────────────────────────────────────
    (
        "py_user_main",
        "def main():\n    x = 1\n    x += 41\n    return x\n\nprint(main())\n",
        "42",
    ),
    // Called through a value, and next to a def that spells the escape.
    (
        "py_user_main_value",
        "def main():\n    return 40\n\ndef __py_main():\n    return 2\n\nh = main\nprint(h() + __py_main())\n",
        "42",
    ),
    // A *variable* spelled like the escaped `main` must not shadow it.
    (
        "py_variable_spelled_like_escaped_main",
        "def main():\n    return 1\n\ndef g():\n    __py_main = 5\n    return main() * 10 + __py_main\n\nprint(g())\n",
        "15",
    ),
    // ── floored modulo `%` across all signs (#18496 item 2) ───────────
    (
        "py_mod_signs",
        "print(7 % 3)\nprint(-7 % 3)\nprint(7 % -3)\nprint(-7 % -3)\n",
        "1\n2\n-2\n-1",
    ),
    // ── list in-place mutation and aliasing (#18496 item 4) ────────────
    (
        "py_list_append_aliased",
        "xs = [1, 2]\nys = xs\nxs.append(3)\nprint(ys)\n",
        "[1, 2, 3]",
    ),
    (
        "py_list_aug_assign_aliased",
        "xs = [1, 2]\nys = xs\nxs += [3, 4]\nprint(ys)\n",
        "[1, 2, 3, 4]",
    ),
    (
        "py_number_aug_assign_not_aliased",
        "n = 1\nm = n\nn += 2\nprint(m)\nprint(n)\n",
        "1\n3",
    ),
    // ── subscript assignment evaluation order (Python §6.3, #18496 review)
    (
        "py_subscript_assign_eval_order",
        "log = []\ndef f():\n    log.append(1)\n    return 42\ndef g():\n    log.append(2)\n    return 0\nxs = [0]\nxs[g()] = f()\nprint(xs[0])\nprint(log)\n",
        "42\n[1, 2]",
    ),
    (
        "py_chained_subscript_assign_eval_order",
        "log = []\ndef f():\n    log.append(1)\n    return 99\ndef g():\n    log.append(2)\n    return 0\ndef h():\n    log.append(3)\n    return 1\nxs = [0, 0]\nxs[g()] = xs[h()] = f()\nprint(xs)\nprint(log)\n",
        "[99, 99]\n[1, 2, 3]",
    ),
    // ── nested def shadowing parameter or local (#18496 item 7) ──────
    (
        "py_nested_def_shadow_param",
        "def f(n):\n    def n():\n        return 42\n    return n()\n\nprint(f(10))\n",
        "42",
    ),
    (
        "py_nested_def_shadow_local",
        "def f():\n    n = 1\n    def n():\n        return 42\n    return n()\n\nprint(f())\n",
        "42",
    ),
    (
        "py_nested_def_shadow_param_returns_closure",
        "def f(n):\n    def n():\n        return 42\n    return n\n\nprint(f(10)())\n",
        "42",
    ),
    // ── closure cell variables (#18496 item 5) ────────────────────────
    (
        "py_param_captured_then_assigned",
        "def f(x):\n    def g():\n        return x\n    x = 20\n    return g()\n\nprint(f(10))\n",
        "20",
    ),
    (
        "py_local_captured_then_assigned",
        "def f():\n    x = 10\n    g = lambda: x\n    x = 20\n    return g()\n\nprint(f())\n",
        "20",
    ),
    (
        "py_closure_defined_before_variable_assigned",
        "def f():\n    def g():\n        return x\n    x = 42\n    return g()\n\nprint(f())\n",
        "42",
    ),
    (
        "py_loop_closure_captures_variable",
        "def f():\n    fn = lambda: 0\n    for i in [1, 2, 3]:\n        fn = lambda: i\n    return fn()\n\nprint(f())\n",
        "3",
    ),
    (
        "py_nested_closure_cell_variable",
        "def outer():\n    x = 1\n    def mid():\n        def inner():\n            return x\n        return inner\n    fn = mid()\n    x = 100\n    return fn()\n\nprint(outer())\n",
        "100",
    ),
];

/// `(program, backend, why)`: cells that fail for a reason outside what
/// this file tests, each tracked separately.  Such a cell must still
/// *fail*: once its bug is fixed, the run test reports the entry as stale,
/// so it cannot outlive the bug and hide a later regression.
const KNOWN_GAPS: &[(&str, Target, &str)] = &[
    // Empty.  The two #18771 programs were listed here for Ruby: they need a
    // class named `__param_a`, which the Ruby backend emitted as
    // `Object.const_set(:__param_a, …)` and Ruby refused (`wrong constant
    // name`).  The backend now renames such a class to a valid constant
    // (#18778), so both cells run, and the stale-entry check below made
    // their entries come out.
];

/// CPython's stdout for `source` (trailing newlines trimmed), or `None` when
/// no `python3` is on `PATH`.
///
/// `-I` (isolated mode) keeps the current directory and `PYTHON*`
/// variables out of the interpreter's import path.
fn cpython(source: &str) -> Option<String> {
    let mut child = Command::new("python3")
        .args(["-I", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("python")
                .args(["-I", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        })
        .ok()?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(source.as_bytes())
        .expect("write the program to python3");
    let out = child.wait_with_output().expect("python3 runs");
    assert!(
        out.status.success(),
        "CPython rejected the program:\n{source}\n{}",
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
        let module = lower_source_via(name, source, Frontend::Python)
            .unwrap_or_else(|e| panic!("`{name}` failed to lower: {e}"));
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "`{name}` failed validation: {:?}",
            report.issues
        );
    }
}

/// Every program prints CPython's output on every available backend.
///
/// Every `(program, backend)` cell runs before the test reports, so one
/// failure lists all the cells that disagree.  A failed run is reported
/// with only the head of its message: the full message embeds the whole
/// emitted program and runtime.
#[test]
fn programs_print_cpythons_output_on_every_backend() {
    let mut ran = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (name, source, expected) in PROGRAMS {
        // The expectation is CPython's output: check it whenever CPython
        // is available, so a wrong expectation cannot hide a wrong lowering.
        if let Some(actual) = cpython(source) {
            assert_eq!(actual, *expected, "`{name}`: CPython prints {actual:?}");
        }
        for &target in Target::all() {
            let gap = KNOWN_GAPS
                .iter()
                .find(|(program, t, _)| program == name && *t == target);
            if let Some((_, _, why)) = gap {
                match run_source_via(name, source, Frontend::Python, target) {
                    RunOutcome::Ran(stdout) if stdout == *expected => wrong.push(format!(
                        "`{name}` on {} now passes: remove its KNOWN_GAPS entry ({why})",
                        target.tag()
                    )),
                    _ => eprintln!("known gap {name}/{}: {why}", target.tag()),
                }
                continue;
            }
            match run_source_via(name, source, Frontend::Python, target) {
                RunOutcome::Ran(stdout) => {
                    if stdout != *expected {
                        wrong.push(format!(
                            "`{name}` on {}: printed {stdout:?}, expected {expected:?}",
                            target.tag()
                        ));
                    }
                    ran += 1;
                }
                // A missing toolchain is a skip.  A backend that *refuses*
                // the program ("does not yet accept") is a failure: every
                // program here is ordinary Python that each backend ran at
                // some point, and a refusal is how a regression in the C
                // backend's dispatch path first showed up (#18768).
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
    eprintln!("python statement forms: {ran} program/backend runs");
    assert!(
        wrong.is_empty(),
        "{} of the program/backend cells disagree with CPython:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
