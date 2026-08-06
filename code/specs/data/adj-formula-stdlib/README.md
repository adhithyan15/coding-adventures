# adj-formula-stdlib

The **compute** half of the ADJ curriculum standard library — see
[`ADJ-FORMULA-LIBRARIES.md`](../../ADJ-FORMULA-LIBRARIES.md) for the full spec. The
**knowledge** half (grounded facts, relations) lives in the sibling `adj-facts-stdlib`
directory. Together they are the importable stdlib a small model draws on: a curriculum
question is answered by *recalling* which library applies, *decomposing* the input with
byte-provenance, and letting the ADJ engine *compute* the answer on the CPU — zero
arithmetic performed by the model.

## What's here

Every content library is a **pair** of files:

- `<name>.adj` — a `formulabook` (or `table`) declaring one or more provenanced formulas.
  Every `formula`/`table` clause carries a `source`/`locator`/`trust` provenance envelope
  (required — the linter rejects an unsourced one); a fully byte-pinned clause additionally
  carries a `quote "..." at N snapshot "<sha256>"` (see `adj-stdlib-provenance/README.md` for
  that migration — most shipped libraries are `source_labeled`, not yet byte-pinned).
- `<name>.query.adj` — a worked example: `import`s the library, `observe`s inputs, and
  `?`-queries the formula(s), demonstrating the *decompose → bind → import → compute*
  consumer surface a real answer follows.

Libraries are organized by domain, matching the curriculum's math/science tracks:

| Directory | Contents |
|---|---|
| `arithmetic/` | The elementary primitives (`sum`, `difference`, `product`, `quotient`, `ratio`, `percent`, `average`, …) every higher formula composes — write-once, use-many. |
| `mathematics/` | Non-arithmetic-primitive math: geometry (area/perimeter), and the K-2 foundational trio (counting sequence, comparison, cardinality). |
| `physics/`, `chemistry/`, `metrology/` | Domain formulas (kinematics, gas laws, temperature, …). |
| `reference/` | NIST/SI unit-conversion tables (`table`, not `formula` — a published conversion factor is looked up, not computed). |
| `clinical/` | Medical/pharmacology calculators (BMI, Cockcroft-Gault, Apgar score, …) — the MLE-apex layer; composes the layers below it. |

A library may `import` another **within this tree**, including across domain directories
(e.g. `clinical/cockcroft_gault.adj` and `mathematics/cardinality.adj` both import
`../arithmetic/arithmetic.adj`). The CLI's import sandbox resolves relative to the **entry
program's own directory** — a consumer that imports a cross-directory library must itself sit
at a common ancestor (see any `<name>.query.adj` whose header comment says "stays at the
stdlib root"), not next to the library it's demonstrating.

## Curriculum mapping

A library existing here does not by itself mean it's part of the tracked curriculum — see
[`code/specs/data/adj-stdlib-coverage/manifest.json`](../adj-stdlib-coverage/manifest.json)
for the objective-to-library crosswalk (band, domain, competency, prerequisites,
CCSS/NGSS/USMLE coverage root) and
[`ADJ-STDLIB-COVERAGE.md`](../../ADJ-STDLIB-COVERAGE.md) for the gap ledger and delivery
order this directory is built against.

## Verification

- `python code/scripts/adj_stdlib_report.py --format json --fail-on-unreferenced-tests
  --formula-inventory-binary <bin> --formula-audit-binary <bin>` — structural completeness
  (every library has a query companion, a source envelope, and a repository test that names
  it).
- `python code/scripts/adj_stdlib_manifest.py --validate-json-schema ...` — the curriculum
  manifest is internally consistent (every listed library path exists, no cyclic
  prerequisites, `fully_verified` objectives actually resolve in the CAS).
- `cargo test -p adj-lang-cli` — end-to-end: each library's `.query.adj` companion actually
  parses and computes the expected value through the real CLI (see
  `code/packages/rust/adj-lang-cli/tests/formula_*_e2e.rs` for the per-library test pattern).
- Full byte-pin verification (`adj-verify --snapshots`) is a separate, Linux-only track — see
  `code/specs/data/adj-stdlib-provenance/README.md`.
