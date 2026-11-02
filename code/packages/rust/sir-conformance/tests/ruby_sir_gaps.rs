//! The Ruby-on-SIR gaps of #18778, run on every backend.
//!
//! Each gap below made an ordinary program fail to compile, crash, or — the
//! worst kind — print the wrong answer with no error:
//!
//! | gap                                             | what used to happen                         |
//! |-------------------------------------------------|---------------------------------------------|
//! | a Python `class dog:` on the Ruby backend       | `wrong constant name dog (NameError)`       |
//! | a Ruby `for` loop                               | parse panic (no `for` in the grammar)       |
//! | `rescue => e` with no class                     | `=> e` lowered as a call of a method `=>`   |
//! | `f() + f` at the start of a statement           | parse panic at the `+`                      |
//! | a bare `g` that names a method                  | refused: "references unknown name `g`"      |
//! | `[1].each do … end`                             | refused by the Ruby backend                 |
//! | a `[` that starts a line after an expression    | glued on as an index: `x = 5[1].each …`; C: `undefined method []`, and `puts` ⏎ `[7].each …` printed the wrong thing on every backend |
//!
//! Fixing them turned up two more of the same kind, pinned here too:
//!
//! - a `while` body's last statement became the body's *value*, which the C
//!   and Ruby backends never emit: `while … puts x end` printed nothing
//!   there;
//! - a method ending in `x if c` returned `nil` whether or not `c` held.
//!
//! Every program's expected stdout is the reference interpreter's: real
//! `ruby` for the Ruby programs and CPython for the Python ones.  The run
//! test re-checks each expectation against the interpreter when it is on
//! `PATH`, so a wrong expectation cannot hide a wrong lowering.
//!
//! A cell may be listed in [`REFUSED`]: a backend that does not implement
//! something the program needs must refuse it at compile time ("does not
//! yet accept").  A listed cell that runs instead fails the test, so the
//! entry cannot outlive the gap; an unlisted refusal fails it too.  A wrong
//! answer is never acceptable anywhere.

use std::io::Write;
use std::process::{Command, Stdio};

use sir_conformance::{lower_source_via, run_source_via, Frontend, RunOutcome, Target};

