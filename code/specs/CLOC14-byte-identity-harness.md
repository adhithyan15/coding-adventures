# CLOC14 — End-to-end byte-identity test harness

**Status:** v0.1 shipped. Harness + 3 PASS + 1 IGNORED seed fixtures.
**Layer:** Above CLOC11 (CLI compat) and CLOC12 (upstream test ports), below CLOC15+ (whatever comes next).
**Depends on:** closurec CLI being runnable end-to-end.
**Unblocks:** Every future gap-fix can be *measured* against upstream Closure's output instead of unit-tested in isolation.

---

## 1. Why CLOC14 exists

The closurec termination condition is "drop-in binary-compatible". That's a **behavioural** property — the same input + same flags should produce the same output as Google Closure Compiler. Until CLOC14, every gap-fix PR was theoretical: we'd patch a constant-fold rule, the unit tests would go green, but we had no measurement of whether the *composed* compiler's output actually matched upstream.

Without an end-to-end byte-comparison test, individually-correct passes can compose into a diverging compiler — and we wouldn't know until a user runs both and notices a diff.

CLOC14 is the missing instrument. Every PR from here forward can be measured against it: "did this gap-fix flip a fixture from failing to passing?"

## 2. Design

```
tests/
└── diff/
    ├── minify_<name>/          ← per-fixture directory, name starts with `minify_`
    │   ├── flags.txt            ← one CLI flag per line; comments via `#`
    │   ├── input/               ← input files referenced by flags.txt
    │   │   └── a.js
    │   ├── expected.stdout      ← golden captured from upstream Closure
    │   └── README.md            ← what this fixture pins + capture provenance
    └── ...
```

A single test runner at `tests/diff_minify.rs`:

1. Walks `tests/diff/` at *test time* (not compile time) and collects every directory whose name starts with `minify_`.
2. For each fixture: reads `flags.txt`, execs `closurec` with those flags, captures stdout, compares against `expected.stdout`.
3. Collects per-fixture verdicts (`Match` / `Diverge` / `Error` / `Skipped`) and panics with a single multi-fixture failure report if any non-ignored fixture diverged.

The runner is one Rust file. Adding a new fixture requires zero source-code changes — just create the directory.

## 3. The IGNORE_FIXTURES list

A fixture can be intentionally left in a failing state, pinned to upstream behaviour we know we don't yet match. The runner skips it (reporting `SKIP` instead of `FAIL`) when listed in `tests/diff_minify.rs::IGNORE_FIXTURES`. Each entry includes a reason.

This is the same pattern CLOC12 uses for unported upstream tests: visible in test output, documented in code, removable in a follow-up PR that closes the gap.

## 4. Golden provenance

Each fixture's `README.md` documents:

1. The Google Closure Compiler version that produced the golden (e.g. `v20240317`).
2. The exact command line used to capture the golden.
3. Any caveats — e.g. "trailing-newline behaviour assumed; replace with a fresh capture once available".

The seed fixtures shipped in v0.1 are **hand-traced**, not captured from a running upstream JAR — the inputs are constrained enough (`var x=1;`, `var x="hi";`, `var x=null;var y=1;`) that upstream's WHITESPACE_ONLY output is unambiguous. A future PR should replace each hand-traced golden with a real capture and remove the "hand-traced" caveat from the README.

The `minify_empty` fixture is shipped as **IGNORED** specifically because the empty-input trailing-byte behaviour of upstream Closure is *not* unambiguous — we don't know without running it whether upstream emits `\n` or zero bytes. closurec today emits `\n`. The first capture run should resolve this.

## 5. The seed fixture set (v0.1)

| Fixture | Status | What it pins |
|---|---|---|
| `minify_minimal_var` | PASS | A single `var x=1;` round-trips verbatim under WHITESPACE_ONLY. Pins the trailing-newline contract and lex/parse/emit identity. |
| `minify_string_literal` | PASS | A `"hi"` string literal preserves its quote style and content. Catches quote-flip / escape-double regressions. |
| `minify_two_statements` | PASS | Two consecutive top-level statements emit on a single line with no inserted separator. Catches statement-separator drift. |
| `minify_empty` | IGNORED | Empty input. Pending captured upstream golden to confirm whether upstream emits `\n` or zero bytes. |

## 6. The PR cadence going forward

1. **A gap-fix PR that thinks it improved upstream parity should also add (or un-ignore) a minify fixture.** That makes the parity gain measurable.
2. **A failing fixture should be removed from IGNORE_FIXTURES the moment the corresponding gap closes.** The IGNORE list is intentionally an embarrassment that should shrink over time.
3. **Capturing real upstream goldens** is its own follow-up workstream: someone with a working Closure Compiler installation should script the capture across the seed set + every new fixture.

## 7. What this harness does NOT cover (yet)

- **`--js_output_file`** writes — the runner currently captures stdout, not files. A future variant should support file-output fixtures.
- **Source maps** — comparing `.map` outputs byte-for-byte requires special handling (mappings are order-sensitive; source ordering must match).
- **`SIMPLE_OPTIMIZATIONS` / `ADVANCED_OPTIMIZATIONS`** — until the CLOC13 apply steps for inline/rename/collapse-properties ship, SIMPLE-level fixtures will diverge. WHITESPACE_ONLY is the right starting compilation level.
- **Multi-file inputs** — supported by the harness (flags.txt can list multiple `--js`) but no seed fixture exercises it yet.

## 8. References

- Source: `code/programs/rust/closurec/tests/diff_minify.rs`
- Fixtures: `code/programs/rust/closurec/tests/diff/minify_*/`
- Related: CLOC11 (CLI compat), CLOC12 (upstream test ports + gap tracker)
