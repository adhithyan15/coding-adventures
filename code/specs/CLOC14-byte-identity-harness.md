# CLOC14 — End-to-end byte-identity test harness

**Status:** v0.3 manifest design selected (CCR-003). The harness now discovers
**462 `minify_*` fixtures**, while `tests/diff/` contains 626 differential
fixture directories in total. The checked-in bytes pass, but their provenance
is fragmented across README prose and multiple Closure releases. v0.3 makes
that provenance machine-verifiable before CCR-004 refreshes the 462 goldens.
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

The seed fixtures were initially **hand-traced**. **CLOC14.1 captured real upstream goldens** by downloading `closure-compiler-v20240317.jar` from Maven Central and running it against each fixture's `flags.txt` + `input/`. The three originally-PASS hand-traced goldens were confirmed byte-identical to upstream — the constrained inputs (`var x=1;`, `var x="hi";`, `var x=null;var y=1;`) really are unambiguous under WHITESPACE_ONLY. Their READMEs now document the real capture details and removed the "hand-traced" caveat.

The `minify_empty` fixture had been **IGNORED** specifically because the empty-input trailing-byte behaviour of upstream Closure was unknown. CLOC14.1's capture run resolved it: upstream emits a single `\n` (0x0a) byte, exactly what closurec emits. `minify_empty` flipped from IGNORED to PASS and the entry was removed from `IGNORE_FIXTURES`.

README prose remains useful explanation, but it is not an enforceable registry.
The 2026-09-19 inventory found 626 fixture directories, 546 READMEs, 466
READMEs with a release token, and only 22 with a `java -jar` command. Eight of
the 462 minify fixtures have no version token. Existing mentions span at least
`v20240317`, `v20260712`, and `v20260915`.

### 4.1 Canonical oracle identity

`tests/oracle/manifest.json` is the v0.3 source of truth. Schema version 1 pins:

- project and Apache-2.0 source/license URLs;
- release `v20260915`, annotated tag object
  `72421c28d352e5dda9a111bec39c3d41af46f3a3`, and dereferenced release commit
  `56007b2869ef6ce70b659b033459b8d8113101de`;
- Maven coordinates `com.google.javascript:closure-compiler:v20260915`, the
  canonical artifact URL, byte length 14,976,538, and SHA-256
  `9C8AF06056AA06F968B5A457540A85869C7BA2861C211C56D8D4EF6C35DDF36D`;
- capture runtime Java 21.0.12 and the artifact's embedded JDK 21 marker;
- the separately audited upstream `master` commit
  `10ca677aff381d2c2e6e1b254ba32861e503173d`; and
- locale, timezone, encoding, working-directory, input, output, and command
  template assumptions needed for deterministic captures.

The release tag, release source commit, binary artifact, and later audited
`master` commit are distinct fields. The manifest must never imply that the
post-release audit commit produced the released JAR.

### 4.2 Fixture dispositions

Every immediate directory under `tests/diff/` appears exactly once in a
manifest fixture set. Sets use explicit fixture-name arrays; filename globs are
not provenance because a newly added directory must fail verification until a
reviewer classifies it.

Each set has one disposition:

1. `upstream_golden`: bytes or diagnostics captured from the pinned release;
   requires exactly one command template and artifact pin.
2. `mixed_contract`: combines an upstream observation with a deliberately
   different local fail-closed or unsupported behavior; requires both the
   upstream command and a written local boundary.
3. `local_extension`: exercises correlation-vector or other closurec-only
   behavior with no upstream equivalent; requires a reason and local harness,
   and must not claim an upstream golden.
4. `local_contract`: tests local packaging/help/version or another contract
   whose expected bytes were not captured upstream; requires a reason and
   local harness.

The 462 `minify_*` fixtures form one explicit `upstream_golden` cohort and are
the only automatic input to CCR-004. The remaining 164 directories are still
classified by v0.3, but classification does not manufacture an upstream
command for a local-only extension.

### 4.3 Offline verifier

`tests/oracle_manifest.rs` parses the manifest with strict Serde structures
that reject unknown fields. It collects all violations before failing so a
stale manifest produces one actionable report. Without network access or an
oracle JAR, it verifies:

- the exact supported schema version and immutable pin/hash shapes;
- uniqueness of command, fixture-set, and fixture identities;
- every `tests/diff/` directory is classified exactly once and every declared
  fixture exists;
- disposition-specific required and forbidden fields;
- all referenced `flags.txt`, inputs, expected outputs, harnesses, and command
  placeholders resolve inside the package tree; and
- the explicit minify cohort equals runtime discovery by
  `tests/diff_minify.rs`.

Tests construct malformed manifests for unknown fields, unsupported schema
versions, invalid hashes, duplicate or missing mappings, stale paths,
ambiguous commands, and disposition violations. CI never downloads or
executes the upstream artifact; acquisition and hash verification are an
explicit maintainer regeneration step.

## 5. The seed fixture set (v0.1)

| Fixture | Status | What it pins |
|---|---|---|
| `minify_minimal_var` | PASS | A single `var x=1;` round-trips verbatim under WHITESPACE_ONLY. Pins the trailing-newline contract and lex/parse/emit identity. |
| `minify_string_literal` | PASS | A `"hi"` string literal preserves its quote style and content. Catches quote-flip / escape-double regressions. |
| `minify_two_statements` | PASS | Two consecutive top-level statements emit on a single line with no inserted separator. Catches statement-separator drift. |
| `minify_empty` | PASS | Empty input round-trips to a single `\n` byte. CLOC14.1 captured upstream — emits `\n`, same as closurec; flipped from IGNORED to PASS. |

## 6. The PR cadence going forward

1. **A gap-fix PR that thinks it improved upstream parity should also add (or un-ignore) a minify fixture.** That makes the parity gain measurable.
2. **A failing fixture should be removed from IGNORE_FIXTURES the moment the corresponding gap closes.** The IGNORE list is intentionally an embarrassment that should shrink over time.
3. **Capturing real upstream goldens** is CCR-004. It must use the verified
   v0.3 manifest and publish equal/changed/declined counts; no README-only or
   hand-traced provenance is sufficient.

## 7. What this harness does NOT cover (yet)

- **`--js_output_file`** writes — the runner currently captures stdout, not files. A future variant should support file-output fixtures.
- **Source maps** — comparing `.map` outputs byte-for-byte requires special handling (mappings are order-sensitive; source ordering must match).
- **`SIMPLE_OPTIMIZATIONS` / `ADVANCED_OPTIMIZATIONS`** — until the CLOC13 apply steps for inline/rename/collapse-properties ship, SIMPLE-level fixtures will diverge. WHITESPACE_ONLY is the right starting compilation level.
- **Multi-file inputs** — supported by the harness (flags.txt can list multiple `--js`) but no seed fixture exercises it yet.

## 8. References

- Source: `code/programs/rust/closurec/tests/diff_minify.rs`
- Fixtures: `code/programs/rust/closurec/tests/diff/minify_*/`
- Related: CLOC11 (CLI compat), CLOC12 (upstream test ports + gap tracker)
- Oracle manifest: `code/programs/rust/closurec/tests/oracle/manifest.json`
- Manifest verifier: `code/programs/rust/closurec/tests/oracle_manifest.rs`
- Tracking: CCR-003 / <https://github.com/adhithyan15/coding-adventures/issues/15549>