/// `(name, Ruby source, Ruby's stdout)`.
const RUBY_PROGRAMS: &[(&str, &str, &str)] = &[
    // ── `for` ─────────────────────────────────────────────────────────
    // The loop variable is a local of the enclosing scope: it keeps the
    // last item after the loop.
    (
        "rb_for_over_an_array_keeps_the_last_item",
        "for x in [1, 2, 3]\n  puts x * 10\nend\nputs x\n",
        "10\n20\n30\n3",
    ),
    // `a...b` stops before `b`, `a..b` at `b`; `do` is optional.
    // t = (0+1+2+3) + (100+200+300) = 606; i = 3, j = 3.
    (
        "rb_for_over_exclusive_and_inclusive_ranges",
        "t = 0\nfor i in 0...4 do\n  t = t + i\nend\nfor j in 1..3\n  t = t + j * 100\nend\nputs t\nputs i + j\n",
        "606\n6",
    ),
    // A loop that never runs leaves a bound variable alone, and leaves a
    // new one `nil`.
    (
        "rb_for_zero_trips",
        "y = 7\nfor y in []\nend\nputs y\nfor z in []\nend\nif z == nil\n  puts \"none\"\nend\n",
        "7\nnone",
    ),
    // Inside a method, over a parameter's value; the variable is read
    // after the loop.  6 * 100 + 3 = 603.
    (
        "rb_for_in_a_method_body",
        "def total(xs)\n  t = 0\n  for x in xs\n    t = t + x\n  end\n  t * 100 + x\nend\nputs total([1, 2, 3])\n",
        "603",
    ),
    // A body that ends in an `if` (its `puts` must still run on C and
    // Ruby), and an inner loop that rebinds the outer loop's variable.
    (
        "rb_for_body_ends_in_an_if_and_nested_loops_share_a_variable",
        "for i in 1..3\n  if i == 2\n    puts i\n  end\nend\nfor i in 0...2\n  for i in 0...3\n  end\n  puts i\nend\n",
        "2\n2\n2",
    ),
    // (A loop variable named like a method is in
    // `local_shadows_function.rs`.)
    // ── a loop body's last statement ──────────────────────────────────
    (
        "rb_while_body_ends_in_puts",
        "x = 0\nwhile x < 3\n  x += 1\n  puts x\nend\n",
        "1\n2\n3",
    ),
    // ── `rescue => e` ─────────────────────────────────────────────────
    (
        "rb_rescue_arrow_without_a_class",
        "begin\n  raise \"boom\"\nrescue => e\n  puts \"caught\"\nend\n",
        "caught",
    ),
    // The binding, named like a method, is the local in the clause body.
    // `message` is a built-in method C and Ruby do not run yet: REFUSED.
    (
        "rb_rescue_arrow_binding_named_like_a_method",
        "def e\n  1\nend\nbegin\n  raise \"boom\"\nrescue => e\n  puts e.message\nend\n",
        "boom",
    ),
    // ── `f() + f` ─────────────────────────────────────────────────────
    // At the start of a statement (here the method's last), …
    (
        "rb_call_plus_name_at_the_start_of_a_statement",
        "def g(f)\n  f() + f\nend\ndef f\n  40\nend\nputs g(2)\n",
        "42",
    ),
    // … inside a block body, …
    (
        "rb_call_plus_name_in_a_block",
        "def f\n  40\nend\n[2].each do |f|\n  puts f() + f\nend\n",
        "42",
    ),
    // … and before a modifier `if`, as a method's value.
    (
        "rb_call_plus_name_before_a_modifier_if",
        "def f\n  40\nend\ndef h(f)\n  f() + f if f > 0\nend\nputs h(2)\n",
        "42",
    ),
    // ── a bare method name ────────────────────────────────────────────
    // A call until a local of that name is assigned; `g()` always calls.
    (
        "rb_bare_name_calls_until_a_local_is_assigned",
        "def g\n  42\nend\nputs g\ng = 1\nputs g\nputs g()\n",
        "42\n1\n42",
    ),
    (
        "rb_bare_name_in_a_method_body",
        "def two\n  2\nend\ndef four\n  two * two\nend\nputs four\n",
        "4",
    ),
    (
        "rb_bare_name_in_a_block",
        "def base\n  10\nend\n[1, 2].each do |x|\n  puts base + x\nend\n",
        "11\n12",
    ),
    // A local around the block wins over the method of the same name.
    (
        "rb_outer_local_beats_a_method_in_a_block",
        "def n\n  100\nend\nn = 5\n[1].each do |x|\n  puts n + x\nend\n",
        "6",
    ),
    // ── `each` with a block, and a `[` that starts a line ─────────────
    (
        "rb_each_over_an_array",
        "[1, 2].each do |x|\n  puts x * 10\nend\n",
        "10\n20",
    ),
    (
        "rb_each_over_a_hash",
        "{a: 1, b: 2}.each do |k, v|\n  puts v\nend\n",
        "1\n2",
    ),
    (
        "rb_each_in_a_method",
        "def f\n  [1, 2].each do |x|\n    puts x\n  end\nend\nf\n",
        "1\n2",
    ),
    // The issue's C failure: `x = 5` ⏎ `[1, 2].each …` read as an index
    // into 5.
    (
        "rb_bracket_line_after_an_assignment",
        "x = 5\n[1, 2].each do |v|\n  puts v + x\nend\nputs x\n",
        "6\n7\n5",
    ),
    // `puts 3` ⏎ `[4].each …` read as `puts(3[4].each …)`.
    (
        "rb_bracket_line_after_a_call",
        "puts 3\n[4].each { |v| puts v }\n",
        "3\n4",
    ),
];

/// `(name, Python source, CPython's stdout)`: class names that are not Ruby
/// constants.
const PYTHON_PROGRAMS: &[(&str, &str, &str)] = &[
    (
        "py_lower_case_class",
        "class dog:\n    def bark(self):\n        return 7\nprint(dog().bark())\n",
        "7",
    ),
    // A lower-case base class, and three classes whose Ruby constants must
    // stay apart: `dog` is renamed, `Dog` is not, and `SirCldog` is spelled
    // exactly like `dog`'s renamed constant.
    (
        "py_renamed_classes_stay_one_to_one",
        "class animal:\n    def legs(self):\n        return 4\n\nclass dog(animal):\n    def speak(self):\n        return \"woof\"\n\nclass Dog:\n    def speak(self):\n        return \"Woof\"\n\nclass SirCldog:\n    def speak(self):\n        return \"impostor\"\n\nd = dog()\nprint(d.speak())\nprint(d.legs())\nprint(Dog().speak())\nprint(SirCldog().speak())\n",
        "woof\n4\nWoof\nimpostor",
    ),
];

