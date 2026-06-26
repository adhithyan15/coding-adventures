# VisiCalc multi-sheet workbooks + cross-sheet references

Status: **draft** (spec-first; no implementation yet)
Scope: the next cross-cutting feature after find/replace, rolled out the same way
— engine first, then the 3 facades, then all 6 VisiCalc demos
(web → Qt → Flutter → Compose → XAML → SwiftUI), one PR per stage/backend, each
security-reviewed, headless-proven, and babysat to merge.

## 1. Why this, and what already exists

A real spreadsheet is a *workbook* of named *sheets*, and formulas reach across
them (`=Summary!B2 + Detail!B2`). The shared Rust `spreadsheet-core` engine
already has most of the container:

- `Workbook` holds `sheets: Vec<Sheet>` + `sheet_by_name: HashMap<String, SheetId>`,
  with `add_sheet`, `sheet_count`, `sheet_id(name)`, `sheet_name(id)`.
- Every cell operation is already keyed by `SheetId` (`set_value(sheet, addr, …)`,
  `used_range(sheet)`, `changed_since(sheet, …)`, …).
- The dependency graph is **already cross-sheet**: `dag::Node = (SheetId, CellAddress)`
  and the `Workbook` field is documented "Cross-sheet dependency graph".

So the missing piece is **the formula layer**: today `FormulaAst::Ref(CellAddress)`
and `FormulaAst::Range(CellRange)` carry no sheet, so every reference resolves
against the cell's *own* sheet. `Summary!B2` cannot be parsed, evaluated,
dependency-tracked, or re-emitted. The parser module comment even lists
"3-D refs (`Sheet1:Sheet3!A1`)" as an aspiration that isn't built.

This spec closes that gap and exposes sheet management + cross-sheet formulas
all the way out to the six demos.

## 2. Engine design (`spreadsheet-core`)

### 2.1 AST — a reference gains an optional sheet qualifier

`FormulaAst::Ref` / `Range` carry an **optional** `SheetId`:

```
Ref   { sheet: Option<SheetId>, addr: CellAddress }
Range { sheet: Option<SheetId>, range: CellRange }
```

`None` = "the formula's own sheet" (the status-quo behaviour — every existing
same-sheet formula keeps `None`, so nothing about single-sheet workbooks changes).
`Some(id)` = an explicit cross-sheet reference. A range's two endpoints share one
sheet qualifier (Excel/Sheets semantics: `Sheet2!A1:B2`, never
`Sheet2!A1:Sheet3!B2` — 3-D refs are explicitly **out of scope** here and stay a
later feature).

Rationale for `Option` over always-`Some`: keeps the common same-sheet ref small,
keeps `shift()` relative-vs-absolute logic untouched for same-sheet refs, and
makes "this ref follows its host sheet" the zero-cost default.

### 2.2 Parser / grammar — `Name!A1`

Extend `parse_ident_or_ref` so a bareword (or a single-quoted name) **followed by
`!`** is a sheet qualifier on the reference that follows:

- `Summary!B2`, `Summary!A1:B10` — unquoted names (letters/digits/underscore, not
  starting with a digit, and not a legal A1 address — disambiguation rule below).
- `'Q1 Budget'!B2` — single-quoted names allow spaces/punctuation; `''` escapes a
  literal quote inside the name (Excel rule).
- The sheet **name** is resolved to a `SheetId` at *parse-against-a-workbook* time,
  not at bare-`parse()` time (the bare parser has no workbook). Two options, decided
  in the engine PR: (a) the AST stores the *name* string until bound, or (b) parsing
  stays workbook-aware. Leaning (a) — a `Ref { sheet: Option<SheetName>, … }` at the
  AST layer, lowered to `SheetId` when set on a workbook — so `parse()` stays pure
  and a formula referencing a not-yet-created sheet is a clean `#REF!` rather than a
  parse error. The engine PR will pin this down and document the choice.
- Unknown sheet name → the reference evaluates to `#REF!` (not a parse error), so a
  workbook can load formulas in any order.
- Ambiguity: `A1!B2` — `A1` is both a valid A1 address and a candidate sheet name.
  The `!` settles it: anything immediately before a `!` is a **sheet name**, never a
  cell. (A sheet literally named `A1` is legal and quoting is optional.)

### 2.3 Evaluation, dependencies, re-emit

- **Eval**: resolve `Ref { sheet, addr }` against `sheet.unwrap_or(host_sheet)`;
  read that sheet's cached value. Same for ranges (and SUM-over-range).
- **Dependencies**: the precedent extractor already yields `(SheetId, CellAddress)`
  nodes; emit the *referenced* sheet's id for a qualified ref (and the host sheet's
  for `None`). The cross-sheet DAG then dirties `Summary!B2`'s dependents on the
  other sheet automatically — `changed_since` already spans sheets.
- **Re-emit** (`to_a1` / source round-trip): a qualified ref prints
  `SheetName!A1`, quoting the name iff it needs it (spaces/punctuation/leading
  digit/looks-like-A1). This is what the formula bar shows and what `serialize`
  stores, so the qualifier survives save/load.

### 2.4 Structural-edit + fill/sort interaction (the subtle part)

- `shift(d_row, d_col)` on a **same-sheet** (`None`) ref is unchanged.
- A **cross-sheet** ref shifts its *address* under fill/copy like any ref but
  **keeps its sheet qualifier** (filling `=Detail!A1` down a column gives
  `=Detail!A2`).
