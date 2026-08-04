//! The golden conformance matrix.
//!
//! Each [`Program`] in `CORPUS` is real Ruby source paired with the exact
//! stdout a Ruby interpreter would produce. The matrix test lowers every
//! program through the frontend and runs it through **every** backend's real
//! toolchain, asserting the output equals the reference on each — the single
//! test that drives Ruby *source* all the way to Python, JavaScript, Go, and
//! Rust and proves they agree.

use sir_conformance::{run, Program, RunOutcome, Target};

/// The reference corpus. Outputs are intentionally strings and integers only —
/// booleans render differently across backends (`#t` vs `true`), which is a
/// separate formatting concern, not a behavioural one, so we keep the oracle
/// unambiguous.
const CORPUS: &[Program] = &[
    // Operator precedence: `*` binds tighter than `+`.
    Program {
        name: "arithmetic",
        ruby: "puts(2 + 3 * 4)\n",
        expected: "14",
    },
    // A method with parameters, returning its last expression.
    Program {
        name: "def_params",
        ruby: "def add(a, b)\n  a + b\nend\n\nputs add(2, 3)\n",
        expected: "5",
    },
    // Implicit return of a trailing `if` (FC — shipped, proven here end-to-end).
    Program {
        name: "tail_if",
        ruby: "def bigger(a, b)\n  if a > b\n    a\n  else\n    b\n  end\nend\n\nputs bigger(10, 7)\n",
        expected: "10",
    },
    // Implicit return of a trailing `case` (relies on the `case_eq` builtin
    // that had been missing from Go/Rust/JS — this is the regression oracle for
    // that whole class of bug).
    Program {
        name: "tail_case",
        ruby: "def grade(n)\n  case n\n  when 90\n    \"A\"\n  when 80\n    \"B\"\n  else\n    \"C\"\n  end\nend\n\nputs grade(90)\nputs grade(80)\nputs grade(50)\n",
        expected: "A\nB\nC",
    },
    // String concatenation (polymorphic `+`).
    Program {
        name: "string_concat",
        ruby: "puts(\"ab\" + \"cd\")\n",
        expected: "abcd",
    },
    // A user-defined class, `.new`, and an instance method call.
    Program {
        name: "oop_method",
        ruby: "class Dog\n  def speak\n    \"woof\"\n  end\nend\n\nputs Dog.new.speak\n",
        expected: "woof",
    },
    // A `while` loop with a mutable accumulator: 0+1+2+3+4.
    Program {
        name: "while_loop",
        ruby: "i = 0\nsum = 0\nwhile i < 5\n  sum = sum + i\n  i = i + 1\nend\n\nputs sum\n",
        expected: "10",
    },
    // Array literal + `.length` (a method call, which lowers cleanly).
    //
    // UPDATE: the frontend gaps that used to block index *reads* (`a[1]`,
    // `h["k"]`) are now FIXED — `a[1]`/`h["k"] = v` parse and lower correctly
    // on every backend (PR #9686, both the grammar and the shared
    // `__method__("[]"/"[]=", ...)` lowering). Still NOT in this corpus,
    // though, for a DIFFERENT reason: that lowering only has a runtime
    // dispatch implementation on the C backend so far — Python/JS/Go/Rust's
    // OOP runtime catalogs have no `[]`/`[]=` entries yet, so a bracket-index
    // program fails at runtime (not a skip) on those four. Tracked as its own
    // follow-up ("Python/JS/Go/Rust backends: implement []/[]= bracket-index
    // runtime dispatch"); adding it here now would break the matrix rather
    // than prove anything. `.length` covers array construction end-to-end
    // without tripping the index path.
    Program {
        name: "array_length",
        ruby: "a = [10, 20, 30]\nputs a.length\n",
        expected: "3",
    },
    // `puts` on an Array UNPACKS it one element per line, recursively
    // flattening nested arrays (real Ruby's `Kernel#puts` rule) — a
    // core/always-available builtin, not a Collections method, so this
    // exercises `puts` itself, not method dispatch. Previously the C and
    // Ruby backends each bracket-displayed an Array argument instead
    // (their own separate reimplementations of `puts`/`sir_fmt`, neither
    // delegating to a native array-aware `puts`); both fixed together.
    Program {
        name: "puts_array_unpack",
        ruby: "puts [1, 2, 3]\nputs [4, [5, 6], 7]\n",
        expected: "1\n2\n3\n4\n5\n6\n7",
    },
    // String `.length` (a method on a String receiver). NOTE: `.upcase` /
    // `.downcase` are deliberately excluded for now — they expose a real
    // JavaScript-backend gap: that backend translates Ruby method names to
    // native JS names *at emit time* (e.g. `upcase` → `toUpperCase`), and the
    // rename table is missing the case-conversion pair, so `"x".upcase` raises
    // `NoMethodError` on JS while Python/Go/Rust (which dispatch Ruby names in a
    // runtime catalog) handle it. Tracked for a separate focused fix; see
    // `lessons.md`. `.length` is spelled identically across all four, so it
    // exercises string-method dispatch end-to-end without tripping the gap.
    Program {
        name: "string_length",
        ruby: "puts \"hello\".length\n",
        expected: "5",
    },
    // Instance state via `@ivar` mutated across method calls.
    Program {
        name: "counter_state",
        ruby: "class Counter\n  def initialize\n    @n = 0\n  end\n  def inc\n    @n = @n + 1\n  end\n  def value\n    @n\n  end\nend\n\nc = Counter.new\nc.inc\nc.inc\nputs c.value\n",
        expected: "2",
    },
    // A module mixed into a class with `include`.
    Program {
        name: "mixin_include",
        ruby: "module Greet\n  def hi\n    \"hi\"\n  end\nend\n\nclass P\n  include Greet\nend\n\nputs P.new.hi\n",
        expected: "hi",
    },
    // Short-circuit `||` / `&&` returning the deciding OPERAND (Ruby semantics),
    // exercising both a truthy and a falsy left-hand side. `"a" || "b"` → "a";
    // `nil || "b"` → "b"; `"x" && "y"` → "y". Previously these lowered to a
    // `BuiltinCall("or"/"and")` that Go/Rust/JS emitters didn't handle, so any
    // `||`/`&&` threw `unknown builtin` at runtime on three of five backends.
    Program {
        name: "logical_ops",
        ruby: "puts(\"a\" || \"b\")\nputs(nil || \"b\")\nputs(\"x\" && \"y\")\n",
        expected: "a\nb\ny",
    },
    // A `case` with a multi-value `when` (`when 1, 2, 3`), which folds through
    // the same `or` builtin. Re-enabled now that `or`/`and` work on all backends.
    Program {
        name: "multi_when",
        ruby: "def sz(n)\n  case n\n  when 1, 2, 3\n    \"small\"\n  else\n    \"big\"\n  end\nend\n\nputs sz(2)\nputs sz(9)\n",
        expected: "small\nbig",
    },
    // Ruby String methods whose names differ from JS natives (`upcase` →
    // `toUpperCase`, `downcase` → `toLowerCase`, `strip` → `trim`). Python/Go/Rust
    // dispatch Ruby names in a runtime catalog; the JS backend renames Ruby names
    // to native JS — previously the case/strip renames were missing, so these
    // raised `NoMethodError` on JS only.
    Program {
        name: "string_case",
        ruby: "puts(\"hello\".upcase)\nputs(\"WORLD\".downcase)\nputs(\"  hi  \".strip)\n",
        expected: "HELLO\nworld\nhi",
    },
    // Sequential local assignments where a later binding READS an earlier one
    // (`b = a + 1`, `c = b + a`). Ruby is sequential (`let*`); the frontend
    // previously lowered these to parallel `LetBinding`s, which the SIR
    // validator rejected ("var-ref ... unknown name `a`") — so `newvar =
    // existing_local` failed to compile on every backend. Now fixed.
    Program {
        name: "seq_assign",
        ruby: "a = 5\nb = a + 1\nc = b + a\nputs a\nputs b\nputs c\n",
        expected: "5\n6\n11",
    },
    // ── Collections cascade (C-backend slices 3-10) ──────────────────────
    //
    // The programs below all print SCALARS (never a bare Array/Hash), so
    // they sidestep a separate, real display-convention bug this batch
    // uncovered: `puts` on an Array bracket-displays it on the C backend
    // (`[1, 2, 3]`) but correctly unpacks one element per line on
    // Python/JS/Go/Rust, matching real Ruby (`puts [1,2,3]` → "1\n2\n3\n").
    // Tracked as its own follow-up ("C backend: puts on an Array should
    // unpack one-per-line") rather than fixed here — this corpus addition's
    // job is proving the METHOD CATALOG agrees, not the display layer.
    //
    // A block method (`reduce`), proving closures round-trip identically
    // through every backend's calling convention.
    Program {
        name: "array_reduce",
        ruby: "puts [1, 2, 3, 4].reduce { |acc, x| acc + x }\n",
        expected: "10",
    },
    // Two Array 0-arg query methods (slice 3).
    Program {
        name: "array_count_sum",
        ruby: "puts [1, 2, 3, 4, 5].count\nputs [1, 2, 3].sum\n",
        expected: "5\n6",
    },
    // Hash non-block methods (slice 6): `.length` and `.fetch`.
    Program {
        name: "hash_length_fetch",
        ruby: "h = {\"a\" => 1, \"b\" => 2}\nputs h.length\nputs h.fetch(\"a\")\n",
        expected: "2\n1",
    },
    // Remaining String methods (slice 8): literal `sub`/`gsub`.
    Program {
        name: "string_gsub_sub",
        ruby: "puts \"aaa\".gsub(\"a\", \"b\")\nputs \"aaa\".sub(\"a\", \"b\")\n",
        expected: "bbb\nbaa",
    },
    // Numeric methods (slice 9): `abs` and `gcd`.
    Program {
        name: "numeric_abs_gcd",
        ruby: "puts((-5).abs)\nputs 12.gcd(18)\n",
        expected: "5\n6",
    },
    // Symbol methods (slice 10): `upcase` widened from the String helper,
    // `length` widened the same way.
    Program {
        name: "symbol_upcase_length",
        ruby: "puts :hello.upcase\nputs :hello.length\n",
        expected: "HELLO\n5",
    },
];

