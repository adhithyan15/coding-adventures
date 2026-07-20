# ADJ-TABLES — native tabular data as a first-class language construct (RS-5)

**Status:** Spec-first. Introduces the `table` construct, a sibling of `dictionary`,
`rulebook`, and `formulabook`. Companion to [ADJ-FORMULA-LIBRARIES](ADJ-FORMULA-LIBRARIES.md)
(the *formula* standard library) and the relational-recall substrate
([REL-1](REL-1-RELATIONAL-RECALL.md)). Closes the long-open gap noted in
[ADJ14](ADJ14-rule-elicitation.md) §"explicit table support remains … open."
**Author:** RS-5 direction pass, 2026-07-13.

---

## 1. Why tabular data needs to be first-class

Real reference knowledge is overwhelmingly **tabular**, not prose:

- **Unit conversions** — NIST publishes "1 inch = 2.54 cm" in a *table*, not a sentence.
- **Reference ranges** — normal serum sodium is 135–145 mEq/L (a labelled interval).
- **Dose-by-weight charts** — paediatric dosing is a weight → dose table.
- **Tax / tariff brackets** — a step function over income breakpoints.
- **Nomograms / calibration curves** — a sampled function you interpolate between.
- **Defining constants** — a fixed table of exact numeric values.

Today ADJ has no way to hold a table. Each of these must be hand-expanded into individual
`relate`/`observe` facts, with the provenance envelope **repeated per row**, and the
range/interpolated cases (brackets, nomograms) are **not expressible at all**.

### 1.1 The grounding motivation (the concrete trigger)

The ADJ grounding discipline (see [feedback: nothing human-authored]) requires every shipped
fact to be defended by a **byte-quotable span** from a citable source. When the source *is a
published table*, there is no prose sentence naming both the entity and its value — a cell
like `kilo | k | 10³` is not a sentence. Forcing tabular data through a sentence-shaped hole
means hunting for rare prose restatements and fighting numeric-rendering fragility
(superscripts, thousands separators). That friction is the missing-substrate symptom.

**The resolution:** when the source is a table, **the citable artifact is the table itself at
its locator.** A `table` ingests the published table verbatim as rows and cites the page
once; every lookup answer is then auditable back to that table — the faithfulness bar is met
without a reconstructed sentence. Tabular data gets a *faithful home*, and grounding tabular
reference data (unit conversions, reference ranges, constants) becomes clean.

---

## 2. Surface syntax

A `table` is a top-level, importable construct. Its keywords (`table`, `columns`, `row`) are
IDENT-matched literals — exactly like `formulabook`/`rulebook`/`define` — so no new lexer
tokens are introduced.

```
table unit_conversions {
    columns unit, centimetres          % ordered column names (arity of every row)
    row (inch, 2.54)                   % atoms and exact numbers, one per column
    row (foot, 30.48)
    row (yard, 91.44)
    source  "General Tables of Units of Measurement …"   % the published table
    locator "https://www.nist.gov/…"                     % where it was read
    trust   authoritative
}
```

Grammar (added to the `statement` alternation; see [ADJ01 grammar](ADJ01-adjudication-ir-grammar.md)):

```
table_decl   = "table" IDENT LBRACE { use_decl } columns_decl { table_row } { annotation } RBRACE ;
columns_decl = "columns" IDENT { COMMA IDENT } ;
table_row    = "row" LPAREN row_item { COMMA row_item } RPAREN [ LBRACE { annotation } RBRACE ] ;
row_item     = NUMBER | IDENT | STRING ;
```

**Per-row provenance (RS-5e).** A row may carry its **own** `{ … }` annotation block, which
overrides the table envelope *field by field* for that row — so a row can supply just the span
that defends **it** and inherit the shared `locator`/`trust`:

```
table air_quality_index {
    columns min_aqi, category
    row (0,   good)     { source "Green   Good   0 to 50" }
    row (51,  moderate) { source "Yellow   Moderate   51 to 100" }
    source  "The AQI includes six color-coded categories, each corresponding to a range of index values."
    locator "https://www.airnow.gov/aqi/aqi-basics/"
    trust   authoritative
}
```