- Structural edits (insert/delete rows/cols, sort) on sheet *S* must shift **every
  reference that points into S** — including qualified refs living on *other*
  sheets. Today structural edits walk one sheet's cells; they must also walk other
  sheets' formulas for inbound qualified refs. A reference whose whole band is
  deleted becomes `#REF!` (qualifier preserved or dropped — match Excel: the whole
  ref becomes `#REF!`).
- **Rename a sheet** ⇒ rewrite the qualifier string in every formula that names it
  (the stored `SheetId` is stable, so eval is unaffected; only the *display/source*
  name changes — handled at re-emit time if the AST holds `SheetId`, or by a
  rewrite pass if it holds the name). **Delete a sheet** ⇒ inbound qualified refs
  become `#REF!`.

### 2.5 Sheet-management API on `Workbook`

Add the operations a host needs (some exist):

- `add_sheet(name) -> SheetId` (exists), `sheet_count`, `sheet_id`, `sheet_name`
  (exist).
- `rename_sheet(id, new_name) -> Result<(), _>` (reject duplicate/empty names),
- `delete_sheet(id)` (inbound refs → `#REF!`; reject deleting the last sheet),
- `move_sheet(id, to_index)` (reorder for the tab bar),
- `sheet_names() -> Vec<&str>` in tab order.

`serialize`/`deserialize` already rebuild "the sheets in file order"; extend the
document to carry **all** sheets' sources + formats + the tab order, and round-trip
cross-sheet qualifiers.

### 2.6 Engine PR slicing (each its own PR, tests + spec sync)

1. **AST + parser**: optional sheet qualifier on `Ref`/`Range`; parse
   `Name!A1` / `'q'!A1:B2`; re-emit with quoting; round-trip tests. No eval yet
   (qualifier present but resolved as same-sheet is *not* allowed to ship — gate
   behind the eval PR, or land both together if small).
2. **Eval + dependencies**: resolve qualified refs; emit cross-sheet dep nodes;
   cross-sheet recompute test (`Sheet2!A1` edited ⇒ `Sheet1` formula updates).
3. **Structural-edit + fill/sort propagation**: inbound-ref shifting + `#REF!` on
   deleted band; cross-sheet fill keeps qualifier.
4. **Sheet management**: rename/delete/move + `serialize` of all sheets + qualifier
   round-trip; rename rewrites the displayed qualifier; delete → `#REF!`.

Bump `spreadsheet-core` minor per PR; keep `#![forbid(unsafe_code)]`; clippy-clean;
update this spec + `spreadsheet-core.md` as the design firms up.

## 3. Facades

The three facades currently **pin one sheet** (the original VisiCalc was
single-sheet). Make them multi-sheet-aware **once the engine PRs merge**:

- **core-wasm `SpreadsheetSession`**: keep a *current/active* sheet for the bare-A1
  reads/writes the demos already use (zero churn for existing calls), and add:
  `add_sheet(name)`, `rename_sheet`, `delete_sheet`, `move_sheet`,
  `sheet_names() -> JSON`, `active_sheet` / `set_active_sheet(name|index)`. The
  `raw` echo map becomes per-sheet. Cross-sheet *formulas* need no new write path —
  they're just text the engine parses; only the read/active-sheet plumbing is new.
- **capi**: `sc_add_sheet`, `sc_rename_sheet`, `sc_delete_sheet`, `sc_move_sheet`,
  `sc_sheet_names` (JSON char*), `sc_active_sheet` / `sc_set_active_sheet`
  (+ `include/spreadsheet.h`).
- **wasm**: matching linear-mem exports + JS loader wrappers + rebuilt
  `pkg/spreadsheet_engine.wasm` + bundle.
- Bump the 3 facades; CHANGELOGs; `verify-infinite.mjs` gains a multi-sheet +
  cross-sheet proof. One facades PR (or two if large).

## 4. Demos (one PR per backend, the find/replace rollout shape)

Each demo gains a **sheet tab bar** (the active sheet's tab highlighted; `＋` to
add, double-click to rename, context/delete to remove) and switches the grid to the
active sheet. Cross-sheet formulas need no special UI — typing `=Summary!B2` into
the existing formula bar just works once the engine resolves it. Per backend:

- re-vendor the capi/wasm,
- model passthrough: `sheetNames()`, `activeSheet`, `addSheet`, `renameSheet`,
  `deleteSheet`, `selectSheet`,
- a tab-bar view bound to that passthrough,
- a headless proof: two sheets, a cross-sheet formula (`Sheet2!A1` on Sheet1
  recomputes when `Sheet2!A1` changes), add/rename/delete, save/load round-trips the
  second sheet + the qualifier,
- run the backend proof (`verify-infinite.mjs` / `tst_window` / `flutter test` /
  `verify.sh` / `swift test`), `/security-review` inline, PR
  `feat(visicalc-<backend>): multi-sheet workbook in the <Backend> demo`.

## 5. Out of scope (later features)

- 3-D refs (`Sheet1:Sheet3!A1`) — the AST single-sheet-qualifier choice keeps the
  door open but does not build it.
- Sheet-level formatting/visibility, sheet colors, very-hidden sheets.
- Workbook-level named ranges (a separate candidate feature).

## 6. Verification summary

Every stage keeps the established proof discipline: engine unit tests
(cross-sheet recompute, structural propagation, rename/delete → `#REF!`, save/load
round-trip), facade `verify-infinite.mjs`, and each demo's headless proof asserting
a cross-sheet formula recomputes live. The single-sheet path stays byte-identical
(every existing same-sheet formula keeps an unqualified `None` ref), so this is
additive, not a rewrite.