/// `(program, backend, why)`: cells that must be refused at compile time.
const REFUSED: &[(&str, Target, &str)] = &[
    (
        "rb_rescue_arrow_binding_named_like_a_method",
        Target::C,
        "the built-in method `message` (Collections batch)",
    ),
    (
        "rb_rescue_arrow_binding_named_like_a_method",
        Target::Ruby,
        "the built-in method `message` (Collections batch)",
    ),
];

/// The reference interpreter's stdout for `source` (trailing newlines
/// trimmed), or `None` when it is not on `PATH`.
fn reference(frontend: Frontend, source: &str) -> Option<String> {
    let mut cmd = match frontend {
        Frontend::Ruby => {
            let mut c = Command::new("ruby");
            c.arg("-");
            c
        }
        Frontend::Python => {
            // `-I` keeps the current directory and `PYTHON*` variables out
            // of the interpreter's import path.
            let mut c = Command::new("python3");
            c.args(["-I", "-"]);
            c
        }
        Frontend::JavaScript => unreachable!("no JavaScript programs here"),
    };
    let mut child = cmd
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
        .expect("write the program to the interpreter");
    let out = child.wait_with_output().expect("the interpreter runs");
    assert!(
        out.status.success(),
        "{} rejected the program:\n{source}\n{}",
        frontend.tag(),
        String::from_utf8_lossy(&out.stderr)
    );
    Some(
        String::from_utf8_lossy(&out.stdout)
            .replace("\r\n", "\n")
            .trim_end_matches('\n')
            .to_string(),
    )
}

/// Every program, with its frontend.
fn all_programs() -> impl Iterator<Item = (Frontend, &'static str, &'static str, &'static str)> {
    RUBY_PROGRAMS
        .iter()
        .map(|&(n, s, e)| (Frontend::Ruby, n, s, e))
        .chain(PYTHON_PROGRAMS.iter().map(|&(n, s, e)| (Frontend::Python, n, s, e)))
}

/// Every program lowers to SIR that passes the validator.
#[test]
fn programs_lower_and_validate() {
    for (frontend, name, source, _) in all_programs() {
        let module = lower_source_via(name, source, frontend)
            .unwrap_or_else(|e| panic!("`{name}` failed to lower: {e}"));
        let report = semantic_ir::validate(&module);
        assert!(report.is_ok(), "`{name}` failed validation: {:?}", report.issues);
    }
}

/// Every refusal entry names a real program.
#[test]
fn every_refusal_names_a_program() {
    for (program, target, _) in REFUSED {
        assert!(
            all_programs().any(|(_, name, _, _)| name == *program),
            "REFUSED lists `{program}` on {}, which is not a program here",
            target.tag()
        );
    }
}

/// Every program prints the reference output on every available backend,
/// or — only where [`REFUSED`] says so — is refused at compile time.
///
/// Every `(program, backend)` cell runs before the test reports, so one
/// failure lists all the cells that disagree.
#[test]
fn programs_print_the_reference_output_on_every_backend() {
    let mut ran = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (frontend, name, source, expected) in all_programs() {
        if let Some(actual) = reference(frontend, source) {
            assert_eq!(actual, expected, "`{name}`: {} prints {actual:?}", frontend.tag());
        }
        for &target in Target::all() {
            let refusal = REFUSED
                .iter()
                .find(|(program, t, _)| *program == name && *t == target);
            let cell = format!("`{name}` on {}", target.tag());
            match (run_source_via(name, source, frontend, target), refusal) {
                (RunOutcome::Skipped(why), Some(_)) if why.contains("does not yet accept") => {
                    eprintln!("refused as expected {cell}: {why}")
                }
                (RunOutcome::Skipped(why), None) if why.contains("does not yet accept") => {
                    wrong.push(format!("{cell}: refused: {why}"))
                }
                (RunOutcome::Skipped(why), _) => eprintln!("skip {cell}: {why}"),
                (RunOutcome::Ran(stdout), Some((_, _, why))) => wrong.push(format!(
                    "{cell} ran (printing {stdout:?}) but REFUSED lists it ({why}): \
                     remove the entry if the output is right"
                )),
                (RunOutcome::Ran(stdout), None) => {
                    if stdout != expected {
                        wrong.push(format!("{cell}: printed {stdout:?}, expected {expected:?}"));
                    }
                    ran += 1;
                }
                (RunOutcome::Failed(msg), _) => {
                    let head: Vec<&str> = msg.lines().take(6).collect();
                    wrong.push(format!("{cell}: failed:\n    {}", head.join("\n    ")));
                }
            }
        }
    }
    eprintln!("ruby-on-sir gaps: {ran} program/backend runs");
    assert!(
        wrong.is_empty(),
        "{} program/backend cells are wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