/// Every program must produce its reference output on every available backend.
/// A backend whose toolchain is absent is skipped (logged), never failed; a
/// mismatch or a crash is a hard failure naming the exact `(program, backend)`.
#[test]
fn corpus_agrees_across_all_backends() {
    let mut ran = 0usize;
    let mut skipped = 0usize;

    for program in CORPUS {
        for &target in Target::all() {
            match run(program, target) {
                RunOutcome::Ran(stdout) => {
                    assert_eq!(
                        stdout, program.expected,
                        "\nCONFORMANCE MISMATCH\n  program:  {}\n  backend:  {}\n  expected: {:?}\n  actual:   {:?}\n",
                        program.name,
                        target.tag(),
                        program.expected,
                        stdout,
                    );
                    ran += 1;
                }
                RunOutcome::Skipped(why) => {
                    eprintln!("skip {}/{}: {why}", program.name, target.tag());
                    skipped += 1;
                }
                RunOutcome::Failed(msg) => {
                    panic!(
                        "\nCONFORMANCE FAILURE\n  program: {}\n  backend: {}\n  {msg}\n",
                        program.name,
                        target.tag(),
                    );
                }
            }
        }
    }

    eprintln!(
        "conformance matrix: {} corpus x {} backends = {ran} ran, {skipped} skipped",
        CORPUS.len(),
        Target::all().len(),
    );
    // The harness is worthless if it proved nothing; require at least one real
    // run so a fully-toolchain-less CI box can't report a hollow pass.
    assert!(
        ran > 0,
        "no backend toolchain was available — conformance proved nothing"
    );
}

/// Sanity: every corpus program lowers through the frontend (independent of any
/// backend toolchain), so a parse/lower regression is caught even on a host
/// with no toolchains at all.
#[test]
fn corpus_all_lowers() {
    for program in CORPUS {
        sir_conformance::lower(program)
            .unwrap_or_else(|e| panic!("`{}` failed to lower: {e}", program.name));
    }
}