The block is **braced deliberately**: a table's envelope is conventionally written *after* its
rows, so a bare trailing annotation would be ambiguous — the parser could not tell whether it
belonged to the last row or to the table. `LBRACE` disambiguates, and a row with no block behaves
exactly as before (inherits the whole envelope), so every existing table is unchanged.

- **`use <dict>`** (optional): vocabulary-check column entities against a `dictionary`, as a
  `rulebook`/`formulabook` does.
- **`columns`**: the ordered column names; also fixes the row **arity** (rows with a
  different count are a compile error — `TableArity`).
- **`row (…)`**: one row; items are exact numbers or atoms, positionally bound to columns.
- **`{ annotation }`**: the shared `source "…" locator "…" trust <tier>` provenance envelope
  (identical to `relate`/`formula`). Attached to the whole table; a shipped table must carry
  a non-empty `source`. (Per-row provenance is a documented future extension; §6.)

---

## 3. Semantics — rows are relations

A `table T { columns c1,…,cn; row (v1,…,vn) … }` lowers so that **each row becomes a ground
relation** `T(v1,…,vn)`, carrying the table's provenance — byte-for-byte the same lowering as
a `relate T(v1,…,vn)` edge. This is the whole point: **exact lookup reuses the existing SLD
resolver**, with zero new engine machinery.

### 3.1 Exact lookup (this deliverable)

A binding query over the table relation *is* an exact lookup:

```
? unit_conversions(inch, $cm)     % binds $cm = 2.54, cited to the unit_conversions table
```

- A hit returns the bound value **with the table's citation** (via the proof's `via_facts`).
- A miss returns `"abstained": true` — the honest "not in the table" outcome, exactly as
  relational recall abstains on an absent edge.
- Because a one-key→one-value row is a relation, a looked-up **number can feed a `let` /
  `formula`** through the existing slot/`Ref` resolution — a table value composes into
  downstream arithmetic with no special case.

### 3.2 Range / bracket lookup (RS-5c)

For step functions (tax brackets, dose bands, reference-range classification), rows define
**breakpoints**: the lookup selects the row whose key is the greatest key `≤` the query
(equivalently, the interval `[key_i, key_{i+1})` the query falls in). This reuses the engine's
existing exact comparators `CmpOp {Ge,Le,Gt,Lt,Eq}` on the exact `BigRational` path — no new
number or engine machinery. The final call-site form is a `?`-prefixed recall that names the
table, the key column bound to a concrete value, the mode, and the value column to return:

```
? lookup bmi_categories min_bmi = 27.3 mode range give category      % overweight
```

- **`lookup`/`mode`/`give`/`range`** are IDENT-matched literals (no new lexer tokens); the form
  is folded into `query_decl` (`QUESTION ( lookup_expr | term )`) so it coexists with the exact
  binding query.
- The table declaration is **unchanged** — a `range` table is an ordinary table read
  differently. The key column must be numeric (checked at lower time: `LookupNonNumericKeyColumn`);
  an unknown table or column is `LookupUnknownTable` / `LookupUnknownColumn`; `mode interpolated`
  is reserved for RS-5d (`LookupModeUnsupported`).
- A hit returns the value column **with the selected breakpoint row's citation** (the same
  `via_facts → provenance` flow as exact lookup) and records the matched key in the audit, so the
  answer names *which* bracket it fell in. A query **below the smallest key** has no key `≤` it
  and honestly **abstains** — "below the table's domain", not a fabricated classification.

### 3.3 Interpolated lookup (spec'd here, built in a follow-up)

For sampled continuous functions (nomograms, calibration curves), the lookup **linearly
interpolates** between the two rows that bracket the query key, computing on the exact
`BigRational` arithmetic already in the compute evaluator. No interpolation code exists in the
engine today; this tactic is genuinely new. Interpolation is only defined for numeric key and
value columns; a non-numeric column is a compile/lookup error.

---

## 4. Provenance & auditability

- One `source`/`locator`/`trust` envelope per table, folded through the *same*
  `annotations_to_provenance` surface used by `relate`, `rule`, `prior`, `contributes`, and
  `formula`. Trust tiers are the existing five (`consensus`/`authoritative`/`empirical`/
  `inferred`/`unattributed`).
- **Per-row override (RS-5e).** A row's own `{ … }` block overrides the envelope *field by field*
  for that row, so the answer cites **the span that defends the row actually selected** — not the
  table's first sentence. This closes a real accounting gap: a six-band table with one envelope
  made every answer, in every band, cite the *same* sentence. A range lookup made it glaring,
  because the selected row is explicit in the audit; but it affected every multi-row table.
  Mechanically it is nearly free — each row already lowers to its **own** `Fact`, and every
  citation path (exact recall, range lookup, the proof DAG's `via_facts`) already cites *the fact
  that produced the answer*. Giving that fact the row's provenance is the whole fix; no renderer
  changes.
- This is what makes the audit trail honest at the table level: *every asserted fact quotes the
  byte span that supports **it***. A row without a block still inherits the envelope, so tables
  authored before RS-5e keep working unchanged.
- **How a table appears in the audit trail (RS-4).** `ADJ-REASON-MATH.md` §E — the normative
  audit-trail contract — gives tables two of its step kinds: `FromTableRow { table, row_index }`
  for an exact hit and `FromRangeBracket { table, key, matched_key, mode }` for a bracket select.
  `matched_key` is recorded explicitly because *which breakpoint you landed on* is the entire
  content of a bracket decision, and RS-5e is what lets that step quote a span defending the row
  it names. Abstention below a table's floor is likewise a typed reason there
  (`BelowTableDomain { table, key, min_key }`), distinct from a malformed key
  (`NonNumericKey`) — the two are opposite failures and must never render alike.
- Every lookup answer (exact, range, or interpolated) carries the table's citation. For
  interpolated answers the derivation records the two bracketing rows it combined — the answer
  remains auditable to the source rows, honouring *hallucination is an accounting failure;
  every step is auditable.*

---

## 5. Worked example — the NIST unit-conversion table (Facts front)

`code/specs/data/adj-formula-stdlib/reference/unit-conversions.adj` ships a real,
NIST-cited exact-conversion table; `unit-conversions.query.adj` is the worked recall:

```
import "reference/unit-conversions.adj"
? unit_conversions(inch, $cm)      % 2.54
? unit_conversions(foot, $cm)      % 30.48
? unit_conversions(fathom, $cm)    % abstains — not in the table
```

This is exactly the artifact the Facts front was blocked on: instead of a fragile prose span
per conversion, one table cites the NIST page once and every conversion is auditable to it.

---

## 6. Scope & staging

| stage | what | status |
|-------|------|--------|
| RS-5a | this spec | **this PR** |
| RS-5b | grammar + AST + adapter + lower; rows→relations; **exact lookup** e2e; shipped NIST table | **this PR** |
| RS-5c | **range/bracket** lookup tactic (reuses the exact `BigRational` order) + e2e (inline BMI bands) | shipped |
| RS-5e | **per-row provenance** — a row's `{ … }` block overrides the envelope; the answer cites the SELECTED row's span | **this PR** |
| RS-5d | **interpolated** lookup tactic (new, on `BigRational`) + e2e (nomogram) | follow-up |

**Explicitly deferred:** multi-key composite lookup beyond positional binding; typed/dimensioned
columns (columns are untyped atoms/numbers today). These are additive and do not change the
row-as-relation core. (*Per-row provenance was deferred at RS-5b and is now delivered by RS-5e.*)

---

## 7. Non-goals

`table` is **not** a spreadsheet, a SQL engine, or a dataframe. It is a small, importable,
provenanced, *finite* relation with lookup semantics — the honest home for published reference
tables in a language whose contract is that every answer is computed on the CPU and cited.
