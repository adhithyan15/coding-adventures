# Changelog — latex

All notable changes to the full-fidelity LaTeX parser crate.

## [0.70.0] — 2026-07-11

### Added — single-integer TOTAL of the duplicate ("multiply defined") `\label`s (LTXDOC03 S34)

A **new** public method `Document::duplicate_label_count(&self) -> String` that renders the decimal
**COUNT** of the duplicate (later, losing) `\label`s — the `\label`s of already-defined keys, which LaTeX
flags with *"Label `key' multiply defined"* — as one integer line. It is the **label-side twin** of S33's
`duplicate_bibliography_count`: where S33 counts the losing `\bibitem`s, S34 counts the losing `\label`s.
It is the *count-total* companion of S20's `duplicate_label_definitions` (which renders one `\label{key}`
warning line per losing duplicate): S20 and S34 are two *views* of the one `duplicates` list
`resolve_references()` produces — S34 collapses the whole list to a single `.len()` tally. It is the
**warning-side companion** of S29's `label_definition_count` (which counts the *winning* label
definitions), completing the *label totals family* just as S33 completed the citation totals family. S29
counts the winning `definitions` and S34 the losing `duplicates`; together they **partition** every
`\label` in the document — `label_definition_count + duplicate_label_count` equals the total number of
`\label`s, because `resolve_references()` routes each `\label` into exactly one of
`definitions`/`duplicates`. It reads only `resolve_references().duplicates.len()` — the winning first
`\label` of a key lives in `definitions` (S22/S29's domain) and is excluded by construction — never a
`key`/`kind`/`span`, no source slicing at all, so every losing `\label` (section, figure, equation, or
inline) folds into one total. We do **not** de-duplicate: a key defined *three* times contributes *two*
losing duplicates, exactly the two warning lines S20 emits. Being a COUNT renderer, its empty case (no
labels, or every key defined exactly once) is the honest number `"0"` — **not** a `(no duplicate label
definitions)` marker (that discipline belongs to the *list* renderer S20; this mirrors
S27/S28/S29/S30/S31/S32/S33). One line, no trailing newline. E.g. `\label{x}` twice + `\label{y}` once →
`1`. It is a read-only view over `resolve_references()`; every S1–S33 output is left **byte-for-byte
unchanged**; S34 is purely additive and leaves the `to_latex()` round-trip fixed point intact. Total &
panic-free.

## [0.69.0] — 2026-07-10

### Added — single-integer TOTAL of the duplicate ("multiply defined") `\bibitem`s (LTXDOC03 S33)

A **new** public method `Document::duplicate_bibliography_count(&self) -> String` that renders the decimal
**COUNT** of the duplicate (later, losing) `\bibitem`s — the `\bibitem`s of already-defined keys, which
LaTeX flags with *"Citation `key' multiply defined"* — as one integer line. It is the *count-total*
companion of S16's `duplicate_bibliography_entries` (which renders one `\bibitem{key}` warning line per
losing duplicate): S16 and S33 are two *views* of the one `duplicate_entries` list `resolve_citations()`
produces — S33 collapses the whole list to a single `.len()` tally. It is the **warning-side companion**
of the resolved (S30/S31) and unresolved (S32) citation totals, completing the *totals family* over the
citation tables. S30 counts the winning `entries` and S33 the losing `duplicate_entries`; together they
**partition** every `\bibitem` inside a `thebibliography` — `bibliography_entry_count +
duplicate_bibliography_count` equals the total number of `\bibitem`s, because `resolve_citations()` routes
each `\bibitem` into exactly one of `entries`/`duplicate_entries`. It reads only
`resolve_citations().duplicate_entries.len()` — the winning first `\bibitem` of a key lives in `entries`
(S19/S30's domain) and is excluded by construction — never a `key`/`span`, no source slicing at all, so
every losing `\bibitem` folds into one total. We do **not** de-duplicate: a key defined *three* times
contributes *two* losing duplicates, exactly the two warning lines S16 emits. Being a COUNT renderer, its
empty case (no bibliography, or every key defined exactly once) is the honest number `"0"` — **not** a
`(no duplicate bibliography entries)` marker (that discipline belongs to the *list* renderer S16; this
mirrors S27/S28/S29/S30/S31/S32). One line, no trailing newline. E.g. `\bibitem{smith}` twice +
`\bibitem{jones}` once → `1`. It is a read-only view over `resolve_citations()`; every S1–S32 output is
left **byte-for-byte unchanged**; S33 is purely additive and leaves the `to_latex()` round-trip fixed
point intact. Total & panic-free.

---

## [0.68.0] — 2026-07-09

### Added — single-integer TOTAL of the unresolved (dangling) citations (LTXDOC03 S32)

A **new** public method `Document::unresolved_citation_count(&self) -> String` that renders the decimal
**COUNT** of the unresolved (dangling) citations — the `\cite` keys **no** `\bibitem` defines — as one
integer line. It is the *count-total* companion of S17's `unresolved_citations_by_source` (which renders
the dangling keys grouped by their source `\cite`): S17 and S32 are two *views* of the one `unresolved`
list `resolve_citations()` produces — S32 collapses the whole list to a single `.len()` tally. It is the
exact unresolved-**citation-side twin** of S27's `unresolved_reference_count`, and the **dangling sibling**
of S31's resolved `citation_count`. Together S31 and S32 **partition** every per-key `\cite` record:
`citation_count + unresolved_citation_count` equals the total number of cited keys, because
`resolve_citations()` routes each key into exactly one of `resolved`/`unresolved`. It reads only
`resolve_citations().unresolved.len()` — a resolved `\cite{a}` lives in `resolved` (S15/S31's domain) and is
excluded by construction — never a `cite_span`/dangling `key`, no source slicing at all, so every dangling
key folds into one total. Being a COUNT renderer, its empty case (every cited key resolving, or none at
all) is the honest number `"0"` — **not** a `(no unresolved citations)` marker (that discipline belongs to
the *list* renderer S17; this mirrors S27/S28/S29/S30/S31). One line, no trailing newline. E.g. `\cite{a,b}`
(both defined) + `\cite{c,ghost}` (only `c` defined) → `1`. It is a read-only view over
`resolve_citations()`; every S1–S31 output is left **byte-for-byte unchanged**; S32 is purely additive and
leaves the `to_latex()` round-trip fixed point intact. Total & panic-free.

---

## [0.67.0] — 2026-07-09

### Added — single-integer TOTAL of the resolved citations (LTXDOC03 S31)

A **new** public method `Document::citation_count(&self) -> String` that renders the decimal **COUNT** of
the resolved citations — the `\cite` keys some `\bibitem` defines — as one integer line. It is the
*count-total* companion of S15's `citations_by_source` (which renders the resolved keys grouped by their
source `\cite`): S15 and S31 are two *views* of the one `resolved` list `resolve_citations()` produces —
S31 collapses the whole list to a single `.len()` tally. It is the exact resolved-**citation-side twin** of
S28's `resolved_reference_count`, extending the *totals family* onto the resolved-citation table: S27
`unresolved_reference_count` and S28 `resolved_reference_count` count the two reference tables, S29
`label_definition_count` counts the label definitions, S30 `bibliography_entry_count` counts the
bibliography entries, and S31 counts the resolved citations. It reads only
`resolve_citations().resolved.len()` — a dangling `\cite{ghost}` lives in `unresolved` (S17's domain) and is
excluded by construction — never a `cite_span`/`entry_span`, no source slicing at all, so every resolved key
folds into one total. Being a COUNT renderer, its empty case (every cited key dangling, or none at all) is
the honest number `"0"` — **not** a `(no resolved citations)` marker (that discipline belongs to the *list*
renderer S15; this mirrors S27/S28/S29/S30). One line, no trailing newline. E.g. `\cite{a,b}` (both defined)
+ `\cite{c,ghost}` (only `c` defined) → `3`. It is a read-only view over `resolve_citations()`; every
S1–S30 output is left **byte-for-byte unchanged**; S31 is purely additive and leaves the `to_latex()`
round-trip fixed point intact. Total & panic-free.

---

## [0.66.0] — 2026-07-09

### Added — single-integer TOTAL of the bibliography entries (LTXDOC03 S30)

A **new** public method `Document::bibliography_entry_count(&self) -> String` that renders the decimal
**COUNT** of the winning bibliography entries — the distinct `\bibitem` keys the document defines inside a
`thebibliography` environment — as one integer line. It is the *count-total* companion of S19's
`bibliography_entries` (which renders one `[n] key` **line per winning entry**, 1-based in source order):
S19 and S30 are two *views* of the one winning `entries` list `resolve_citations()` produces — S30
collapses the whole list to a single `.len()` tally. It is the exact **citation-side analogue** of S29's
`label_definition_count`, completing the *totals family*: S27 `unresolved_reference_count` and S28
`resolved_reference_count` count the two reference tables, S29 `label_definition_count` counts the label
definitions, and S30 counts the bibliography entries. It reads only `resolve_citations().entries.len()` — a
later duplicate `\bibitem{dup}` lives in `duplicate_entries` (S16's domain) and is excluded by construction,
so the count is exactly the number of lines S19 lists. Being a COUNT renderer, its empty case (no `\bibitem`
at all) is the honest number `"0"` — **not** a `(no bibliography entries)` marker (that discipline belongs
to the *list* renderer S19; this mirrors S27/S28/S29). One line, no trailing newline. It is a read-only view
over `resolve_citations()`; every S1–S29 output is left **byte-for-byte unchanged**; S30 is purely additive
and leaves the `to_latex()` round-trip fixed point intact. Total & panic-free.

---

## [0.65.0] — 2026-07-08

### Added — single-integer TOTAL of the label definitions (LTXDOC03 S29)

A **new** public method `Document::label_definition_count(&self) -> String` that renders the decimal
**COUNT** of the winning label definitions — the distinct `\label` keys the document defines — as one
integer line. It is the *count-total* companion of S22's `label_definitions` and S23's
`label_definitions_by_kind` (which render one `\label{key}` **line per winning definition**, flat in
source order or grouped by kind): S22/S23 and S29 are two *views* of the one winning `definitions` list
`resolve_references()` produces — S29 collapses the whole list to a single `.len()` tally. It is the exact
label-definition-side **analogue** of the reference-side totals S27's `unresolved_reference_count` and
S28's `resolved_reference_count`, and the count-total sibling of the census family (S25 `label_kind_counts`)
but for the *whole* definition list rather than per-kind. It is a read-only view over
`resolve_references()`; every S1–S28 output is left **byte-for-byte unchanged**; S29 is purely additive and
leaves the `to_latex()` round-trip fixed point intact.

- **Counts the WINNING definitions only.** S29 reads `resolve_references().definitions.len()` — each entry
  a `LabelDef` for the first `\label` of a distinct key. A later re-definition `\label{dup}` lives in
  `resolve_references().duplicates` (S20's domain), never in `definitions`, so it is excluded by
  construction — the count is exactly the number of lines S22 lists.
- **A single decimal line, always.** The output is the decimal `.len()` of the `definitions` list — one
  line, no trailing newline. There is **no** source slicing and **no** `kind` read at all (unlike S25's
  per-kind census); section, figure, equation, and inline labels all fold into one total — only `.len()`
  is taken.
- **Zero is the honest value `"0"`.** Being a COUNT renderer, its empty case (no `\label` at all) is the
  number `"0"` — **not** a `(no label definitions)` marker. This mirrors S27/S28 exactly. The `(no …)`
  marker discipline belongs to the *list* renderers S22/S23, whose empty case has no lines to show; a
  total count of zero *is* a number.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single read of
  the already-bounded `definitions` list's length. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s29_multiple_definitions_count_the_integer`,
  `s29_duplicate_label_counted_once_matching_s22` (cross-checking that a later `\label{dup}` is a
  duplicate — S20's domain — not a second definition), `s29_no_labels_counts_zero`,
  `s29_mixed_document_counts_only_label_definitions` (the count is unaffected by refs/citations),
  `s29_count_equals_number_of_s22_lines` (cross-checking the total against the number of lines S22
  enumerates), and `s29_is_additive_leaves_s1_s28_outputs_unchanged` (which pins a handful of prior
  S1–S28 outputs byte-for-byte — including S22's `label_definitions`, S25's `label_kind_counts`, S27's
  `unresolved_reference_count`, and S28's `resolved_reference_count` — alongside the new count).

## [0.64.0] — 2026-07-08

### Added — single-integer TOTAL of the resolved references (LTXDOC03 S28)

A **new** public method `Document::resolved_reference_count(&self) -> String` that renders the decimal
**COUNT** of the RESOLVED `\ref`/`\eqref`/`\pageref` references — the ones some `\label` defines — as one
integer line. It is the *count-total* companion of S21's `resolved_references_by_source` and S24's
`resolved_references_by_kind` (which render one `\<command>{key}` **line per resolved reference**, flat in
source order or grouped by target kind): S21/S24 and S28 are two *views* of the one `resolved` list
`resolve_references()` produces — S28 collapses the whole list to a single `.len()` tally. It is the exact
resolved-side **twin** of S27's `unresolved_reference_count`: together S28 + S27 split every reference into
the pair (resolved, dangling), so their two totals sum to the total reference count. It is a read-only
view over `resolve_references()`; every S1–S27 output is left **byte-for-byte unchanged**; S28 is purely
additive and leaves the `to_latex()` round-trip fixed point intact.

- **Counts the RESOLVED refs only.** S28 reads `resolve_references().resolved.len()` — each entry a
  `ResolvedRef` for a `\ref`/`\eqref`/`\pageref` that some `\label` defines. A dangling `\ref{nope}` lives
  in `resolve_references().unresolved` (S18/S27's domain), never in `resolved`, so it is excluded by
  construction.
- **A single decimal line, always.** The output is the decimal `.len()` of the `resolved` list — one
  line, no trailing newline. There is **no** source slicing and **no** `target_kind` read at all (unlike
  S26's per-kind census); section, table, and equation references all fold into one total — only `.len()`
  is taken.
- **Zero is the honest value `"0"`.** Being a COUNT renderer, its empty case (every ref dangles, or there
  are none at all) is the number `"0"` — **not** a `(no resolved references)` marker. This mirrors S27
  exactly. The `(no …)` marker discipline belongs to the *list* renderers S21/S24, whose empty case has no
  lines to show; a total count of zero *is* a number.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single read of
  the already-bounded `resolved` list's length. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s28_two_resolved_plus_one_dangling_counts_two` (cross-checking that S27 returns `"1"`
  on the same doc — the two totals split the references), `s28_no_resolvable_refs_counts_zero`,
  `s28_no_references_at_all_counts_zero`, `s28_mixed_target_kinds_count_the_integer`,
  `s28_count_equals_number_of_s21_lines` (cross-checking the total against the number of lines S21
  enumerates), and `s28_is_additive_leaves_s1_s27_outputs_unchanged` (which pins a handful of prior
  S1–S27 outputs byte-for-byte — including S21's `resolved_references_by_source` and S27's
  `unresolved_reference_count` — alongside the new count).

## [0.63.0] — 2026-07-08

### Added — single-integer TOTAL of the unresolved (dangling) references (LTXDOC03 S27)

A **new** public method `Document::unresolved_reference_count(&self) -> String` that renders the decimal
**COUNT** of the UNRESOLVED (dangling) `\ref`/`\eqref`/`\pageref` references — the ones no `\label`
defines (LaTeX's *"Reference `key' undefined"*, the `??`) — as one integer line. It is the *count-total*
companion of S18's `unresolved_references_by_source` (which renders one `\<command>{key}` **line per
dangling reference**): S18 and S27 are two *views* of the one `unresolved` list `resolve_references()`
produces — S27 collapses the whole list to a single `.len()` tally. It is the count-total sibling of the
census family (S25 `label_kind_counts`, S26 `resolved_reference_kind_counts`), but for the UNRESOLVED
refs — which carry **no** `target_kind` (a dangling ref bound to nothing), so a per-kind census is not
viable; a single total is the clean move. It is a read-only view over `resolve_references()`; every
S1–S26 output is left **byte-for-byte unchanged**; S27 is purely additive and leaves the `to_latex()`
round-trip fixed point intact.

- **Counts the UNRESOLVED refs only.** S27 reads `resolve_references().unresolved.len()` — each entry an
  `UnresolvedRef` for a dangling `\ref`/`\eqref`/`\pageref`. A resolved `\ref{sec:i}` lives in
  `resolve_references().resolved` (S21's domain), never in `unresolved`, so it is excluded by
  construction.
- **A single decimal line, always.** The output is the decimal `.len()` of the `unresolved` list — one
  line, no trailing newline. There is **no** source slicing and **no** `target_kind` read at all (a
  dangling ref never carries one); only `.len()` is taken.
- **Zero is the honest value `"0"`.** Being a COUNT renderer, its empty case (every ref resolves, or
  there are none at all) is the number `"0"` — **not** a `(no …)` marker. The `(no …)` marker discipline
  belongs to the *list* renderers S18/S21/S24, whose empty case has no lines to show; a total count of
  zero *is* a number.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single read
  of the already-bounded `unresolved` list's length. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s27_two_dangling_plus_one_resolved_counts_two`, `s27_all_refs_resolve_counts_zero`,
  `s27_no_references_at_all_counts_zero`, `s27_mixed_kinds_of_danglers_count_the_integer`,
  `s27_count_equals_number_of_s18_lines` (cross-checking the total against the number of lines S18
  enumerates), and `s27_is_additive_leaves_s1_s26_outputs_unchanged` (which pins a handful of prior
  S1–S26 outputs byte-for-byte — including S18's `unresolved_references_by_source` and S26's
  `resolved_reference_kind_counts` — alongside the new count).

## [0.62.0] — 2026-07-08

### Added — per-kind census (counts) of the resolved references (LTXDOC03 S26)

A **new** public method `Document::resolved_reference_kind_counts(&self) -> String` that renders a
**per-kind CENSUS** of the RESOLVED `\ref`/`\eqref`/`\pageref` references — one `<kind>: <n>` line per
`LabelKind` that has at least one resolved ref, carrying the integer **count** rather than a list. It is
the *count* companion of S24's `resolved_references_by_kind` (which renders one `[kind] \<command>{key}`
**line per resolved reference**, grouped by the kind each ref bound to): S24, S21's flat
`resolved_references_by_source`, and S26 are three *views* of the one `resolved` list
`resolve_references()` produces — S26 collapses each kind's group to a single tally line. It is to S24
what S25's `label_kind_counts` is to S23: a numeric summary. It is a read-only view over
`resolve_references()`; every S1–S25 output is left **byte-for-byte unchanged**; S26 is purely additive
and leaves the `to_latex()` round-trip fixed point intact.

- **Counts the RESOLVED refs only.** S26 reads `resolve_references().resolved` — each a `ResolvedRef`
  carrying the `target_kind` (the `LabelKind` of the label it bound to). A dangling `\ref{nope}` lives
  in `resolve_references().unresolved` (S18's domain), never in `resolved`, so it is excluded by
  construction — never contributing a spurious `<kind>: 0` line.
- **Per-kind counts in a fixed, document-independent order** — the `LabelKind` enum declaration order:
  `Section`, `Table`, `Figure`, `Equation`, `Inline` (the SAME `const KIND_ORDER` slice S23/S24/S25
  use). The method iterates that explicit fixed order (not a hash map keyed by kind), so the line order
  is deterministic — the same `Vec`-scan discipline S17/S18/S23/S24/S25 use to avoid hash-order
  nondeterminism.
- **`<kind>: <n>` line shape.** Each line is the stable lowercase tag from `LabelKind::as_str()` (the
  SAME kind string S24 renders — `"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`) + `": "` +
  the decimal count of resolved refs whose `target_kind` is that kind. There is **no** source slicing
  at all (only the `target_kind` field is read).
- **Zero-count kinds omitted.** A kind with no resolved refs contributes no line (there is never a bare
  `table: 0` for a doc that references no tables).
- **Stable empty marker.** No resolved references at all → the **same** fixed string
  `(no resolved references)` S21/S24 use, never the empty string — the stable-marker discipline S12–S25
  share.
- **`\n`-joined, no trailing newline** — matching S24's `resolved_references_by_kind` and every S11–S25
  renderer.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single
  stable-ordered pass (fixed kind order × pre-order filter/count) over the already-bounded `resolved`
  list. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s26_counts_multiple_kinds_in_fixed_kind_order_zero_kinds_omitted`,
  `s26_exactly_one_kind`, `s26_two_refs_to_two_section_labels_count_two`,
  `s26_all_dangling_or_none_returns_marker`, `s26_newline_join_no_trailing_newline_excludes_dangling`,
  and `s26_is_additive_leaves_s1_s25_outputs_unchanged` (which pins every S1–S25 output byte-for-byte,
  including S24's grouped `resolved_references_by_kind` and S25's `label_kind_counts`, alongside the new
  per-kind counts).

## [0.61.0] — 2026-07-08

### Added — per-kind census (counts) of the winning label definitions (LTXDOC03 S25)

A **new** public method `Document::label_kind_counts(&self) -> String` that renders a **per-kind
CENSUS** of the winning `\label` definitions — one `<kind>: <n>` line per `LabelKind` that has at
least one winning definition, carrying the integer **count** rather than a list. It is the *count*
companion of S23's `label_definitions_by_kind` (which renders one `[kind] \label{key}` **line per
definition**, grouped by kind): S23, S22's flat `label_definitions`, and S25 are three *views* of the
one winning `definitions` list `resolve_references()` produces — S25 collapses each kind's group to a
single tally line. It is to S23 what S14's `list_summary` (`"Sections: 1"`) is to a full enumeration:
a numeric summary. It is a read-only view over `resolve_references()`; every S1–S24 output is left
**byte-for-byte unchanged**; S25 is purely additive and leaves the `to_latex()` round-trip fixed point
intact.

- **Counts the WINNING definitions only.** S25 reads `resolve_references().definitions` — one row per
  distinct key (the first `\label` of each key). A `\label{dup}` written twice contributes **one** to
  its kind's count, because its later re-definition is a `Duplicate` (S20's domain), never a second row
  in `definitions`.
- **Per-kind counts in a fixed, document-independent order** — the `LabelKind` enum declaration order:
  `Section`, `Table`, `Figure`, `Equation`, `Inline` (the SAME `const KIND_ORDER` slice S23/S24 use).
  The method iterates that explicit fixed order (not a hash map keyed by kind), so the line order is
  deterministic — the same `Vec`-scan discipline S17/S18/S23/S24 use to avoid hash-order nondeterminism.
- **`<kind>: <n>` line shape.** Each line is the stable lowercase tag from `LabelKind::as_str()` (the
  SAME kind string S23 renders — `"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`) + `": "` +
  the decimal count of winning definitions of that kind. There is **no** source slicing at all (only
  the `kind` field is read).
- **Zero-count kinds omitted.** A kind with no winning definitions contributes no line (there is never
  a bare `table: 0` for a doc with no table labels).
- **Stable empty marker.** No winning label definitions at all → the **same** fixed string
  `(no label definitions)` S22/S23 use, never the empty string — the stable-marker discipline S12–S24
  share.
- **`\n`-joined, no trailing newline** — matching S23's `label_definitions_by_kind` and every S11–S24
  renderer.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single
  stable-ordered pass (fixed kind order × pre-order filter/count) over the already-bounded
  `definitions` list. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s25_counts_multiple_kinds_in_fixed_kind_order_zero_kinds_omitted`,
  `s25_exactly_one_kind`, `s25_no_labels_returns_marker`,
  `s25_duplicate_definitions_count_only_the_winner`, `s25_newline_join_no_trailing_newline`, and
  `s25_is_additive_leaves_s1_s24_outputs_unchanged` (which pins every S1–S24 output byte-for-byte,
  including S22's flat `label_definitions` and S23's grouped `label_definitions_by_kind`, alongside the
  new per-kind counts).

## [0.60.0] — 2026-07-08

### Added — resolved references grouped by target kind (LTXDOC03 S24)

A **new** public method `Document::resolved_references_by_kind(&self) -> String` that renders the
**resolved `\ref`/`\eqref`/`\pageref` references grouped by the `LabelKind` they resolved TO** — a
per-kind census of the successfully-matched references. It is the by-kind grouping companion of S21's
`resolved_references_by_source` (which lists the same resolved references **flat**, one
`\<command>{key}` per line in source pre-order): S21 and S24 are two *views* of the one `resolved` list
`resolve_references()` produces. It mirrors S23's `label_definitions_by_kind` idiom (same
`const KIND_ORDER`, same flat_map/filter pass) but over the **resolved-references** list instead of the
**definitions** list, and stays **command-aware** like S21. It is a read-only view over
`resolve_references()`; every S1–S23 output is left **byte-for-byte unchanged**; S24 is purely additive
and leaves the `to_latex()` round-trip fixed point intact.

- **Grouped by target kind in a fixed, document-independent order** — the `LabelKind` enum declaration
  order: `Section`, `Table`, `Figure`, `Equation`, `Inline` (the SAME `const KIND_ORDER` slice S23
  uses). The method iterates that explicit fixed order (not a hash map keyed by kind), so the group
  order is deterministic — the same `Vec`-scan discipline S17/S18/S23 use to avoid hash-order
  nondeterminism.
- **Pre-order within each kind.** Refs that resolved to a given kind keep their existing pre-order from
  `resolved`; grouping never reorders within a kind.
- **`[kind] \<command>{key}` line shape, command-aware.** Each line is `[` + the stable lowercase tag
  from `LabelKind::as_str()` (of the ref's `target_kind`) + `] \` + the ref's **own** `command` + `{` +
  its **owned `key` `String`** + `}` — so a resolved `\eqref` renders `[equation] \eqref{eq:m}` and a
  resolved `\pageref` renders `[section] \pageref{sec:i}`; the command is **never** hard-coded to
  `\ref`. There is **no** source slicing at all (matching S21 `resolved_references_by_source` and S23
  `label_definitions_by_kind`). The `[kind]` prefix makes the census visible on every line while
  staying one-line-per-ref; it is a report annotation, not round-trippable LaTeX.
- **Dangling refs excluded by construction.** A `\ref{nope}` with no `\label` never entered `resolved`,
  so it appears in S18's `unresolved_references_by_source`, not here.
- **No empty groups.** A kind with no resolved refs contributes no lines and no bare `[table]` header.
- **Stable empty marker.** No resolved references at all (every ref dangles, or there are none) → the
  **same** fixed string `(no resolved references)` S21 uses, never the empty string — the stable-marker
  discipline S12–S23 share.
- **`\n`-joined, no trailing newline** — matching S21's `resolved_references_by_source` and every
  S11–S23 renderer.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing (`ref_span`/
  `target_span` unused); a single stable-ordered pass (fixed kind order × pre-order filter) over the
  already-bounded `resolved` list. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s24_groups_different_kinds_in_fixed_kind_order`,
  `s24_reorders_source_to_fixed_kind_order`, `s24_same_kind_grouped_in_preorder_command_aware`,
  `s24_only_dangling_or_none_returns_marker`, `s24_newline_join_no_trailing_newline_excludes_dangling`,
  and `s24_is_additive_leaves_s1_s23_outputs_unchanged` (which pins every S1–S23 output byte-for-byte,
  including S21's flat `resolved_references_by_source`, alongside the new grouped output).

## [0.59.0] — 2026-07-08

### Added — winning label definitions grouped by kind (LTXDOC03 S23)

A **new** public method `Document::label_definitions_by_kind(&self) -> String` that renders the
**winning label definitions grouped by their `LabelKind`** — a per-kind census of the `\label`
definitions. It is the by-kind grouping companion of S22's `label_definitions` (which lists the same
winning definitions **flat**, one `\label{key}` per line in pure pre-order): S22 and S23 are two
*views* of the one winning `definitions` list `resolve_references()` produces. It is a read-only view
over `resolve_references()`; every S1–S22 output is left **byte-for-byte unchanged**; S23 is purely
additive and leaves the `to_latex()` round-trip fixed point intact.

- **Grouped by kind in a fixed, document-independent order** — the `LabelKind` enum declaration order:
  `Section`, `Table`, `Figure`, `Equation`, `Inline`. The method iterates that explicit fixed order
  (not a hash map keyed by kind), so the group order is deterministic — the same `Vec`-of-groups
  discipline S17/S18 use to avoid hash-order nondeterminism.
- **Pre-order within each kind.** Definitions of a given kind keep their existing pre-order from
  `definitions`; grouping never reorders within a kind.
- **`[kind] \label{key}` line shape.** Each line is `[` + the stable lowercase tag from
  `LabelKind::as_str()` (`"section"`, `"table"`, `"figure"`, `"equation"`, `"inline"`) + `] \label{` +
  the definition's **owned `key` `String`** + `}` — so there is **no** source slicing at all (matching
  S13 `resolve_namerefs`, S19 `bibliography_entries`, S20 `duplicate_label_definitions`, and S22
  `label_definitions`). The `[kind]` prefix makes the census visible on every line while staying
  one-line-per-definition; it is a report annotation, not round-trippable LaTeX.
- **No empty groups.** A kind with no definitions contributes no lines and no bare `[table]` header.
- **Stable empty marker.** No label definitions at all → the **same** fixed string
  `(no label definitions)` S22 uses (S23 groups the identical list), never the empty string — the
  stable-marker discipline S12–S22 share.
- **`\n`-joined, no trailing newline** — matching S11's `to_plain_text_by_kind` and every S12–S22
  renderer.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single
  stable-ordered pass (fixed kind order × pre-order filter) over the already-bounded `definitions`
  list. Borrows `self` immutably, returns owned `String`.
- **Tests.** Added `s23_groups_different_kinds_in_fixed_kind_order`,
  `s23_reorders_source_to_fixed_kind_order`, `s23_same_kind_grouped_in_preorder`,
  `s23_no_labels_returns_marker`, `s23_newline_join_no_trailing_newline`, and
  `s23_is_additive_leaves_s1_s22_outputs_unchanged` (which pins every S1–S22 output byte-for-byte,
  including S22's flat `label_definitions`, alongside the new grouped output).

## [0.58.0] — 2026-07-08

### Added — winning label definitions report (LTXDOC03 S22)

A **new** public method `Document::label_definitions(&self) -> String` that renders the **winning label
definitions** — the `\label{key}` definitions that references resolve against, one `\label{key}` line
per distinct key. It is the label-family analogue of S19's `bibliography_entries` (which renders the
winning `\bibitem` entries) and the *winning-side* counterpart of S20's `duplicate_label_definitions`
(which renders the *losing* duplicate `\label` definitions). It is a read-only view over
`resolve_references()`: S1 splits every `\label` into the **winning** first definition of each key
(`definitions`, a `Vec<LabelDef>` with one row per distinct key, in pre-order) and the **losing** later
re-definitions (`duplicates`); S22 renders the `definitions` list. Every S1–S21 output is left
**byte-for-byte unchanged**; S22 is purely additive and leaves the `to_latex()` round-trip fixed point
intact.

- **One reconstructed `\label{key}` line per winning definition**, in the existing pre-order — **not**
  re-sorted and **not** de-duplicated (no de-duplication is needed: `definitions` already holds exactly
  one row per distinct key, later re-definitions having gone to `duplicates`). Each line is
  reconstructed from the definition's **owned `key` `String`** via `format!("\\label{{{}}}", def.key)`,
  so there is **no** source slicing at all (matching S13 `resolve_namerefs`, S15 `citations_by_source`,
  S16 `duplicate_bibliography_entries`, S19 `bibliography_entries`, and S20
  `duplicate_label_definitions`). `\label{key}` is the correct form for any `LabelKind` (section,
  figure, equation, or bare inline label).
- **Winning key appears once.** A `\label{dup}` written twice appears **once** here — the winning first
  definition; its losing second definition lives in S20 (`duplicate_label_definitions`), never here.
- **Stable empty marker.** No label definitions at all → the fixed string `(no label definitions)`,
  never `""` (the same discipline S12–S21 use). Lines are joined by `\n` with **no** trailing newline.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single pass
  over the already-bounded `definitions` list. Borrows `self` immutably and returns owned `String`.
- Tests: winning definitions in pre-order, empty-marker, duplicate-key-wins-once (cross-checked against
  S20's losing side), `\n`-join with no trailing newline, and an additivity test pinning every S1–S21
  output byte-for-byte on a representative document.

## [0.57.0] — 2026-07-08

### Added — resolved-references-by-source report (LTXDOC03 S21)

A **new** public method `Document::resolved_references_by_source(&self) -> String` that renders the
**resolved (successfully-matched) `\ref`/`\eqref`/`\pageref` references grouped by their source `\ref`**
— the exact structural mirror of S18's `unresolved_references_by_source` (which renders the *dangling*
half of the same split), but for the references that **did** resolve to a real `\label`. It is a
read-only view over `resolve_references()`: S3 splits every reference into the **resolved** ones
(`resolved`, a `Vec<ResolvedRef>`) and the **unresolved** (dangling) ones (`unresolved`); S21 renders
the `resolved` list. Every S1–S20 output is left **byte-for-byte unchanged**; S21 is purely additive and
leaves the `to_latex()` round-trip fixed point intact.

- **Command-aware, one line per resolved reference.** Each line is reconstructed from the ref's **own**
  `command` and `key` as `format!("\\{}{{{}}}", r.command, r.key)`, so a resolved `\eqref{eq:main}`
  renders `\eqref{eq:main}` and a resolved `\pageref{sec:intro}` renders `\pageref{sec:intro}` — the
  command is **never** hard-coded to `\ref`. No source slicing at all (matching S13
  `resolve_namerefs`, S15 `citations_by_source`, S17/S18's reports).
- **First-appearance grouping, source order.** Entries are grouped by their shared `ref_span` into a
  `Vec<(Span, Vec<&ResolvedRef>)>` (not a hash map) preserving first-appearance order — the same
  grouping idiom as S18. Each reference takes exactly one key, so every group emits one line.
- **Dangling refs excluded by construction.** A `\ref{nope}` with no `\label` lives in `unresolved`
  (S18), never in `resolved`, so it never appears here.
- **Stable empty marker.** No resolved references (every ref dangles, or none at all) → the fixed string
  `(no resolved references)`, never `""` (the same discipline S12–S20 use). Lines are joined by `\n`
  with **no** trailing newline.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single pass
  over the already-bounded `resolved`. Borrows `self` immutably and returns owned `String`.

Tests: `s21_preserves_command_ref_eqref_pageref`, `s21_no_references_returns_marker`,
`s21_only_dangling_returns_marker`, `s21_mixed_lists_only_resolved`, and an additivity test
`s21_is_additive_leaves_s1_s20_outputs_unchanged` pinning S1–S20 outputs (including
`duplicate_label_definitions`) byte-for-byte alongside the new method.

## [0.56.0] — 2026-07-07

### Added — losing duplicate-`\label` report (LTXDOC03 S20)

A **new** public method `Document::duplicate_label_definitions(&self) -> String` that renders the
**losing duplicate `\label` definitions** — the label-family mirror of S16's
`duplicate_bibliography_entries` (which renders the losing `\bibitem` duplicates). It is a read-only
view over `resolve_references()`: S1 splits every `\label` into the **winning** first definition of each
key (`definitions`) and the **losing** later re-definitions of an already-defined key (`duplicates` —
LaTeX's *"Label `key' multiply defined"* warning); S20 renders that `duplicates` list. Every S1–S19
output is left **byte-for-byte unchanged**; S20 is purely additive and leaves the `to_latex()`
round-trip fixed point intact.

- **One line per losing duplicate, in pre-order.** The `duplicates` list is already in body pre-order
  (first-definition-wins). S20 renders it verbatim — **not** re-sorted, **not** de-duplicated — so
  every *"multiply defined"* warning gets its own line, exactly like S16.
- **`\label{key}` reconstructed from the owned key.** Each line is `format!("\\label{{{}}}", dup.key)`
  — no source slicing at all (matching S13 `resolve_namerefs`, S15 `citations_by_source`, S16
  `duplicate_bibliography_entries`, and S17–S19's reports). Labels are always defined by `\label{…}`,
  so `\label{key}` is the correct reconstruction regardless of the duplicate's `LabelKind` (a
  re-`\label`ed section, figure, equation, or bare inline label all render the same `\label{key}`).
- **Winner lives elsewhere.** The winning first definition of each key stays in `definitions` (what
  `\ref`/`\eqref`/`\pageref` resolve against), never in this report.
- **Stable empty marker.** No duplicate labels (every key defined once, or no labels at all) → the
  fixed string `(no duplicate label definitions)`, never `""` (the same discipline S12–S19 use). Lines
  are joined by `\n` with **no** trailing newline.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing, no source slicing; a single pass
  over the already-bounded `duplicates`. Borrows `self` immutably and returns owned `String`.

Tests: `s20_reports_duplicate_label`, `s20_two_distinct_duplicates_preorder`,
`s20_no_duplicates_returns_marker`, and an additivity test
`s20_is_additive_leaves_s1_s19_outputs_unchanged` pinning S1–S19 outputs (including
`bibliography_entries`) byte-for-byte alongside the new method.

## [0.55.0] — 2026-07-07

### Added — numbered winning-bibliography-entry list (LTXDOC03 S19)

A **new** public method `Document::bibliography_entries(&self) -> String` that renders the **winning
bibliography entries as a numbered list** — the rendered bibliography a reader actually sees, and the
table citations resolve against. It is a **distinct** view over `resolve_citations()`: S16
(`duplicate_bibliography_entries`) renders the **losing** `duplicate_entries` as `\bibitem{key}` warning
lines, and S15 (`citations_by_source`) renders the per-source *resolved cite keys* — S19 fills the
remaining cell by rendering the **winning** `entries` themselves. Every S1–S18 output is left
**byte-for-byte unchanged**; S19 is purely additive and leaves the `to_latex()` round-trip fixed point
intact.

- **One numbered line per winning entry, 1-based, in pre-order.** S2 already collects the first
  `\bibitem{key}` of each distinct key into `resolve_citations().entries` (body pre-order; later
  re-definitions go to `duplicate_entries`, never here). S19 numbers that list 1-based via
  `enumerate()` + `n + 1`, emitting `format!("[{}] {}", n + 1, entry.key)` → `[1] smith2020`,
  `[2] jones2019`, …. Each line is reconstructed from the entry's **owned `key` `String`** (no source
  slicing at all, matching S13 `resolve_namerefs`, S15 `citations_by_source`, S16
  `duplicate_bibliography_entries`, and S17/S18's dangling reports).
- **`[n] key` chosen deliberately.** The numbered shape reads as a *rendered bibliography* and is
  **visually distinct** from S16's `\bibitem{key}` losing-duplicate lines, so the two never look alike
  even when they list overlapping keys.
- **Duplicates appear once.** Because `entries` holds only the **first** `\bibitem` of each key, a
  `\bibitem{dup}` written twice appears **once** here — the winner — exactly as a real bibliography
  renders one line per key. The losing re-definitions remain the S16 view.
- **Empty marker.** A document with **no** bibliography entries — no `thebibliography`, or an empty one
  — returns the fixed marker `"(no bibliography entries)"`, so the output is never the empty string
  (the same stable-marker discipline S12/S13/S14/S15/S16/S17/S18 use). Lines are joined by `\n` with
  **no** trailing newline.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all — keys
  are already owned `String`s); a single pass over the already-bounded `entries` list. Borrows `self`
  immutably, returns owned `String`.

Five `s19_*` tests pin the exact rendered strings: two distinct entries (`[1] a\n[2] b`), a duplicate
key winning once with a peer (`[1] dup\n[2] other`), three entries numbered in pre-order
(`[1] x\n[2] y\n[3] z`), a bibliography-free document returning the `(no bibliography entries)` marker,
and an additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`,
`resolve_namerefs`, `list_summary`, `citations_by_source`, `duplicate_bibliography_entries`,
`unresolved_citations_by_source`, and `unresolved_references_by_source` are all byte-for-byte unchanged.

## [0.54.0] — 2026-07-07

### Added — unresolved (dangling) references grouped by source `\ref` (LTXDOC03 S18)

A **new** public method `Document::unresolved_references_by_source(&self) -> String` that surfaces the
**unresolved (dangling) references grouped per source `\ref`** — the `\ref`-family parallel of S17's
dangling-CITATION report, and a **distinct** view from S6's flat *"Dangling references: k1, k2"* footer:
S18 reconstructs each dangling reference on **its own line**, **command-aware**, so `\eqref` and
`\pageref` render as themselves rather than being flattened to `\ref`. These keys were already computed
by S3 (`resolve_references().unresolved`) but grouped-by-source by **no** method until now. Every
S1–S17 output is left **byte-for-byte unchanged**; S18 is purely additive and leaves the `to_latex()`
round-trip fixed point intact.

- **One line per dangling reference, command preserved.** S3 walks every `\ref`/`\eqref`/`\pageref` in
  body pre-order and routes the dangling ones into `unresolved` as `UnresolvedRef { key, command,
  ref_span }`. S18 groups them by their shared `ref_span`, preserving the **first-appearance order** of
  the ref_spans (source order) via a `Vec<(Span, Vec<&UnresolvedRef>)>` — **not** a hash map — so the
  order is deterministic. Unlike a multi-key `\cite`, each reference takes exactly **one** key, so every
  group holds a single entry (the structural mirror of S17 is kept for readability). Each line is `\` +
  the reference's own `command` + `{` + its `key` + `}`, reconstructed from the owned `command`/`key`
  `String`s (no source slicing, matching S13 `resolve_namerefs`, S15 `citations_by_source`, and S17
  `unresolved_citations_by_source`) — so a dangling `\eqref{eq:x}` renders `\eqref{eq:x}` and a dangling
  `\pageref{p}` renders `\pageref{p}`, never a hard-coded `\ref`.
- **Only dangling references shown.** A `\ref` that resolves to a `\label` never enters `unresolved`, so
  it is excluded by construction. Lines are joined by `\n` with **no** trailing newline (matching S15
  `citations_by_source` and S17 `unresolved_citations_by_source`).
- **Empty marker.** A document with **no** unresolved references — every reference resolves, or there
  are none at all — returns the fixed marker `"(no unresolved references)"`, so the output is never the
  empty string (the same stable-marker discipline S12/S13/S14/S15/S16/S17 use).
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
  `command` and `key` are already owned `String`s); a single pass over the already-bounded `unresolved`
  list. Borrows `self` immutably, returns owned `String`.

Six `s18_*` tests pin the exact rendered strings: a single dangling `\ref{nope}` (`\ref{nope}`), a
dangling `\eqref{eq:ghost}` and `\pageref{p:ghost}` preserving their commands
(`\eqref{eq:ghost}\n\pageref{p:ghost}`), two distinct dangling `\ref`s in source order
(`\ref{nope1}\n\ref{nope2}`), a resolved `\ref` excluded so a fully-resolved document returns the
`(no unresolved references)` marker, a reference-free document returning the same marker, and an
additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`,
`list_summary`, `citations_by_source`, `duplicate_bibliography_entries`, and
`unresolved_citations_by_source` are all byte-for-byte unchanged.

## [0.53.0] — 2026-07-07

### Added — unresolved (dangling) citations grouped by source `\cite` (LTXDOC03 S17)

A **new** public method `Document::unresolved_citations_by_source(&self) -> String` that surfaces the
**unresolved (dangling) citations grouped per source `\cite`** — the DANGLING-key mirror of S15's
`citations_by_source`, and the citation-family parallel of S6's flat *"Dangling citations"* footer but
rendered **per source `\cite`** (a distinct new view). These keys were already computed by S2
(`resolve_citations().unresolved`) but grouped-by-source by **no** method until now. Every S1–S16
output is left **byte-for-byte unchanged**; S17 is purely additive and leaves the `to_latex()`
round-trip fixed point intact.

- **One line per source `\cite` with ≥1 dangling key.** S2 flattens every `\cite` into per-key rows,
  splitting them into resolved keys and unresolved (dangling) keys, each tagged with the citing
  `\cite`'s `cite_span`. S17 groups the *dangling* keys by their shared `cite_span`, preserving the
  **first-appearance order** of the cite_spans (source order) via a `Vec<(Span, Vec<&str>)>` — **not**
  a hash map — so the order is deterministic. Keys within a group stay in left-to-right order. Each
  line is `\cite{` + that group's dangling keys joined by `", "` + `}`, reconstructed from the owned
  `key` `String`s (no source slicing, matching S13 `resolve_namerefs`, S15 `citations_by_source`, and
  S16 `duplicate_bibliography_entries`).
- **Only dangling keys shown.** Because `unresolved` holds only the dangling keys, a `\cite{a, ghost}`
  where `a` resolves and `ghost` dangles renders `\cite{ghost}` — the exact analogue of how S15 shows
  only the *resolved* keys of a mixed `\cite`. Lines are joined by `\n` with **no** trailing newline
  (matching S11 `to_plain_text_by_kind`, S12 `list_of_floats`, S13 `resolve_namerefs`, S14
  `list_summary`, S15 `citations_by_source`, S16 `duplicate_bibliography_entries`).
- **Empty marker.** A document with **no** unresolved citations — every cited key resolves, or there
  are no citations at all — returns the fixed marker `"(no unresolved citations)"`, so the output is
  never the empty string (the same stable-marker discipline S12/S13/S14/S15/S16 use).
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all — keys
  are already owned `String`s); a single pass over the already-bounded `unresolved` list. Borrows
  `self` immutably, returns owned `String`.

Six `s17_*` tests pin the exact rendered strings: a single dangling `\cite{ghost}` (`\cite{ghost}`), a
mixed `\cite{known, ghost}` showing only the dangling key (`\cite{ghost}`), a fully-dangling
`\cite{x, y}` reuniting both keys on one line (`\cite{x, y}`), two distinct dangling `\cite`s in source
order (`\cite{ghost1}\n\cite{ghost2}`), an all-resolved document returning the
`(no unresolved citations)` marker, and an additivity check that `to_plain_text`,
`to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`, `citations_by_source`,
and `duplicate_bibliography_entries` are all byte-for-byte unchanged.

## [0.52.0] — 2026-07-07

### Added — duplicate (multiply-defined) bibliography entries (LTXDOC03 S16)

A **new** public method `Document::duplicate_bibliography_entries(&self) -> String` that surfaces the
**duplicate (multiply-defined) `\bibitem` entries** — LaTeX's *"Citation `key' multiply defined"*
warnings. These were already computed by S2 (`resolve_citations().duplicate_entries`) but rendered by
**no** method until now. It is the citation-family parallel of S6's *"Dangling citations"* footer, for
the *other* bibliography warning. Every S1–S15 output is left **byte-for-byte unchanged**; S16 is
purely additive and leaves the `to_latex()` round-trip fixed point intact.

- **One line per losing duplicate, in pre-order.** S2 collects every `\bibitem{key}` in `walk`
  pre-order; the **first** of each key wins, and every **later** `\bibitem` of an already-defined key
  is a losing duplicate in `duplicate_entries`. S16 emits one line per duplicate, in that existing
  pre-order (**not** re-sorted). Each line is the offending command **reconstructed from its key**:
  `\bibitem{` + the duplicate's key + `}`. We reconstruct from the owned key rather than slice
  `&src[span]` (matching S13 `resolve_namerefs` and S15 `citations_by_source`), so the render needs no
  source borrow and can never index out of bounds.
- **Every duplicate, never de-duplicated.** If a key is defined *three* times, both the second and the
  third lose, so two `\bibitem{key}` lines are emitted (one per *"multiply defined"* warning LaTeX
  would raise) — the point is to surface every warning, not the fact that a key is duplicated. The
  winning first `\bibitem` is never listed (it is an entry, not a duplicate). Lines are joined by `\n`
  with **no** trailing newline (matching S11 `to_plain_text_by_kind`, S12 `list_of_floats`, S13
  `resolve_namerefs`, S14 `list_summary`, S15 `citations_by_source`).
- **Empty marker.** A document with **no** duplicate entries — no bibliography, or every key defined
  exactly once — returns the fixed marker `"(no duplicate bibliography entries)"`, so the output is
  never the empty string (the same stable-marker discipline S12/S13/S14/S15 use).
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all — keys
  are already owned `String`s); a single pass over the already-bounded `duplicate_entries` list.
  Borrows `self` immutably, returns owned `String`.

Four `s16_*` tests pin the exact rendered strings: a bibliography defining `smith` twice and `jones`
once (`\bibitem{smith}`, only the loser), two distinct keys each defined twice
(`\bibitem{a}\n\bibitem{b}` in pre-order), a no-duplicate bibliography returning the
`(no duplicate bibliography entries)` marker, and an additivity check that `to_plain_text`,
`to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`, and
`citations_by_source` all still produce their exact prior strings.

## [0.51.0] — 2026-07-07

### Added — resolved citations grouped by their source `\cite` (LTXDOC03 S15)

A **new** public method `Document::citations_by_source(&self) -> String` that renders the resolved
citations **grouped by the source `\cite` construct they came from** — the citation-family parallel
of S11's `to_plain_text_by_kind` (which groups resolved *references*) and S13's `resolve_namerefs`
(one rendered line per target). It reads only `resolve_citations().resolved` and re-assembles the
per-key rows that S2 flattened out of each multi-key `\cite`. Every S1–S14 output is left
**byte-for-byte unchanged**; S15 is purely additive and leaves the `to_latex()` round-trip fixed
point intact.

- **Groups by `cite_span`, in source order.** S2 emits one `ResolvedCite` per key, every key of a
  `\cite{a,b}` sharing that one `\cite`'s `cite_span`. S15 groups the `resolved` rows back by
  `cite_span`, preserving the **first-appearance order** of the cite_spans (source order of the
  `\cite`s, since `resolved` is already in body pre-order) and keeping keys within a group in their
  original left-to-right order.
- **One line per source `\cite`, reconstructed from its resolved keys.** Each line is
  `\cite{` + the group's resolved keys joined by `", "` + `}`. A **dangling** key (one no `\bibitem`
  defines) never entered `resolved`, so it is **excluded** by construction: a `\cite{a,ghost}` where
  only `a` resolves renders `\cite{a}`, not `\cite{a,ghost}`. We reconstruct rather than slice the
  raw `&src[cite_span]` precisely because the source text would still contain the dangling `ghost`;
  reconstruction shows exactly what *bound*. Lines are joined by `\n` with **no** trailing newline
  (matching S11 `to_plain_text_by_kind`, S12 `list_of_floats`, S13 `resolve_namerefs`, S14
  `list_summary`).
- **Empty marker.** A document with **no** resolved citations — none present, or every cited key
  dangling — returns the fixed marker `"(no resolved citations)"`, so the output is never the empty
  string (the same stable-marker discipline S12/S13/S14 use).
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
  keys are already owned `String`s); a single pass over the already-bounded `resolved` list. Borrows
  `self` immutably, returns owned `String`.

Four `s15_*` tests pin the exact rendered strings: a doc grouping a multi-key `\cite{a,b}` and a
separate `\cite{c}` (`\cite{a, b}\n\cite{c}`), a partial `\cite{a,ghost}` rendering only the
resolved key (`\cite{a}`), an all-dangling doc returning the `(no resolved citations)` marker, and
an additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`,
`resolve_namerefs`, and `list_summary` all still produce their exact prior strings.

## [0.50.0] — 2026-07-07

### Added — per-kind census of the numbered-label table (LTXDOC03 S14)

A **new** public method `Document::list_summary(&self) -> String` that renders a compact per-kind
count of the document's **numbered** labels — how many sections, figures, tables, and equations
carry a `\label`. It is a pure tally of the rows `number_labels()` returns, grouped by `LabelKind`,
so the census can never drift from the S4 numbering it summarises. Every S1–S13 output is left
**byte-for-byte unchanged**; S14 is purely additive and leaves the `to_latex()` round-trip fixed
point intact.

- **Counts exactly the numberable kinds.** Only a numbered `\section`, a `figure`, a `table`, and a
  non-starred display `equation` label reach `number_labels()`, so those are the only kinds counted.
  A bare inline `\label{…}` (`LabelKind::Inline`) is **not** numbered — it never appears in
  `number_labels()` — and is therefore counted nowhere. Confirmed by reading the numbering pass
  (it records no `Inline` rows) and by an exploratory tally over a mixed fixture.
- **One line per non-zero kind, in a fixed order.** The output emits `"Sections: n"`, `"Figures: n"`,
  `"Tables: n"`, `"Equations: n"` — in that fixed order (deterministic, never document order) — with
  a **fixed plural** label regardless of `n` (a single section still prints `Sections: 1`). A kind
  whose count is 0 is **omitted** entirely, mirroring S11's "kinds with 0 refs are omitted"
  convention. Lines are joined by `\n` with **no** trailing newline (matching S11
  `to_plain_text_by_kind`, S12 `list_of_floats`, S13 `resolve_namerefs`).
- **Empty marker.** A document with **no** numbered label at all → the fixed marker `"(no labels)"`,
  so the output is never the empty string (the same stable-marker discipline S12/S13 use).
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; a single pass over the
  already-bounded numbering table. Borrows `self` immutably, returns owned `String`.

Four `s14_*` tests pin the exact rendered strings: a doc counting all four kinds
(`Sections: 2\nFigures: 1\nTables: 1\nEquations: 1`), a sections-only doc (`Sections: 3`, zero-count
kinds omitted), an equation/inline-only doc returning the empty `(no labels)` marker, and an
additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, and
`resolve_namerefs` all still produce their exact prior strings.

## [0.49.0] — 2026-07-07

### Added — `\nameref` resolution to a target's name text (LTXDOC03 S13)

A **new** public method `Document::resolve_namerefs(&self) -> String` that resolves every
`\nameref{key}` in the body to the **name** (title/caption text) of its target — the `nameref`
package's name-valued sibling of `\ref` (number-valued) and `\pageref` (page-valued). A
`\nameref{sec:intro}` yields the section's title (`Introduction`), not "Section 1"; a
`\nameref{fig:p}` yields the figure's caption text. Every S1–S12 output is left **byte-for-byte
unchanged**; S13 is purely additive.

- **`\nameref` is not a `REF_COMMAND`.** The S1 resolver binds only `\ref`/`\eqref`/`\pageref`, so a
  `\nameref` appears in *neither* the resolved *nor* the unresolved reference table. Confirmed by an
  AST probe: `\nameref{sec:intro}` lowers to `Inline::CrossRef { command: "nameref", target:
  "sec:intro", .. }`, and `resolve_references()` returns it in no table. That is *why* S13 is a brand
  new method rather than a tweak to `REF_COMMANDS` — it reads the same `\label` table S1 builds but
  answers a different question (*what is it called?*), touching no existing output.
- **One line per `\nameref`, in body order, formatted `\nameref{<key>} -> <name>`** (mirroring the
  S6 `\ref{k} -> …` arrow). A single document-order `walk()` collects every `nameref` cross-ref, then
  each key is resolved against the winning label table (`ReferenceResolution::definition`, the same
  first-wins table `\ref` uses) and the target's name read from its defining node via the S3
  `label_def_node` accessor.
- **Name text per kind.** A `Section` target → its `title` inlines flattened; a `Figure`/`Table`
  target → its `\caption` via the S12 `caption_text` helper (so a `\nameref` and the List-of-Floats
  entry read the *same* caption). An `Equation`/`Inline` target carries no name (a number, not a
  title) → the fixed marker `(no name)`. An undefined key → `(undefined nameref: <key>)`. A document
  with no `\nameref` at all → `(no namerefs)`.
- **Shared flatten helper.** S12's caption-flattening descent (`Text`/`Code` verbatim, `Space` → one
  space, `Strong`/`Emph`/`Styled` recursed, trimmed) is factored into a module-level
  `flatten_inlines_to_text(&[Inline]) -> String`, reused by both `caption_text` (captions) and
  `resolve_namerefs` (section titles) so the two name-rendering paths can never drift.
- **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; reuses the bounded `walk()`,
  the S1 `resolve_references` table, and the S3 `label_def_node` accessor. Borrows `self` immutably,
  returns owned `String`.

Five `s13_*` tests pin the exact rendered strings: section+figure names, an undefined-key
placeholder, equation/inline `(no name)`, the empty `(no namerefs)` marker, and an additivity
check that `\namerefs` stay out of the resolved/unresolved ref tables and leave `list_of_floats`
unchanged.

## [0.48.0] — 2026-07-07

### Added — List of Figures / List of Tables index (LTXDOC03 S12)

A **new** public method `Document::list_of_floats(&self) -> String` that renders the document's
**List of Figures** and **List of Tables** — LaTeX's `\listoffigures` / `\listoftables`, as plain
text — directly from the document's floats. Real LaTeX gates these on a `\listoffigures` /
`\listoftables` command, but those are not parser-recognised commands here, so — like S11's grouped
report — S12 is exposed as a method the caller invokes rather than a gated render. Every S1–S11
output is left **byte-for-byte unchanged**; S12 is purely additive.

- **Every float gets a numbered line, in document order.** A single document-order walk threads the
  same `Counters` float counters `number_labels` uses, so each float's line number equals the flat
  figure/table counter's value at that float — a labeled float's List-of number and its `\ref`
  number agree, and the two renderings can never drift. Figures are numbered `1, 2, 3, …`; tables
  are numbered independently from `1`.
- **`<n>. <caption text>` per line.** The caption text is the plain rendering of the float's
  `\caption{…}` inlines — `Text`/`Code` runs verbatim, `Space` as a single space, and the text
  inside font wrappers (`\textbf`/`\emph`/`\texttt`/other `Styled`) recursively, then trimmed. This
  is the same descent the `ref_target_node_for_figure_reaches_its_caption` test exercises, factored
  into a private `caption_text(&Option<Caption>) -> String` helper.
- **Uncaptioned floats keep their line.** A float with **no** `\caption` renders the fixed
  placeholder `(no caption)`, so every float still gets a numbered line and the numbering stays
  aligned with the real float count.
- **Optional blocks, distinct empty marker.** The `List of Figures` heading is emitted **only** when
  there is ≥1 figure; `List of Tables` **only** when there is ≥1 table. A document with **no** floats
  returns the fixed marker `"(no floats)"`. Lines are joined by `\n` with no trailing newline.
- **Additive, no AST/grammar/counter change.** A pure assembly method over existing document blocks,
  the existing `Counters` float walk, and existing caption extraction. No new field, no new counter
  type, no new dependency, no `unsafe`, no I/O; `to_latex()` remains a fixed point. 3 new S12 tests;
  version bump 0.47.0 → 0.48.0.

## [0.47.0] — 2026-07-07

### Added — cross-reference report: grouped-by-kind rendering (LTXDOC03 S11)

A **new, separate** public method `CrossReferenceReport::to_plain_text_by_kind(&self) -> String` that
renders the **same** resolved references as `to_plain_text`, but **grouped under fixed-order kind
subheadings** instead of flat source order — so a reader can see "which sections / figures / equations
does this document cross-reference?" at a glance. `to_plain_text` is left **byte-for-byte unchanged**;
S11 is purely additive.

- **Fixed kind order, source-order within a group.** Groups are emitted **Sections, Figures, Tables,
  Equations, Inline** regardless of source order (`S11_KIND_ORDER`). Within a group the refs keep the
  report's existing pre-order (a filter of `refs` for that kind, preserving order).
- **Subheading + two-space-indented lines.** Each kind with ≥1 resolved ref emits a pluralised
  capitalised subheading (`Sections:`, `Figures:`, `Tables:`, `Equations:`, `Inline:`, from the new
  `kind_group_heading` helper) followed by one two-space-indented line per ref. Kinds with zero
  resolved refs are **omitted entirely** — no empty subheading.
- **Same per-command rendering — factored into a shared helper.** The single-line rendering (S8/S9/S10
  rules: `\eqref` to an equation → `\eqref{k} -> Equation (N)`; `\pageref` → `\pageref{k} -> page ?`;
  else `\ref{k} -> Kind N`) is factored into a private `render_resolved_ref(&RefEntry) -> String` that
  **both** `to_plain_text` and `to_plain_text_by_kind` call, so the flat and grouped renderings can
  never drift. `to_plain_text` was refactored to call it with byte-for-byte identical output (the
  existing S6–S10 tests pass unchanged). A `\pageref` groups under its **target kind** (e.g. a
  `\pageref` to a section sits under `Sections:`).
- **Resolved refs only; distinct empty marker.** Citations and dangling footers are **not** included
  (the flat `to_plain_text` remains the full report). A report with **zero** resolved refs renders the
  fixed string `"(no resolved references)"` — the S11 analogue of `to_plain_text`'s
  `"(no cross-references)"`.
- **Additive, no AST/struct/numbering change.** A pure report-assembly method over data the report
  already holds (`refs` with their `kind`). No new field, no new dependency, no `unsafe`, no I/O;
  `to_latex()` remains a fixed point. 5 new S11 tests; version bump 0.46.0 → 0.47.0.

## [0.46.0] — 2026-07-07

### Added — cross-reference resolution: distinct `\pageref` rendering (LTXDOC03 S10)

Closes the surface-form gap S8/S9 left for the *page* reference family. A `\pageref{key}` asks "what
**page** is the target on", a fundamentally different question from `\ref`'s "what **number** is the
target". Through S9 the report conflated the two: a resolved `\pageref` rendered **identically** to a
`\ref` — `\ref{key} -> Kind N`. The crate has no page model, so it cannot compute a real page number,
but it can at least render the page family *honestly and distinctly*:

- **`\pageref` renders `page ?`.** In `CrossReferenceReport::to_plain_text`, a resolved reference
  whose `command == "pageref"` (to **any** target kind — Section/Table/Figure/Equation/Inline) now
  renders `\pageref{sec:i} -> page ?` — the `\pageref` spelling is kept and the number/kind are
  replaced by the fixed literal placeholder `page ?`. The `?` mirrors LaTeX's own `??` for an
  unresolved page reference (and the S7 number-placeholder pattern): it means "page number not
  modelled", NOT the kind and NOT the number.
- **A `\pageref` ignores kind entirely.** Because a page reference is about location, not identity,
  the `\pageref` branch takes precedence over the S8 else-branch and is orthogonal to the S9 amsmath
  branch: a `\pageref` to an equation still renders `page ?`, never `Equation (1)` or `Equation 1`.
- **`\ref` and `\eqref` are byte-for-byte unchanged.** Branch precedence is (1) `\eqref` to Equation →
  parenthesised (S9); (2) `\pageref` any kind → `page ?` (S10, NEW); (3) else → `\ref{key} -> Kind N`
  (S8). Only `\pageref` lines change.
- **Additive, no AST/struct/numbering change.** `RefEntry.command` (the surface spelling) was already
  retained by S1, so S10 is a pure rendering branch in one loop. No AST change, no new field, no
  re-numbering; `to_latex()` remains a fixed point.

## [0.45.0] — 2026-07-06

### Added — cross-reference resolution: `\eqref` parenthesisation (LTXDOC03 S9)

Closes the surface-form gap S8's "Deferred to S9" note flagged. amsmath's `\eqref{eq:e}` typesets the
equation number **parenthesised** — `(1)` — whereas a plain `\ref{eq:e}` typesets a bare `1`. Through
S8 the cross-reference report ignored the surface command and rendered every reference with the
canonical `\ref` prefix and a bare number. S9 makes the report mirror amsmath for the one case that
matters:

- **`\eqref` to an equation parenthesises.** In `CrossReferenceReport::to_plain_text`, a resolved
  reference whose `command == "eqref"` **and** whose `kind == LabelKind::Equation` now renders
  `\eqref{eq:e} -> Equation (1)` — the `\eqref` spelling is kept and the number is wrapped in
  parentheses.
- **Everything else is byte-for-byte unchanged.** All `\ref`, all `\pageref`, and any `\eqref` to a
  **non-equation** kind still render with the canonical `\ref` prefix and a bare number
  (`\ref{sec:intro} -> Section 1.2`), exactly as through S8.
- **Additive, no AST/struct/numbering change.** `RefEntry.command` (the surface spelling) was already
  retained by S1 and populated by `cross_reference_report`, so S9 is a pure rendering split in one
  `format!`. No AST change, no new field, no re-numbering; `to_latex()` remains a fixed point.

## [0.44.0] — 2026-07-06

### Added — cross-reference resolution: equation numbering (LTXDOC03 S8)

Closes the numbering gap S7 left open. S7 made a `\ref`/`\eqref` to a display-math `\label` *resolve*
and appear in the S6 cross-reference report, but the number it carried was the placeholder
`EQUATION_NUMBER_PLACEHOLDER` (`"?"`) — the report printed `Equation ?`. S8 wires the real
`\theequation` counter so the report prints `Equation 1`, `Equation 2`, … in document order.

- **New flat equation counter.** `Counters` gains an `equation: u32` field (initialised to `0` in
  `new()`) and a `step_equation(&mut self) -> u32` method that pre-increments (saturating) and returns
  the new value — mirroring `step_figure`/`step_table` exactly. Equations are numbered on a single
  monotonic run, **independent** of the section/figure/table counters (the `article` default, where
  `\theequation` is not reset per section).
- **Labelled equations get a real number.** In the `Block::DisplayMath { label: Some(key), .. }` arm of
  `Document::number_labels`, the placeholder is replaced with `counters.step_equation().to_string()`.
  A single labelled equation numbers `1`; two in document order number `1` then `2`; a `\section`,
  figure, or table between/around them does not perturb the equation sequence (each counter is its own
  run).
- **S6 report now prints the number.** A resolved `\ref`/`\eqref` to an equation label renders
  `\ref{eq:e} -> Equation 1` (was `-> Equation ?` in S7).
- **`EQUATION_NUMBER_PLACEHOLDER` retained.** The constant stays `pub`/re-exported (still referenced by
  the module's intra-doc links and available to S9+ `\eqref` parenthesisation); only its former
  code use in the numbering arm is replaced.

### Known limitation — unlabelled numbered equations

In real LaTeX *every* non-starred display equation consumes the equation counter, `\label` or not
(like figures/tables). Our AST only marks the **labelled** non-starred case: `Block::DisplayMath`
carries no `numbered` flag, and the D5 lowering sets `label: None` for *both* starred envs and
unlabelled islands (`\[…\]`, `$$…$$`), so an unlabelled-but-numbered `equation` env is
indistinguishable from an unnumbered island. S8 therefore steps the counter **only** for labelled
equations. Consequence: an unlabelled numbered equation sitting between two labelled ones leaves the
second labelled one's number one lower than a full LaTeX run would assign. Closing this gap needs a
`numbered: bool` on `Block::DisplayMath` (an AST change) and is deferred to a later slice.

### Deferred to S9 — `\eqref` parenthesisation

The S6 report renders every reference as `\ref{key} -> Kind number` (canonical `\ref` spelling,
bare number), so `\eqref` does **not** yet parenthesise to `(1)`. That surface distinction is a
later slice; S8 is counter-only.

## [0.43.0] — 2026-07-06

### Added — cross-reference resolution: equation-label lifting (LTXDOC03 S7)

Closes the one gap S6 left open. A `\ref`/`\eqref` to a `\label` that sits **inside** a display-math
environment (`\begin{equation} E=mc^2 \label{eq:e} \end{equation}`) *resolved* in S1 but had **no** S4
number, because `Block::DisplayMath` kept its whole body as one raw `source` string — the `\label` was
swallowed into that string and never became a real label definition. So S6's `cross_reference_report()`
**omitted** such refs (they were neither dangling nor renderable). S7 fixes exactly that.

- **Lifts the `\label` out of the env body.** For a **non-starred** display-math environment
  (`equation`, `align`, `gather`, `multline`, `eqnarray` — *not* the starred `equation*`/… forms, which
  are unnumbered in LaTeX), the D5 lowering now pulls the first `\label{key}` out of the env body onto
  the block and **removes** it from `source` (no duplication). `\begin{equation} E = mc^2 \label{eq:e}
  \end{equation}` → `DisplayMath { source: "E = mc^2", label: Some("eq:e") }`.
- **New `LabelKind::Equation`.** The lifted label is registered — in the *same* collection pass that
  registers section/figure/table labels — as a real `LabelDef` tagged `LabelKind::Equation`
  (`as_str()` → `"equation"`, display name `"Equation"`). So an `\eqref{eq:e}`/`\ref{eq:e}` now
  **resolves** to it.
- **`DisplayMath` carries the label.** `Block::DisplayMath` gains a `label: Option<String>` field,
  mirroring how `Block::Figure`/`Block::Table` carry their lifted label. It is `Some` only for the
  non-starred named-env path; the starred forms and the `\[…\]`/`$$…$$` islands keep `label: None`
  (unchanged behaviour).
- **No longer omitted from the S6 report.** A resolved `\ref`/`\eqref` to an equation label is now
  **included** in `cross_reference_report().refs`, rendering `\ref{eq:e} -> Equation ?`.
- **Numbering deferred to S8.** S7 does *not* wire the equation counter (`\theequation`). The equation
  label carries the placeholder number `EQUATION_NUMBER_PLACEHOLDER` (`"?"`, echoing LaTeX's `??`) in
  the report — enough to make it resolvable-and-reported; the real number is a future S8 rung.
- **Round-trip preserved.** `Document::to_latex()` re-emits a lifted-label equation as
  `\begin{equation}<body> \label{key}\end{equation}` (not `$$…$$`, which would drop the label), so
  `parse(doc.to_latex())` re-lifts to an equal AST — the round-trip fixed point still holds.
- **New/changed API.** `LabelKind::Equation`; `Block::DisplayMath { source, label, span }` (added
  `label`); `pub const EQUATION_NUMBER_PLACEHOLDER: &str = "?"`, exported from the crate root. Additive
  and pure: no existing S1–S6 behaviour changed except that equation-label refs stop being omitted.

## [0.42.0] — 2026-07-06

### Added — cross-reference resolution: the cross-reference report (LTXDOC03 S6)

The **consumer** rung that proves S1–S5 compose. S1 bound each `\ref`, S2 each `\cite`, S4 numbered
the labels, S5 numbered the citations — but nothing yet **assembled** them into a single
consumer-facing artifact. S6 is that assembly: one method that walks S1's resolved `\ref`s and S2's
resolved `\cite`s and produces an owned, plain-data report where each entry carries its rendered
**number** (from S4/S5) alongside its key/command/kind. No new AST walk, no new parsing.

- **Composes the five passes.** `Document::cross_reference_report()` runs S1 + S2, numbers each family
  **once** with S4 (`number_labels`) / S5 (`number_citations`), then *looks each key up* in the
  resulting number table — never a per-entry re-numbering (the anti-pattern `ref_number`/`cite_number`
  warn about in a loop). The whole report costs a constant number of the existing bounded passes.
- **Two resolved families.** A resolved `\ref` with an S4 number → a `RefEntry { key, command, kind,
  number }` (`\ref{sec:intro}` → Section `"1.2"`); a resolved `\cite` key → a `CiteEntry { key,
  number }` (`\cite{b}` → `"[2]"`). A multi-key `\cite{a,b}` yields one `CiteEntry` per key (S2 already
  split them), numbered `[1]`/`[2]` independently.
- **Dangling refs/cites surfaced *separately*.** A `\ref{missing}` (S1's `unresolved`) and a
  `\cite{ghost}` (S2's `unresolved`) — LaTeX's `??` / `[?]` undefined markers — go in their own
  `dangling_refs` / `dangling_cites` key vectors, never folded in among the resolved entries. This
  makes "resolved vs dangling" a type-level fact rather than a field the caller must remember to check.
- **The one subtlety, documented.** A resolved `\ref` to an *inline/equation* `\label` (which S4 leaves
  unnumbered — deferred) has no number, so it is **omitted** from `refs` (it is neither dangling — its
  label exists — nor renderable). Every row in `refs` therefore carries a real number, no placeholder.
  Citations have no analogous gap (a winning `\bibitem` is always S5-numbered).
- **A stable plain-text rendering.** `CrossReferenceReport::to_plain_text()` renders a deterministic,
  pinned string: `\ref{<key>} -> <Kind> <number>` lines, then `\cite{<key>} -> <number>` lines, then
  optional `Dangling references: …` / `Dangling citations: …` footers — joined by single `\n`, no
  trailing newline. An empty report renders the fixed marker `(no cross-references)` (never the empty
  string).
- **New API.** `Document::cross_reference_report() -> CrossReferenceReport { refs: Vec<RefEntry>,
  cites: Vec<CiteEntry>, dangling_refs: Vec<String>, dangling_cites: Vec<String> }` with `RefEntry`,
  `CiteEntry`, and `CrossReferenceReport::to_plain_text() -> String`. All owned plain data (`String`s +
  `Copy` `LabelKind`), mirroring S4/S5, exported from the crate root.
- **Pure & additive.** The S1–S5 result types are unchanged; S6 only *reads* their results and copies
  owned data out, mutating nothing about the tree or any prior pass. Total & panic-free (no
  `unwrap`/`expect`, no unchecked indexing), reusing the bounded passes (no new recursion).

### Deferred (honest boundary, inherited from S4/S5)

- **Equation numbers** — a `\ref` to an equation/inline label is omitted from the report (S4 does not
  number those yet; an equation body is an opaque `Block::DisplayMath` string with no label field).
- **Author-year / natbib sorted citation styles** and **external `.bib`/`.bbl` databases** — out of
  scope at S2/S5, so the report covers only what those passes resolved (in-document `thebibliography`,
  numeric/unsorted style).

## [0.41.0] — 2026-07-06

### Added — cross-reference resolution: citation numbering (LTXDOC03 S5)

The bracketed number a `\cite` actually *prints* — the `[2]` in "as shown in [2]". S4 numbered
sections and floats but explicitly left **citations** unnumbered; S5 fills that gap over the
bibliography S2 already resolved. It is the citation-family analogue of S4's `Document::ref_number`.

- **Listing-order bibliography numbers.** In the default numeric/unsorted style, each `\bibitem` is
  numbered by its **position in the list**: the first is `[1]`, the second `[2]`, …. S5 numbers S2's
  already-ordered winning `entries` by their index — `entries[0]` → `[1]`, `entries[1]` → `[2]` — so
  the number matches LaTeX's list position exactly.
- **First-`\bibitem`-wins duplicates consume no number.** A key defined by two `\bibitem`s puts the
  first in `entries` and the second in `duplicate_entries`; the losing duplicate is not in `entries`,
  so it neither adds a row nor advances the counter — a re-declared entry is numbered the same as its
  first declaration, and the entries after it are **unshifted** (with `a, b, c` and a later duplicate
  `\bibitem{a}`, `c` stays `[3]`, not `[4]`).
- **Dangling `\cite`s are unnumbered.** A `\cite{missing}` whose key has no `\bibitem` is in S2's
  `unresolved`, so it carries no `ResolvedCite` and there is no entry to number — `number_for` returns
  `None` (LaTeX's `[?]` case), never a panic.
- **Bracket style single-sourced.** The `[n]` rendering lives in one `render_cite_number` helper.
- **New API.** `Document::number_citations() -> CitationNumbering { entries: Vec<NumberedCitation> }`
  with `NumberedCitation { key, ordinal, number }` (owned `String`s + `Copy` ordinal, mirroring S4's
  `Numbering`/`NumberedLabel`); `CitationNumbering::number_for(key) -> Option<&str>` (allocation-free
  lookup); and the S2→S5 payoff `Document::cite_number(&ResolvedCite) -> Option<String>` returning a
  resolved `\cite`'s bracketed number (`"[2]"`), or `None` for a non-entry key (total, never a panic).
- **Additive & pure.** S1-S4 result types are unchanged; S5 reads S2's `CitationResolution` and
  produces a new owned aggregate, mutating nothing about the tree or any prior pass.

### Deferred (honest boundary, unchanged from S4)

- **Equation numbers** remain future work: an equation body is an opaque `Block::DisplayMath` raw
  source string with **no** `label` field (an equation's `\label` is buried inside that string, not a
  resolvable label def), so per-equation numbering would need fuzzy string heuristics. Citation
  numbering is well-defined on S2's clean owned data, so S5 does citations; equation numbering stays a
  documented future rung blocked on the `DisplayMath` AST shape.
- **Author-year / natbib sorted styles** (`plainnat`, `alpha`, …) that renumber, re-*label*, or sort
  entries, and **external `.bib`/`.bbl` databases** (S5 does no file I/O, parses no BibTeX), also
  remain future rungs.

## [0.40.0] — 2026-07-06

### Added — cross-reference resolution: document numbering (LTXDOC03 S4)

The number a `\ref` actually *prints*. S1 bound each `\ref` to its target's **bytes** and S3 lifted
that to the target **node**, but neither gives the rendered **number** ("Section 1.2", "Figure 3").
S4 assigns those numbers in one walk over the parsed `Document` — the static analogue of LaTeX's
second `.aux` pass, computing each numbered target's value directly (no `.aux` file, no second parse).

- **Hierarchical section numbers with deeper-reset.** A numbered `\section`…`\subparagraph` shares a
  nested counter family: incrementing a coarser level **resets every finer one to 0**, and the number
  is the dotted join from the top level down to that depth — `1`, `1.1`, `1.2`, `1.2.1`, `2`. A
  starred `\section*` (`numbered == false`) fires **no** counter and is skipped, so the next numbered
  section keeps the number it would have had (a `\section*` between `1` and the next `\section` leaves
  it `2`, not `3`).
- **Flat, independent float counters.** `figure` and `table` each own a running counter that only
  increments: figures `1, 2, 3, …`, tables their **own** `1, 2, 3, …` (a table after two figures is
  `1`, not `3`). **Every** float advances its counter — labeled or not — mirroring LaTeX, where a
  `\label` merely *captures* the value; an unlabeled figure between two labeled ones takes `2`, so the
  labeled ones read `1` and `3`.
- **Missing-parent rule (a document that starts deep).** A `\subsection` before any `\section` has no
  opened parent, so its parent counter sits at its initial `0`; we render from the `\section` depth
  down, surfacing an honest leading `0` — a lone leading `\subsection` numbers `0.1`, a lone
  `\subsubsection` `0.0.1`. A plain top-level `\section` is just `1` (it *is* the reference depth). The
  rule is documented, deterministic, and total — never a panic on the degenerate input.
- **New public API.** `Document::number_labels(&self) -> Numbering` returns one owned `NumberedLabel`
  row per **defined, numberable** label key (section/figure/table), each carrying its `LabelKind` and
  rendered `number: String`, with a `Numbering::number_for(key) -> Option<&str>` lookup. The payoff
  convenience `Document::ref_number(&self, r: &ResolvedRef) -> Option<String>` ties S1 resolution to
  S4 numbering: `\ref{sec:intro}` → `"1.2"`. Inline/equation labels carry no S4 counter and are
  omitted (deferred to S5).
- **Deferred to S5+ (honest boundary).** Equation numbers, citation `[1]` order-of-first-appearance
  numbers, and other `\label`-able counters (enumerate items, theorems, footnotes) are **not** yet
  assigned — each needs a per-environment counter context S4 does not thread.
- **Pure, additive analysis.** No new parsing, no I/O, no tree mutation; reuses the bounded
  `Document::walk` (no new recursion), a fixed-size 7-slot counter array (no unchecked indexing), and
  no `unwrap`/`expect`. A regression test asserts numbering leaves the S1/S2/S3 outputs byte-for-byte
  unchanged. `to_latex` round-trip fixed point preserved.

## [0.39.0] — 2026-07-06

### Added — cross-reference resolution: target → `NodeRef` exposure (LTXDOC03 S3)

The natural depth-add on S1+S2. Both prior slices bound each `\ref`/`\cite` to the **bytes** of its
target (a `Span`) but not the target **node** itself. S3 exposes the actual walked `NodeRef` for a
resolved reference/citation (and for a label/bib definition), so a consumer can read the target's
`kind()` and — for a `Block` — descend into its children (a `\ref` to a section can now enumerate the
section's paragraphs; a `\ref` to a figure can reach its caption). No new parsing, numbering, or I/O
— pure, additive analysis over the existing `Document::walk`.

- **New load-bearing primitive:** `Document::node_for_span(&self, span: Span) -> Option<NodeRef<'_>>`
  — returns the walked body node whose span **exactly equals** `span` (half-open equality of *both*
  `start` and `end`), or `None` if no walked node matches. This is *equality*, not *containment*
  (containment is `node_at`'s job, S4): a resolved S1/S2 target span is, by construction, some walked
  node's own span, so equality is the correct predicate. O(nodes) — one reuse of the bounded `walk`,
  no new recursion, panic-free.
- **Ergonomic accessors (take a resolved record, return the node):**
  `Document::ref_target_node(&self, r: &ResolvedRef)`, `Document::cite_target_node(&self, c:
  &ResolvedCite)`, and `Document::label_def_node(&self, d: &LabelDef)` — each a thin wrapper over
  `node_for_span`. The caller never hand-threads spans.
- **Purely additive — S1/S2 result types unchanged.** `ResolvedRef`/`ResolvedCite`/`LabelDef`/… still
  carry only owned `Span`s (no lifetimes), so a resolution still outlives any borrow of the source;
  the `NodeRef` is fetched **on demand** through these `Document` methods (a `NodeRef` borrows the
  doc, so it cannot live on the owned result types). A regression test asserts calling the S3
  accessors leaves the S1/S2 resolutions byte-for-byte identical.
- **Reachability — verified, not assumed.** An exploratory parse confirmed **every** S1/S2 target
  span corresponds to **exactly one** walked node (zero identical-span collisions among walked nodes):
  a `\ref`→section/figure/table yields the `NodeRef::Block` (kind `"Section"`/`"Figure"`/`"Table"`);
  an inline `\eqref`→`\label` yields the `NodeRef::Inline` (kind `"CrossRef"`); and — the one
  genuinely uncertain case — a `\cite`→`\bibitem` **is** reachable: the `\bibitem` inside a
  `thebibliography` `Block::Environment` survives D2 as an `Inline::Raw` command inside a
  `Block::Paragraph`, which `walk` visits, so `cite_target_node` returns `Some(NodeRef::Inline)` (kind
  `"Raw"`), **not** `None`.
- **`None` is a documented, total outcome.** `node_for_span` returns `None` for any span that is not
  some walked body node's own span — an empty document, a preamble/metadata region (deliberately not
  walked), or a fabricated span between nodes — never a panic. Because every *resolved* target is a
  walked node, the accessors never return `None` for a genuine reference/citation in a well-formed
  document; `None` is reserved for the honest edge.
- **Tie-break (defensive, does not fire in practice).** If two walked nodes ever shared an identical
  span, `node_for_span` returns the **first in pre-order** (the outermost/earliest). No real target
  hits this — it is documented for determinism, not because callers reach it.
- **9 new tests** (all `#[cfg(test)]`, load-bearing — assert the NODE, not just `is_some`): a
  `\ref`→section returns the real `Block::Section` and we descend into its title + owned paragraph; a
  `\ref`→figure reaches its caption text; an inline `\eqref` returns the `CrossRef` inline
  span-matching `\label{eq:x}`; a `\cite`→`\bibitem` returns the walked `Inline::Raw` slicing back to
  exactly `\bibitem{key}`; `label_def_node` returns the defining node; a non-matching span → `None`
  (no panic); `node_for_span(target_span)` agrees with `ref_target_node`; empty-document lookups →
  `None`; and the S1/S2-unchanged additivity regression.
- **Out of scope (future rungs):** citation/reference *numbering* and *external BibTeX* remain
  deferred, as in S1/S2.

## [0.38.0] — 2026-07-06

### Added — cross-reference resolution: `\cite` → bibliography binding (LTXDOC03 S2)

The second document-feature slice, the *parallel* pass to S1 for the **other** cross-reference
family: `\cite`, which resolves against a **bibliography** (`thebibliography` / `\bibitem`) rather
than the `\label` table. Like S1, this is a pure, additive, single-pass analogue of LaTeX's two-pass
`.aux` machinery — it binds each citation *key* to the `\bibitem` that defines it with byte spans on
**both** sides, but never computes a citation number/sort order (BibTeX/`.bbl` territory).

- **Extends the `references` module** with `Document::resolve_citations(&self) -> CitationResolution`
  — a *separate* aggregate from S1's `ReferenceResolution`, reflecting that labels and bibliographies
  are two independent tables. Reuses the existing bounded `Document::walk` plus a `MAX_DEPTH`-bounded
  environment descent; no parser/fold/`walk`/`node_at`/span changes, no new *unbounded* recursion.
- **Bibliography table (entries).** Collects every `\bibitem{key}` **inside a `thebibliography`
  environment** in pre-order (a `\bibitem` surfaces as an `Inline::Raw`-wrapped generic
  `NodeKind::Command`, since `recognize_structure` does not fold it). Each `BibEntry` records the
  key and the **`\bibitem{key}` construct's own tight span** (`&src[span]` slices back to exactly
  `\bibitem{key}` — the trailing author/title/year prose has no entry delimiter, so it is not
  attributed; the honest, defensible span).
- **Duplicate detection, first-entry-wins.** A key defined by two `\bibitem`s records the **first**
  as the winner (citations resolve against it) and the later one as a `DuplicateBib` (LaTeX's
  *"Citation `x' multiply defined"*). Never panics, never drops.
- **Citation resolution, multi-key aware.** For every `\cite` (the `CITE_COMMAND` family), the
  `target` is split on commas into individual trimmed, non-empty keys, each resolved
  *independently*: found → `ResolvedCite` (the `\cite`'s own `cite_span` **and** the entry's
  `entry_span`); missing → `UnresolvedCite` (dangling key + `cite_span`, LaTeX's *"Citation `x'
  undefined"*). A multi-key `\cite{a,b,c}` yields one record **per key**, all sharing that one
  `\cite`'s span, so a caller sees both per-key resolution and which source `\cite` each came from.
  `\cite[note]{key}` keeps the note in the cross-ref's separate `note` field — the key stays exactly
  `key`.
- **External BibTeX still out of scope (honest boundary).** Only an **in-document**
  `thebibliography`/`\bibitem` bibliography is bound — no `.bib`/`.bbl` file I/O, no BibTeX parse, no
  citation numbering/sorting. A `\cite` whose key lives only in an external `.bib` is reported
  unresolved here.
- **Non-interfering with S1.** The two passes read disjoint command families (`\cite` vs
  `\ref`/`\eqref`/`\pageref`/`\label`) and produce disjoint result types; a regression test asserts a
  document with **both** `\ref`/`\label` and `\cite`/`\bibitem` resolves each cleanly with no leakage.
- **New public API (all `Clone`-able plain data; spans `Copy`, keys owned `String`s):** `BibEntry`,
  `DuplicateBib`, `ResolvedCite`, `UnresolvedCite`, `CitationResolution` (with an `entry(key)` lookup
  helper), and the `CITE_COMMAND` constant.
- **10 new tests** (all in `#[cfg(test)]`): a `\cite`→`\bibitem` resolves and *both* spans slice back
  to the exact source (load-bearing), a multi-key `\cite{a,b}` → two bindings sharing the `\cite`
  span, a mixed `\cite{known,unknown}` → one resolved + one unresolved from the same span,
  `\cite[p. 3]{key}` resolves with the note not conflated into the key, a dangling `\cite{ghost}` is
  unresolved, a duplicate `\bibitem{dup}` → first-entry-wins, a `\cite` with no bibliography is
  unresolved (no panic), an empty document yields empty results, the `entry(key)` helper, and the
  S1/S2 coexistence regression.

## [0.37.0] — 2026-07-06

### Added — cross-reference resolution: label table + `\ref` binding (LTXDOC03 S1)

The first document-feature slice on top of the now-complete LTXDOC02 precise-spans work: a pure,
additive **resolution pass** over a parsed `Document` that binds each cross-reference to the label
that defines it, with byte spans on **both** sides. This is the static, single-pass analogue of
LaTeX's two-pass `.aux` machinery — but it binds *structure* (which defining node, at which source
bytes) rather than computing numbers/pages.

- **New module `references` (`src/references.rs`)**, re-exported from the crate root:
  `Document::resolve_references(&self) -> ReferenceResolution`. Two linear passes over the existing
  bounded `Document::walk` traversal — no parser/fold/`walk`/`node_at`/span changes, no new
  recursion.
- **Label table (definitions).** Collects every DEFINED label in pre-order from two sources:
  (a) **hoisted** section/table/figure labels (`Block::Section`/`Table`/`Figure` `label: Some(key)`,
  span = the block's span), tagged `LabelKind::Section`/`Table`/`Figure`; and (b) **inline**
  `\label{key}` (`Inline::CrossRef { command: "label" }`, span = the cross-ref's span), tagged
  `LabelKind::Inline`.
- **Duplicate detection, first-def-wins.** A key defined more than once records the **first**
  definition as the winner (references resolve against it) and every later one as a `Duplicate`
  (LaTeX's *"Label `x' multiply defined"*). Never panics, never drops.
- **Reference resolution.** For every reference-family cross-ref — `REF_COMMANDS = {"ref", "eqref",
  "pageref"}` — looks the target up in the label table: found → `ResolvedRef` (the reference's own
  `ref_span` **and** the target definition's `target_span` + `target_kind`); missing → `UnresolvedRef`
  (the dangling key + `ref_span`, LaTeX's *"Reference `x' undefined"*).
- **`\cite` deferred (out of scope for S1).** A citation resolves against a **bibliography**, a
  separate table from the `\label` one, so it is a later rung: the ref pass treats `\cite` as neither
  resolved nor unresolved. (`\label` is likewise excluded from the *reference* side — it defines, it
  does not use.)
- **Public API (all `Clone`-able plain data; spans `Copy`, keys owned `String`s):** `LabelKind`,
  `LabelDef`, `Duplicate`, `ResolvedRef`, `UnresolvedRef`, `ReferenceResolution` (with a
  `definition(key)` lookup helper), and the `REF_COMMANDS` constant.
- **10 new tests** (all in `#[cfg(test)]`): `\ref`→section resolves and *both* spans slice back to
  the exact source (the load-bearing assertion), `\ref`→figure float resolves with `Figure` kind,
  an inline `\label`→`\eqref` resolves, a dangling `\ref` is unresolved with the correct ref-span, a
  twice-defined key is a duplicate with first-def-wins resolution, `\cite` is excluded from both
  ref tables, `\pageref` resolves like `\ref`, a `table` float label resolves with `Table` kind, the
  `definition()` helper, and an empty/label-free document yields empty results without panicking.

Total & panic-free (no `unwrap`/`expect`/unchecked indexing), no `unsafe`, `MAX_DEPTH`-bounded
(reuses `walk`), and the `to_latex` round-trip fixed point is untouched (this pass is pure analysis —
it never mutates the tree). New spec `LTXDOC03-cross-reference-resolution.md`.

## [0.36.0] — 2026-07-05

### Added — precise byte-coverage capstone; LTXDOC02 arc COMPLETE (LTXDOC02 S5)

The **capstone** of the precise-per-token-spans arc. Over the LTXDOC01 D6 representative
multi-construct corpus (`CAPSTONE_SRC`: a titled `article` with abstract, `\section`, `\textbf`,
inline `$…$` + display `equation` math, an `itemize`, a `tabular` in a `table` float, a `figure`
with caption+label, and a `\cite`), the new test proves the strong invariant the whole S1–S4 arc
was built to earn:

- **New capstone test `capstone_every_body_byte_resolves_to_tightest_covering_node`.** For **every**
  non-whitespace body byte `b`, it asserts (a) `node_at(b).is_some()` — it resolves — AND (b) the
  resolved node `n` is the **tightest-covering** walked node: no *other* walked node `m` whose span
  is a strict subset of `n`'s span (`m.start >= n.start && m.end <= n.end && m.span != n.span`) also
  contains `b`. This is the precise counterpart of D6's earlier *region-scoped* coverage test
  (`capstone_byte_coverage_body_region`), which only asserted that *some* node owns each byte.
- **Honest, not overclaimed.** The load-bearing assertion is **tightest-covering**, *not*
  "every byte is a `Text` leaf". Structural bytes (the `\section` / `\item` / `\begin{…}` machinery)
  and inter-child delimiters legitimately resolve to their enclosing composite (Section / List /
  Environment), which is genuinely the tightest cover there. The test additionally records that the
  *majority* of content bytes land on real leaves as a soft, non-load-bearing signal.
- **No `node_at`/parser/fold logic changed.** S1–S4 already made spans precise and `node_at`
  leaf-resolving; S5 is a pure test rung that formally verifies the tightest-covering invariant over
  the whole representative corpus.

The corpus is fixed and bounded, so iterating every body byte is O(len), not a DoS. Round-trip fixed
point, totality/no-panic, no `unsafe`, and `MAX_DEPTH`-bounded recursion all preserved. Spec
`LTXDOC02-precise-token-spans.md` §5 S5 marked shipped and the arc marked **COMPLETE**.

## [0.35.0] — 2026-07-04

### Changed — precise `node_at` + region-coarse caveat retired for body nodes (LTXDOC02 S4)

With S3's tight body spans in place, `Document::node_at(byte)` now **formally** resolves to the
**true per-token leaf** — the narrowest node whose *precise* span contains the byte (ties → the
deepest node in pre-order). A byte inside `widgets` resolves to the `Text` run owning `widgets`, not
to the enclosing `Paragraph`/`Section`; a byte inside a `\section` title resolves to the title
inline, not the whole `Section` block.

- **Documentation only for `node_at`/`walk` — no logic change.** `node_at` already returned the
  narrowest-span walked node; S4 retires the hedging wording now that the spans are precise. Updated
  the `node_at` doc-comment (the "Resolution" paragraph), the `Provenance` struct docs, the
  `NodeRef::span` doc, the D6 module comment block, and the module-level span-policy note to state
  plainly that body resolution is precise. Preamble/metadata spans stay **honestly region-coarse**
  (the preamble is classified out of `\documentclass`/`\usepackage` *directives*, not walked, and
  `node_at` never resolves into it) — the `DocumentClass`/`Package` field docs keep that note.
- **New leaf-resolution tests.** `node_at_resolves_to_text_leaf_not_paragraph`,
  `node_at_in_section_title_resolves_to_heading_inline`, `node_at_in_textbf_resolves_to_inner_leaf` —
  each plants a byte inside a word/title/inner-run and asserts `node_at` returns exactly that `Text`
  leaf and its span slices back to precisely that word (`&src[prov.span] == "widgets"`).
- **New honest body byte-coverage test.** `body_bytes_resolve_to_containing_node` asserts that for a
  representative multi-node document (section, `\textbf`, inline math, an `itemize`), every
  non-whitespace body byte both resolves and resolves to a node whose precise span actually contains
  it. Scoped to a representative input (not the whole LTXDOC01 corpus) — the tightest-covering-leaf
  capstone remains S5.

Round-trip fixed point, totality/no-panic, no `unsafe`, and `MAX_DEPTH`-bounded recursion all
preserved. Spec `LTXDOC02-precise-token-spans.md` §5 S4 marked shipped; the LTXDOC01 §4/§5 D6
region-coarse caveats updated to say body resolution is now precise (preamble stays coarse).

## [0.34.0] — 2026-07-04

### Changed — precise Document fold (LTXDOC02 S3)

The `build_document` fold now reads each source `Node`'s **carried, precise byte `Span`** (threaded
by S1, unioned onto the recognition-pass nodes by S2) instead of stamping every lowered
`Block`/`Inline` with the coarse enclosing `region`. Body span *values* are now tight:

- **Leaf body nodes carry the node source range.** `lower_block` / `lower_inline` (and
  `lower_inlines` / `lower_accent_base`) stamp each `Block`/`Inline` with the source node's own
  `Node::span()`. `&src[inline.span]` slices back to exactly `\textbf{x}`, a `Text` run's word, an
  inline `$…$` island, a `\cite{…}`, etc. — no longer a shared region.
- **Composite bodies union their children's real spans.** A `Paragraph` = union of its inlines'
  spans (new `span_of_inlines` helper); a `Section` = heading node span ∪ owned children's spans
  (D3 `fold_sections`, unchanged mechanism, now over precise spans); a `List` / `Tabular` /
  `Environment` / `Figure` = the `\begin…\end` extent S2 computed; a captioned `table` float =
  inner-tabular extent ∪ float extent (so it owns the caption/label bytes); a `DocListItem` =
  union of its term-inline and body-block spans (a `ListItem` carries no span of its own); a
  `Caption` = union of its content inlines' spans. All unions fold min-start/max-end over real
  child spans — never substring search.
- **Region plumbing deleted.** `lower_block`, `lower_inline`, `lower_inlines`, `lower_list_item`,
  `lower_accent_base`, `extract_caption_label`, `scan_title_author_date`, and `extract_metadata`
  dropped their `region` parameter. `lower_blocks` keeps a `region` *fallback* only for the
  degenerate empty-paragraph / top-level body-region seed; nested cells/bodies route through a new
  `lower_blocks_precise` that seeds the fallback from the constituents' real spans.
- **Metadata content is precise too.** `\title` / `\author` / `\date` inline runs and the
  `abstract` body lower through the same span-precise fold, so their inline/block content carries
  tight ranges (though `Metadata` itself, an additive index, is not walked).

**Still region-coarse, honestly:** `Preamble.span` and the `span` on `DocumentClass` / `Package`
remain the preamble-region span — the preamble is classified out of directives, not walked as
per-node body content, so a preamble-region span is the right granularity there (these are not
visited by `Document::walk`). **Out of scope (S4):** the *formal* `node_at` "resolves to the true
leaf" guarantee, its dedicated test, and the removal of the remaining "region-coarse" hedging on
`node_at` / `Provenance` — S3 only makes the span *values* precise. Coverage capstone still holds
under tightening; `to_latex` output and the round-trip-modulo-spans fixed point are unchanged. No
`unsafe`; recursion `MAX_DEPTH`-bounded; no new `unwrap`/indexing on untrusted input. New S3 tests
assert `&src[node.span]` slices back to the exact source for a section title inline, a paragraph
`Text` run, a `\textbf` construct, a figure caption, a table cell, list items, and inline
math/cite; the D2-D5 containment tests tighten from "child ⊆ region" to "leaf == node source
range".

## [0.33.0] — 2026-07-04

### Added — spanned recognition passes (LTXDOC02 S2)

With S1 threading a real byte `Span` onto every L1 `Node`, S2 makes each node the OPT-IN
recognition passes *synthesise* carry the exact union of its constituents' real spans instead of a
coarse command/environment placeholder:

- **`recognize_structure`** (`structure.rs`): `Section` / `CrossRef` / `Preamble` / `Styled` fold the
  recognizing command's span with each recognized argument's real `Node::span()` (via a
  `fold_opt_arg` helper); the hoisted `\label` handling keeps real spans.
- **`recognize_accents`** (`text.rs`): the braced-argument `Accent` (`\c{c}`) unions the accent
  command span with its argument's span (the group / next-char cases were already exact in S1).
- **`recognize_tables`** (`tables.rs`): `Tabular` = `\begin{tabular}…\end{tabular}` unioned with every
  cell's content span; `List` = `\begin{env}…\end{env}` unioned with every item's label+body span
  (`union` / `seq_span` / `grid_span` / `list_span` helpers).

`&src[node.span()]` now slices back to the exact source extent for a `Section` (heading through its
owned body), an `Accent`, a `Tabular`, and an `itemize` `List`. All unions reuse the
`Span::new(a.start.min(b.start), a.end.max(b.end))` style over real child spans (never substring
search), fall back safely on degenerate/empty constituents, and leave `to_latex` output text and the
round-trip-modulo-spans fixed point untouched. No `unsafe`; recursion `MAX_DEPTH`-bounded. Precise
Document-fold spans (region-coarse today) land in S3.

## [0.32.0] — 2026-07-04

### Added — spanned L1 nodes (LTXDOC02 S1)

The first implementation rung of the **precise per-token byte spans** arc (spec
`LTXDOC02-precise-token-spans.md` §5). Every `Node` produced by `parse()` now carries its **exact
source byte `Span`**, threaded from the spans the lexer already records on every token — no
substring re-scanning. `&src[node.span().start .. node.span().end]` slices back to the node's own
source (`\textbf{x}`, `{…}` incl. braces, `$…$` incl. the delimiters, `\begin{env}…\end{env}`, a
`Text` run's exact characters, …).

- **`Node` restructured to `{ kind: NodeKind, span: Span }`.** The variants moved to a new public
  `NodeKind` enum; `Node` pairs a `NodeKind` with its `Span`. This keeps the span **orthogonal**
  (a `match` on shape reads `node.kind`; the span is one uniform field every node has) and is the
  single source of truth — the old `Unsupported { span: (usize, usize) }` bespoke tuple is dropped
  in favour of the uniform `Node.span`. New `NodeKind` export from the crate root.
- **Span accessor + terse constructors.** `Node::new(kind, span)`, `Node::span() -> Span`, and
  `Node::text/space/par/group/command(...)` builders. `Group`/`Command`/`Environment`/`Math` spans
  cover their delimiters; composite spans compose from the tracked start/end of the covered tokens.
- **Equality ignores the span.** `Node`'s `PartialEq`/`Eq` compare `kind` only, so the round-trip
  stays a **fixed point modulo spans** (`parse(&render(ast)) == ast`) even though re-emitting moves
  byte offsets — and every existing round-trip/equality test stays valid unchanged.
- **Recognition passes + Document fold threaded through.** `recognize_structure` /
  `recognize_accents` / `recognize_tables` fold onto the folded command's own span (extended over
  any folded siblings — union-of-constituents); `build_document`/`macros::expand` updated to match
  on `node.kind`. Deep precise-span work inside the recognition passes and the Document fold remains
  S2/S3; S1's core deliverable is spanned L1 `Node`s from `parse()`.
- **`to_latex` output text is unchanged** (spans move on re-emit; structure/text do not).
- **Tests:** exact source-slicing for a `Command`/`Group`/`Math`/`Text`/`Environment`; containment
  (`child.span ⊆ parent.span ⊆ 0..src.len()`); and a totality/no-panic test over malformed-but-
  parseable input (`Span::new` guards `end < start`).

## [0.31.0] — 2026-07-03

### Added — provenance API + byte-coverage capstone (LTXDOC01 D6, arc complete)

D6 is the **capstone** of the LTXDOC01 D1–D6 arc: with the `Document` model built (D2–D5), it adds
the two provenance queries the spec §4 promised and closes the arc with a real-paper byte-coverage
test.

- **New `NodeRef<'a>` enum** — a borrowed, `Copy` view over a walked node: `Block(&Block)` /
  `Inline(&Inline)`. Methods `span() -> Span` (reusing `block_span` / a new production
  `inline_span`) and `kind() -> &'static str` (a stable name per variant, e.g. `"Section"`,
  `"Paragraph"`, `"Math"`, `"CrossRef"`). Re-exported from the crate root.
- **New `Provenance<'a>` struct** — `{ node: NodeRef, span: Span }`, the result of a byte query.
  Re-exported from the crate root.
- **`Document::walk() -> impl Iterator<Item = NodeRef>`** — a **pre-order, depth-first** traversal
  of every body `Block` and every nested `Inline` (Section title/body, List item terms+bodies,
  Table/Figure captions+cells, Quote/Environment bodies, Paragraph inlines, composite-inline
  children, Accent bases). Materialized as `std::vec::IntoIter<NodeRef>`; bounded by the parser's
  `MAX_DEPTH`, so it is total and cannot overflow the stack.
- **`Document::node_at(byte) -> Option<Provenance>`** — returns the **innermost** (narrowest-span)
  walked node whose half-open span contains `byte`, ties broken toward the deeper (later pre-order)
  node. Panic-free: no `unwrap`/`expect`/unchecked index; span width is `end.saturating_sub(start)`.
- **Capstone tests** — a realistic titled `article` (abstract + `\section` + `tabular` in a `table`
  float + `itemize` + inline `$…$` + display `equation` + `figure` with `\caption`/`\label` +
  `\cite`): a `to_latex` round-trip fixed point (modulo spans), a non-panicking `walk()` covering
  all headline kinds, and a **byte-coverage** assertion that every non-whitespace byte inside the
  document body region is owned by ≥1 walked node.

### Honesty note — region-coarse spans

The D2–D5 spans are **region-granular**, not precise per-token ranges: many sibling blocks/inlines
share the enclosing region they were lowered from. The `walk`/`node_at` doc-comments and the
capstone test state this plainly — `node_at` resolves to the innermost node *at region
granularity* (not an exact byte→leaf), and the byte-coverage guarantee is scoped to the document
**body region** (preamble directives are indexed into `Preamble`/`Metadata`, not per-node walked).
Precise per-byte resolution needs the parser to thread exact token spans into lowered nodes — noted
as future work in the spec, not overclaimed here.

## [0.30.0] — 2026-07-03

### Added — floats, captions, code & display-math environments (LTXDOC01 D5)

D5 specializes the generic `Block::Environment` fold by environment name, so the `Document` model
finally distinguishes the semantic block kinds a real paper uses: floats with captions, verbatim
code, block quotations, and the named display-math environments.

- **New `Caption` struct** (`content: Vec<Inline>`, `span: Span`), derives
  `Debug, Clone, PartialEq, Eq`, re-exported from the crate root.
- **New `Block` variants**: `Figure { content, caption, label, span }`,
  `CodeBlock { verbatim, span }`, `Quote(Vec<Block>, Span)`. The `Table` variant gains
  `caption: Option<Caption>` and `label: Option<String>` fields (a bare `tabular` leaves both
  `None`; a `table` float attaches them to the inner tabular).
- **Environment classification** in `lower_environment`, all recursing the body through the same
  bounded `lower_blocks`:
  - `figure`/`figure*` → `Block::Figure`; the `\caption{…}` and a hoisted `\label{…}` are lifted
    out of the body, everything else (e.g. `\includegraphics`) stays in `content`.
  - `table`/`table*` → the inner `Block::Table` with the float's `\caption`/`\label` attached; a
    float with no inner tabular degrades to `Block::Figure` so nothing is lost.
  - `verbatim`/`verbatim*` (lexed raw as `VerbatimEnv`) → `Block::CodeBlock` with the raw inner
    text kept **unparsed**; `lstlisting` → `CodeBlock` (parsed body rendered back to source text).
  - `equation`/`equation*`/`align`/`align*`/`displaymath`/`gather`/`multline`/`eqnarray` →
    `Block::DisplayMath` with the inner LaTeX kept as a **source string** (delegated to the math
    frontend on demand — LTXDOC01 never parses math itself).
  - `quote`/`quotation` → `Block::Quote`.
  - any other environment → `Block::Environment` (recursed), unchanged from D2.
- **Caption/label extraction** (`extract_caption_label`) mirrors D3's `\label` hoist: after
  `lower_blocks`, a float's `\caption{X}` is an `Inline::Raw(Node::Command{"caption"})` and its
  `\label{k}` an `Inline::CrossRef{"label"}` inside the float's paragraphs; the first of each is
  lifted, its marker removed, and any now-empty paragraph dropped. Total & panic-free (no
  `unwrap`/`expect`/unchecked indexing).
- **`to_latex` round-trip.** Figures/table floats re-emit their `\begin{figure}`/`\begin{table}`
  wrapper with `\caption`/`\label`; `CodeBlock`s re-emit as `\begin{verbatim}…\end{verbatim}`
  (a fixed point — `lstlisting` normalizes through `verbatim`, text preserved); display-math envs
  re-emit as `$$…$$`. `parse(to_latex(d)).strip_spans() == d.strip_spans()` holds across a corpus
  containing a figure, a table float, a verbatim, an equation, and a quote.
- **Span policy unchanged** (coarse / region-granular; precise per-node coverage is D6). `Caption`
  and every new block carry a span; `strip_spans` and the span-containment tests cover them.

## [0.29.0] — 2026-07-03

### Added — document metadata extraction (LTXDOC01 D4)

D4 lifts the `\title` / `\author` / `\date` directives and the `abstract` environment into a small
typed `Metadata` record on `Document`, so a consumer can ask "what is the title / who are the
authors / what is the abstract?" without walking the block/inline tree. (Inline normalization —
`\textbf`/`\emph`/`\texttt` → `Strong`/`Emph`/`Code`, `$…$` → `Math`, `\ref`/`\cite` → `CrossRef`,
accents → `Accent` — was already delivered by the `lower_inline` pass in D2/D3; D4's remaining work
is the metadata index.)

- **New `Metadata` struct** (`title: Option<Vec<Inline>>`, `authors: Vec<Vec<Inline>>`,
  `date: Option<Vec<Inline>>`, `abstract_: Option<Vec<Block>>`), derives
  `Debug, Clone, PartialEq, Eq, Default`, re-exported from the crate root. A new
  `pub metadata: Metadata` field sits between `Document::preamble` and `Document::body`.
- **Additive, non-destructive projection.** Metadata is a typed *index over* the existing nodes —
  the `\title`/`\author`/`\date` commands still lower into `preamble.raw` (or a body block) and the
  `abstract` environment still becomes a `Block::Environment`. **Nothing is moved or removed**, so
  `to_latex` round-trips byte-for-byte unchanged and re-parsing repopulates the same `Metadata` (a
  fixed point — pinned by a test). `\maketitle` is a no-op for metadata (nothing to capture) and is
  carried through the body as before.
- **Both streams scanned.** `\title`/`\author`/`\date` are honoured in the preamble **or** the body
  (LaTeX allows either); the preamble is scanned first, so a preamble `\title` wins over a stray
  body one. First `\title` and first `\date` win; every `\author` contributes, and each `\and`
  inside an `\author` splits it into multiple author entries.
- **Total & panic-free.** Extraction is a single linear allocation-only pass per stream, with no
  unchecked indexing and no new recursion (the `abstract` body lowers through the same bounded
  `lower_blocks`). Absent directives leave the fields `None`/empty — never fabricated.
- **Spec divergence noted:** the spec's D4 bullet describes metadata extraction but not the
  *additive-projection* decision (keeping the nodes in place for round-trip safety). The spec's D4
  bullet is updated to record this. Spans stay coarse (region-granular); precise per-node byte
  coverage remains D6.

## [0.28.0] — 2026-07-03

### Added — the sectioning fold (LTXDOC01 D3)

D3 folds D2's **flat** body block stream into the **nested sectioning forest**. Where D2 emitted
every heading as a zero-body `Block::Section`, D3's `fold_sections` pass makes each heading OWN the
run of blocks that follow it, up to (but not including) the next heading of the same-or-higher level;
deeper headings nest inside. This runs inside `build_document` for the top-level body **and every
nested block-list** (environment/quote/figure bodies, list items, table cells), because they all
route through the one `lower_blocks` function — so nesting works everywhere.

- **Level ranking.** A new `fn rank(level: SectionLevel) -> u8` gives the hierarchy order
  `Part=0 < Chapter=1 < Section=2 < Subsection=3 < Subsubsection=4 < Paragraph=5 < Subparagraph=6`
  (mirroring `SectionLevel`'s declaration order). A section owns following blocks until it meets a
  heading whose rank is **≤** its own (that heading starts a sibling/ancestor section); strictly
  greater rank nests inside.
- **`Block::Section` gains a `label: Option<String>` field** (per spec §3). `fold_sections` **hoists
  a `\label{key}`** that immediately follows a heading onto this field: the `\label` lowers (via
  `recognize_structure`) to a leading `label` `Inline::CrossRef` on the section's first owned
  paragraph, which the fold peels off (dropping a now-empty paragraph, preserving any following
  text). Only the unambiguous immediately-following case is hoisted; any other `\label` position is
  **left in place in `body`** (never dropped), keeping the fold total. A hoisted label is re-emitted
  by `to_latex` right after the heading so re-parsing + re-folding is a fixed point.
- **Span union.** A folded section's `span` is the union of its heading's (coarse) region span and
  its owned children's spans, so it still satisfies child ⊆ parent ⊆ `Document` (extended
  span-integrity test covers the nested case).
- **Totality preserved.** `fold_sections` is total and panic-free — no `unwrap`/`expect`, no
  unchecked indexing on the block list; recursion folds a strict sub-slice each call, bounded by the
  parser's `MAX_DEPTH`-capped tree. No `unsafe`.
- **Property test.** A `flatten(&[Block])` linearizes the folded forest (heading with emptied body,
  then depth-first its owned blocks); `flatten(fold(flat)) == flat` reproduces D2's pre-fold order.
  Covered case: `\section{A} p1 \subsection{B} p2 \section{C} p3` → A owns `{p1, B{p2}}`, C owns
  `{p3}`; A and C are top-level siblings, B nests in A.

`Cargo.toml` bumped 0.27.0 → 0.28.0. No public API break beyond the added `Section.label` field;
all D2 constructors/matchers/tests updated accordingly.

## [0.27.0] — 2026-07-03

### Added — the hierarchical `Document` layer, preamble/body split (LTXDOC01 D2)

A new `src/document.rs` module lifts LTX01's *flat* `Vec<Node>` into a reusable, hierarchical
`Document` model — the layer above the presentation-shaped node stream. This is a **pure, total
fold** over already-parsed nodes; it touches no lexer/parser/grammar.

- **New public types** (each carries a `token::Span`):
  - `Document { preamble, body: Vec<Block>, span }` — the whole document.
  - `Preamble { document_class, packages, raw: Vec<Node>, span }` — classified directives plus the
    untouched remainder.
  - `DocumentClass { class, options, span }`, `Package { name, options, command, span }`.
  - `Block` — `Section { level, numbered, title, short_title, body, span }` (zero-body in D2),
    `Paragraph`, `List`, `Table`, `DisplayMath`, `Environment`, `Raw(Node, …)`.
  - `Inline` — `Text`, `Space`, `Strong`, `Emph`, `Code`, `Styled`, `Math`, `CrossRef`, `Accent`,
    `Raw(Node, …)`.
  - `DocListItem { term, body, span }` — the D2 analogue of `ListItem`.
- **New public functions:**
  - `build_document(nodes: Vec<Node>, src: &str) -> Document` — the fold in isolation. **Total**:
    never errors, never panics. Splits the node stream at the `document` environment (whole stream
    is preamble if absent), classifies `\documentclass`/`\usepackage`/`\RequirePackage` (matched on
    the `Node::Preamble` variant `recognize_structure` produces), and lowers the body's flat nodes
    into a **flat** `Vec<Block>` (no sectioning nesting yet — every heading is a zero-body
    `Block::Section`; D3 fills the bodies).
  - `parse_document(src: &str) -> Result<Document, ParseError>` =
    `Ok(build_document(recognize_tables(recognize_accents(recognize_structure(parse(src)?))), src))`.
    `recognize_structure` runs first so the fold matches on semantic variants directly; the only
    fallible stage is `parse`.
  - `Document::to_latex()` (+ block/inline renderers) round-trips: `parse_document(&d.to_latex())`
    equals `d` **modulo spans**, compared via a span-stripped `Document::strip_spans()` projection.
- **Span policy (D2 — coarse but honestly nested).** `Document.span = 0..src.len()`; `Preamble.span
  = 0..find("\\begin{document}")` (or `src.len()`); the body region span = end of
  `\begin{document}`..start of `\end{document}` (found by substring search over the source). Every
  block/inline span **defaults to its enclosing region span**, so every child span ⊆ its parent ⊆
  the `Document` span (asserted by a span-integrity test). **Precise per-node byte coverage is
  deferred to D6** once the parser threads token spans through `Node` — a repo-std-#9 divergence
  from the spec's per-node-span ideal; the type carries the field now so later rungs tighten values
  without an API break.
- Re-exported from `lib.rs`: `build_document`, `parse_document`, `Document`, `Preamble`,
  `DocumentClass`, `Package`, `Block`, `Inline`, `DocListItem`. Pipeline doc-comment entry #10 added.
- Tests: preamble/body split on a real document; no-`document`-env fragment (all preamble, empty
  body); `\documentclass`+`\usepackage` classification; a body exercising heading + paragraph +
  itemize + tabular + inline `$…$` + display `\[…\]` + `\ref`; span-integrity (child ⊆ parent ⊆
  document); round-trip modulo spans; totality on junk input; environment recursion.

*No `unsafe`; `cargo clippy -p latex -- -D warnings` clean. `Metadata` (spec §3) is intentionally
deferred to D4 — D2 ships only the skeleton + preamble/body split it specifies.*

## [0.26.0] — 2026-07-03

### Added — document-mode tables & lists via a new `recognize_tables` pass (LTXDOC01 D1)

Closes the L3b gap LTX01 deferred: document-mode `tabular`/`tabular*` grids and the
`itemize`/`enumerate`/`description` list environments now fold into structured AST nodes. This is
a **recognition pass over already-parsed generic environments** — not a parser/lexer change: L1
already parses `\begin{tabular}{lcr}a & b \\ c & d\end{tabular}` as a generic `Node::Environment`
(with `&` → `Text("&")`, `\\` → `Command("\\")`, `\item[t] x` → `Command("item", opt:[t])` + siblings).

- **New `Node` variants** (both span-less, matching the other structural variants):
  - `Node::Tabular { col_spec: Option<String>, rows: Vec<Vec<Vec<Node>>> }` — `rows[r][c]` is cell
    `c` of row `r`; `col_spec` is the column spec captured verbatim (`"lcr"`, `"l|c|r"`), `None` if
    absent. For `tabular*` the `{width}` argument is dropped and the trailing `{colspec}` kept.
  - `Node::List { kind: ListKind, items: Vec<ListItem> }` with new `pub enum ListKind { Itemize,
    Enumerate, Description }` and `pub struct ListItem { label: Option<Vec<Node>>, body: Vec<Node> }`
    (`label` = the `\item[term]` optional term).
- **New `recognize_tables(nodes) -> Vec<Node>` pass** (`src/tables.rs`), mirroring
  `recognize_structure`: a total, infallible fold that recurses into every child node-list and
  splits `tabular` bodies on `&`/`\\` and list bodies on `\item`. Re-exported from `lib.rs` along
  with `ListKind`/`ListItem`.
- **`to_latex` round-trip**: `recognize_tables(parse(&node.to_latex())) == [node]` for tables and
  lists (asserted over a corpus). A recognized `tabular*` round-trips as a plain `tabular` (its
  width was not a column spec — dropping it is faithful to the grid).
- **Totality (spec reconciliation)**: the pass never errors and never panics — ragged rows
  (differing cell counts) are preserved exactly, and a list with stray content before its first
  `\item` is left as a generic `Node::Environment`. The spec's "spanned errors on malformed grids"
  guarantee lives in the **L1 parser** (unbalanced braces / `\begin`/`\end` mismatch, `MAX_DEPTH`
  depth guard), upstream of this total recognition pass; recursion here is bounded by the L1 tree
  depth (no new unbounded recursion, no raw brace counting — `col_spec` comes from parsed argument
  nodes). Per-node byte spans remain the Document layer's job (D2+).
- No `unsafe`; `cargo clippy -p latex -- -D warnings` clean; downstream `adj-lang`/`adj-lang-cli`
  unaffected (the new variants don't reach the math-frontend path).

## [0.25.0] — 2026-07-01

### Changed — comma/semicolon fence lists now keep their delimiters (`Fenced`-of-`Sequence`)

The fence-delimiters arc's remaining slice: a **comma- or semicolon-separated** fence body now
carries its delimiters too. Previously a single-body fence lowered to `MathExpr::Fenced { open,
body, close }` (0.24.0) but a *list* fence (`(a, b)`, `[a, b]`, `\left(a, b\right)`, semicolon
rows) lowered to a bare `MathExpr::Sequence`, **dropping** the surrounding brackets — so `(a, b)`
and `[a, b]` were indistinguishable. Now the frontend adapter wraps the list in a `Fenced` carrying
the open/close strings: `(a, b)` → `Fenced { open: "(", body: Sequence([a, b]), close: ")" }`,
distinct from `[a, b]` → `Fenced { open: "[", … }`. **Both** the delimiters and the list structure
are preserved.

- The change is a one-line relaxation in the neutral-lowering worklist (`frontend.rs`): the
  `MathNode::Fenced` arm now always pushes a `Build::Fenced` wrapper, instead of skipping it when
  the body is a `Sequence`. The inner `Sequence` still builds via its own arm; the `Fenced` build
  wraps it. No new node, no capability change (`fenced_delimiters` + `sequences` already declared).
- Matrices (`pmatrix`/`bmatrix`/…) are unaffected — their delimiter style stays presentation on
  `Matrix` as before; only `\left…\right` / `( ) [ ]` *list* fences are wrapped.
- Tests `comma_fence_lowers_to_sequence` / `semicolon_fence_lowers_to_nested_sequence` become
  `…_lowers_to_fenced_sequence` / `…_lowers_to_fenced_nested_sequence`, asserting the delimiters are
  now carried around the (unchanged) inner sequence. All downstream consumers (`math-frontend`,
  `mathml`, `asciimath`, `unicode-math`, `adj-lang`, `adj-lang-cli`) build and pass unchanged — the
  `adj-lang` adapter already unwraps `Fenced` to its body, so a `Fenced`-of-`Sequence` lowers
  exactly as the bare `Sequence` did (both non-arithmetic).

## [0.24.0] — 2026-06-30

### Changed — fence delimiters preserved as data (frontend adapter)

- The `MathFrontend` adapter now lowers a single-body fence to the new neutral
  **`MathExpr::Fenced { open, body, close }`** (math-frontend 0.7.0) instead of the delimiter-less
  `MathExpr::Group`, carrying the surface delimiters as data: `(x+1)` → `Fenced("(", …, ")")`,
  `[x]` → `Fenced("[", …, "]")`, `\left|x\right|` → `Fenced("|", …, "|")`. An absolute value / norm
  is no longer indistinguishable from a parenthesised group. The latex frontend declares the new
  `fenced_delimiters` capability (via `Capabilities::all()`).
- A **comma-list** fence still lowers to `MathExpr::Sequence` (delimiters dropped, matching MathML
  `<mfenced>` / AsciiMath) — carrying delimiters on sequences is a later slice of this arc.
- The internal `MathNode` and `to_latex` round-trip are unchanged; this only affects the neutral
  lowering. Golden tests updated to assert the preserved delimiters.

## [0.23.0] — 2026-06-30

### Added — down-barb & under harpoon accents (completing the harpoon accent family)

- **Down-barb over-harpoons** (`harpoon` package) — `\overrightharpoondown`, `\overleftharpoondown`,
  the down-barb siblings of the existing up-barb `\overrightharpoonup`/`\overleftharpoonup`. Added
  to `over_arrow_base`, lowering onto `Overset` over the standard amsmath ⇁/↽ relation symbols.
- **Under-harpoons** (`harpoon` package) — `\underrightharpoonup`, `\underleftharpoonup`,
  `\underrightharpoondown`, `\underleftharpoondown`, the under-body mirror of the over-harpoons.
  Added to `under_arrow_base`, lowering onto `Underset` over the plain harpoon symbol.

Both extend the two existing base-name tables (no new machinery, no new AST node, no frontend
change); `to_latex` round-trips through `\overset`/`\underset` over the plain harpoon symbol (an
unknown control word re-parses to a `Sym`). Missing `{body}` is a clean spanned error. With the
prior `\overrightharpoonup`/`\overleftharpoonup` and the `\underrightarrow` family, the crate now
covers all four over/under × up/down harpoon accents.

## [0.22.0] — 2026-06-30

### Added — stretchy UNDER-arrow accents (the mirror of the over-arrow family)

- **Under-arrow accents** (amsmath) — `\underrightarrow`, `\underleftarrow`, `\underleftrightarrow`.
  These are the exact mirror of the existing `\overrightarrow`-family: a stretchy arrow drawn
  **under** the argument rather than over it. Each lowers onto the existing `Underset` node over the
  plain arrow symbol — the same annotation-under-a-body shape as `\underbrace` and an xarrow's
  `[below]` label — so there is no new AST node and no frontend change:
  `\underrightarrow{AB}` → `Underset { under: →, base: AB }` ≡ `\underset{\rightarrow}{AB}`, and
  `to_latex` round-trips through `\underset`. Implemented via a new `under_arrow_base` table mirroring
  `over_arrow_base`; missing `{body}` is a clean spanned error, never a panic.

## [0.21.0] — 2026-06-30

### Added — more stretchy accents & extensible arrows (all onto the existing Overset/Underset node)

- **Extensible harpoons** (mathtools) — `\xrightharpoonup`, `\xrightharpoondown`, `\xleftharpoonup`,
  `\xleftharpoondown`, `\xrightleftharpoons`, `\xleftrightharpoons`, plus `\xLeftrightarrow`. These
  are arrow-like relations with a stretchable label, so they lower **exactly like the xarrows**:
  `\xrightleftharpoons{k}` → `Overset { over: k, base: ⇌ }`, and an optional `[below]` group stacks
  under (`Underset`). Added to the `xarrow_base` table — no new machinery.
- **Over/under paren & group accents** (amsmath/mathtools) — `\overparen`/`\underparen`,
  `\overgroup`/`\undergroup`, alongside the existing `\overbrace`/`\underbrace`. Each draws a
  stretchy parenthesis / group bracket over (under) the body and lowers onto `Overset`/`Underset`
  over the Unicode top/bottom glyph (⏜ U+23DC / ⏝ U+23DD / ⏠ U+23E0 / ⏡ U+23E1), with an optional
  trailing `^{label}` (over) / `_{label}` (under) stacked on the glyph — the same mechanism as
  `\overbrace`. The two hardcoded brace branches were generalised into `over_accent_glyph` /
  `under_accent_glyph` tables (behaviour-identical for the existing braces).
- **No new AST node, no frontend change** — every command reuses the neutral `Overset`/`Underset`
  nodes, so the `math-frontend` lowering and `to_latex` round-trip need no change (the surface
  normalises to `\overset`/`\underset`, which re-parse to the identical tree). 6 new tests (harpoon
  base mapping, harpoon `[below]` underset, the four paren/group accents, label stacking + round-trip
  corpus, missing-body error). `adj-lang` (the only consumer) builds. `latex` 0.20.0 → 0.21.0.

## [0.20.0] — 2026-06-30

### Added — semicolon-separated fences lower to rows (matches MathML `<mfenced>` PR-5)

- A fence whose body contains a top-level semicolon — `(a, b; c, d)`, `\left(a, b; c, d\right)` — is
  now read as **rows**: semicolons are the row separator and commas the within-row (column)
  separator, the classic fenced-matrix notation. `(a, b; c, d)` parses to
  `Sequence([Sequence([a, b]), Sequence([c, d])])` and lowers to the same nested
  `MathExpr::Sequence`. This mirrors the `mathml` crate's PR-5 exactly, extending the write-once
  Sequence node to a second structural frontend's row shape.
- Degenerate shapes are predictable, identical to MathML: a **semicolon-only** fence `(a; b; c)` — a
  column vector with no second column in any row — collapses to the same flat `Sequence([a, b, c])`
  as a comma list; a **ragged** fence `(a; b, c)` stays faithful as
  `Sequence([a, Sequence([b, c])])`. Each cell is still folded to a full relation, so `(x + 1, 2; y)`
  nests the folded `x + 1`. A comma-only fence is unchanged (flat `Sequence`), and a comma/semicolon-
  free fence is still a plain grouped expression.
- **Round-trips** through `to_latex`: a rows `Sequence` (detectable because a comma-list item is
  always a relation, never a *bare* `Sequence`, so a bare-`Sequence` child can only be a row)
  semicolon-joins its rows and comma-joins each row's columns, so `parse_math(n.to_latex()) == n`
  holds for `(a, b; c, d)` and friends. `read_fence_body` records the separator after each relation
  in a bounded loop (never recursion), so a wide grid cannot overflow the stack; a
  leading/trailing/doubled `;` or `,` is a clean spanned error, never a dropped row.
- No new AST node, no shared-crate change (reuses `MathExpr::Sequence`; the neutral node already
  nests, and the frontend lowering already recurses). 10 new tests (rows, semicolon-only flat,
  ragged, folded cells, trailing-`;` error, 4 round-trip corpus entries, frontend nested-lowering).
  `latex` 0.19.0 → 0.20.0.

## [0.19.0] — 2026-06-30

### Added — comma-separated fences lower to the neutral `Sequence` node (third frontend)

- A **comma-separated fence** — `(a, b, c)`, `[x, y, z]`, `\left(p, q\right)` — is now recognised
  as an ordered **list** rather than a single grouped expression. The parser reads the fence body
  with a new `read_fence_body` helper: it parses the first relation, and if the next token is a
  comma it keeps collecting comma-separated items into a new `MathNode::Sequence(items)` AST node.
  A **comma-free** fence — `(a + b)` — is unchanged: it stays a plain grouped expression. Only a
  top-level comma inside the fence triggers a `Sequence`; commas nested inside an inner group are
  the inner group's business.
- **Round-trips** through `to_latex`: a `Sequence` renders its items comma-joined, wrapped by the
  fence that produced it, so `parse_math(n.to_latex()) == n` still holds. `Sequence` participates in
  the iterative-`Drop` `take_children` trampoline, so a pathologically deep nest (tested to 200 000
  levels) drops without a stack overflow — same guarantee as every other node.
- **Neutral-frontend lowering** (`frontend.rs`): a `Sequence` fence lowers to `MathExpr::Sequence`
  (the delimiters are dropped, exactly as for MathML `<mfenced>` comma rows and AsciiMath `(a,b,c)`).
  This makes `latex` the **third** emitter of the neutral `MathExpr::Sequence` node introduced in
  `math-frontend` 0.6.0 — completing write-once-use-many across LaTeX + AsciiMath + MathML: one
  neutral list node, three surface syntaxes, no per-consumer special-casing. A comma-free fence
  still lowers to `MathExpr::Group` as before.
- 9 new tests (parse-to-`Sequence`, comma-free-stays-plain, full-expression items, `\left…\right`
  and bracket fences, 200 000-deep drop, 4 round-trip corpus entries, plus a `frontend.rs` lowering
  test asserting `(a, b, c)` → `MathExpr::Sequence` and `(a + b)` → `MathExpr::Group`).
- `latex` 0.18.0 → 0.19.0.

## [0.18.0] — 2026-06-30

### Added — stretchy over-arrow accents (`\overrightarrow` & friends)

- The parser now accepts the stretchy over-arrow accents `\overrightarrow`, `\overleftarrow`,
  `\overleftrightarrow`, `\overrightharpoonup`, `\overleftharpoonup` — an arrow drawn **over** a
  (possibly multi-token) body, e.g. `\overrightarrow{AB}`. Distinct from `\vec` (a fixed
  single-glyph accent over one symbol). As with `\overbrace` and the xarrows, there is **no new AST
  node**: an over-arrow is an annotation stacked on the body, so it lowers onto the existing
  `Overset` node over the plain arrow symbol — `\overrightarrow{AB}` → `Overset { over: →, base: AB }`,
  identical to `\overset{\rightarrow}{AB}`.
- Reuses the existing `Overset` `to_latex` and neutral-frontend lowering, so **no frontend change**;
  round-trips through `to_latex` (surface normalises to `\overset`). 5 new tests (parse, per-arrow
  base, in-context, round-trip, missing-body error); the `{body}` is mandatory and its absence is a
  clean spanned error, never a panic.
- `latex` 0.17.0 → 0.18.0.

## [0.17.0] — 2026-06-30

### Added — horizontal braces (`\overbrace` / `\underbrace`)

- The parser now accepts `\overbrace{body}` and `\underbrace{body}` — a horizontal brace drawn over
  or under the body, optionally labelled by a trailing `^{label}` (overbrace) / `_{label}`
  (underbrace) that sits over/under the brace. As with the extensible arrows, there is **no new AST
  node**: a brace is exactly an annotation stacked on the body, so it lowers onto the existing
  `Overset`/`Underset` nodes over the Unicode brace glyph (`⏞` U+23DE / `⏟` U+23DF):
  - `\overbrace{a+b}` → `Overset { over: ⏞, base: a+b }`
  - `\overbrace{x+y}^{n}` → `Overset { over: n, base: Overset { over: ⏞, base: x+y } }` (label over
    brace over body); `\underbrace{a}_{k}` is the under-side twin.
- The optional label is consumed inside the brace parse (peeking for `^`/`_`) so it stacks centered
  on the brace, mirroring how the xarrows consume their `[below]`/`{above}` labels — rather than
  being attached as an ordinary raised/lowered script by the postfix mechanism.
- Reuses the existing `Overset`/`Underset` `to_latex` and neutral-frontend lowering, so **no
  frontend change**: `\overbrace`/`\underbrace` reach the neutral `MathExpr::Overset`/`Underset` for
  free. Round-trips through `to_latex` (the surface normalises to `\overset`/`\underset`, which
  re-parse to the identical tree). 7 new tests (parse, labelled, in-context, round-trip, missing-body
  error); the `{body}` is mandatory and its absence is a clean spanned error, never a panic.
- `latex` 0.16.0 → 0.17.0.

## [0.16.0] — 2026-06-30

### Added — extensible / labelled arrows (`\xrightarrow` & friends)

- The parser now accepts the amsmath **extensible arrows** that carry a stretching label:
  `\xrightarrow`, `\xleftarrow`, `\xleftrightarrow`, `\xRightarrow`, `\xLeftarrow`, `\xmapsto`,
  `\xhookrightarrow`, `\xhookleftarrow`. Each takes a **mandatory `{above}` group** (the label set
  over the arrow) and an **optional `[below]` group** (a second label set under it) — the same
  argument shape as in real LaTeX, e.g. `\xrightarrow[\text{below}]{f}`.
- **No new AST node.** A labelled arrow is *exactly* an annotation stacked on the plain arrow
  symbol, so it lowers onto the existing `Overset`/`Underset` nodes: `\xrightarrow{f}` →
  `Overset { over: f, base: \rightarrow }`, and `\xrightarrow[g]{f}` →
  `Underset { under: g, base: Overset { over: f, base: \rightarrow } }`. Because nothing new is
  introduced, the neutral **`MathFrontend` lowering needs zero change** — `\xrightarrow{f}` lowers
  to the identical `MathExpr::Overset` as the explicit `\overset{f}{\rightarrow}`.
- The surface normalises through `to_latex()` to the equivalent `\overset`/`\underset` form, which
  re-parses to the identical tree (round-trip is a fixed point). A missing mandatory `{above}`
  label, or an unterminated optional `[below]` label, is a **spanned `ParseError`, never a panic**.
- 8 new parser tests (`math.rs`) + 1 neutral-lowering test (`frontend.rs`).

## [0.15.0] — 2026-06-30

### Added — the `array` / `subarray` grids (mandatory column-spec)

- The parser now accepts the general **`\begin{array}{cols} … \end{array}`** grid and its
  big-operator-limit cousin **`\begin{subarray}{c} … \end{subarray}`** — the math environments that
  carry a **mandatory column-alignment argument**. This was the documented gap in the matrix family
  (`pmatrix`/`bmatrix`/`cases`/`aligned`/…, all of which take *no* argument): the comment on
  `is_math_env` explicitly parked `array` for "a later layer" because it "need[s] an extra field on
  the node." That field is now here.
- New optional field **`MathNode::Matrix { col_spec: Option<String>, … }`**: the column spec is
  captured **verbatim** (`"cc"`, `"l|cr"`, `"p{3cm}"`, `*{3}{c}`, `>{…}`/`<{…}`/`@{…}` inserts), so
  the node **round-trips** exactly — `to_latex` re-emits `\begin{array}{<spec>}`. It is `None` for
  every environment that takes no argument (a `pmatrix` is unchanged).
- The neutral **lowering drops `col_spec`** (alignment is presentation, PFE01 §2.2): an `array` and
  the equivalent `pmatrix` lower to the **same** `MathExpr::Matrix`. So consumers gain `array` for
  free — no consumer change, no new neutral node, no capability change.
- `read_col_spec` reads the `{…}` argument with a **flat `depth` counter, not recursion**, so it is
  brace-nesting-aware (captures `p{3cm}` whole) yet an adversarial `{{{{…` cannot overflow the
  stack (bounded by input length, like the rest of the tokenizer output). A missing or unterminated
  col-spec is a clean **spanned error**, never a panic.
- Tests (9 new): `array_captures_column_spec_and_cells`, `…_keeps_rules_and_alignment_letters`,
  `…_handles_braced_groups`, `subarray_takes_a_column_spec_too`, `array_round_trips_through_to_latex`,
  `matrix_family_still_has_no_column_spec`, `array_without_column_spec_is_a_spanned_error`,
  `deep_column_spec_braces_do_not_overflow`, and frontend `array_lowers_dropping_column_spec`
  (array ≡ pmatrix after lowering). 150 unit + doc tests pass; clippy `-D warnings` clean;
  downstream `adj-lang` (the only consumer) 86 green.

## [0.14.0] — 2026-06-30

### Added — `\overset` / `\underset` / `\stackrel` → neutral `Overset` / `Underset`

- The parser recognizes **`\overset{over}{base}`**, **`\stackrel{over}{base}`** (amsmath's name
  for the over-set form), and **`\underset{under}{base}`** — two mandatory args (annotation then
  base, the `\frac`/`\binom` shape) — as new `MathNode::Overset` / `MathNode::Underset`, and the L6
  lower emits the neutral **`MathExpr::Overset` / `MathExpr::Underset`** (math-frontend 0.5.0). A
  centered annotation **over/under** the base, **distinct from `Pow`/`Subscript`** (`b^a` / `b_a`).
  This is the LaTeX-side twin of the asciimath over/under-set emitter.
- `to_latex` round-trips both nodes (`parse_math(node.to_latex()) == node`); the iterative
  `take_children` (deep-Drop guard) handles their two children; both stay atoms in the precedence
  table. The lower runs in the existing work-stack trampoline (no new recursion). `capabilities()`
  already advertises `oversets` via `all()`, so emission is conforming with no capability change.
- Tests: parser `overset_underset_parse_and_round_trip`, lower `overset_underset_lower_to_neutral_nodes`
  (overset/underset/stackrel, distinct-from-Pow). 141 unit + doc tests pass; clippy `-D warnings` clean.

## [0.13.0] — 2026-06-29

### Changed — accents lower to the neutral `MathExpr::Accent` (no longer faked as a function call)

The L6 adapter previously lowered a diacritical accent (`\hat{x}`, `\bar{y}`, `\vec{v}`,
`\tilde{a}`, `\dot{x}`, …) to `MathExpr::Call { func: Func::Other(kind), arg }` — a *function
application*, which is the wrong meaning: `\hat{x}` is a mark *over* `x`, not `hat(x)`. Now that
`math-frontend` 0.4.0 provides a neutral **`MathExpr::Accent { accent, body }`** node, the adapter
emits it faithfully.

- `lower()`'s `MathNode::Accent { kind, body }` arm now pushes a new `Build::Accent(kind)` work step
  (instead of `Build::Call(Func::Other(kind))`); its assembler pops the lowered body and produces
  `MathExpr::Accent { accent: kind, body }`. Still inside the iterative trampoline — O(1) call-frame
  depth, stack-safe on deep accent nests.
- `capabilities()` stays `Capabilities::all()` — now **honestly** includes `accents`, which the
  shared conformance harness (math-frontend 0.4.0) enforces (emitting an Accent requires declaring it).
- Test `accent_is_a_named_unary` becomes `accent_lowers_to_neutral_accent_node` (asserts `\hat{x}` →
  `Accent{accent:"hat", x}` and `\vec{v}` → `Accent{accent:"vec", v}`). Tests pass; downstream
  `adj-lang` green (its adapter's catch-all routes an Accent to "unsupported ADJ arithmetic", since a
  diacritic is not computable — behavior unchanged). clippy `-D warnings` clean.

## [0.12.0] — 2026-06-29

### Fixed — deep `MathNode` trees now drop without overflowing the stack

`MathNode` is a recursive `Box`-owning enum, so the compiler-generated destructor recurses
once per level. The parser builds **left-nested** chains via loops with no per-term depth
charge (`parse_add`/`parse_mul`/`parse_relation`), so input like `1+1+1+…` (or a long
juxtaposition `aaa…`) yields an O(n)-deep tree even though `MAX_DEPTH` bounds *nesting*.
Dropping a deep-enough such tree overflowed the stack — an **uncatchable abort**, even
though `parse_math` itself survives (it builds iteratively). This was the pre-existing
latent hazard flagged in the L6 security review.

- **`impl Drop for MathNode`** now dismantles the tree with an explicit **heap worklist**
  instead of the call stack: each node's boxed children are moved onto a `Vec` (replaced
  in place by a cheap `Sym("")` leaf), popped, and repeated, so the generated destructor
  recurses at most one trivial level. O(1) stack depth regardless of tree depth. This
  mirrors `math_frontend::MathExpr`'s `Drop` (added in `math-frontend` 0.3.0).
- **`take_children` helper** does the per-variant child extraction (leaves contribute
  nothing; `Matrix` rows are drained via `mem::take`).
- Because `MathNode` now implements `Drop`, fields can no longer be moved out of an owned
  value in a by-value `match` (E0509). The `frontend.rs` `lower()` trampoline and the
  by-value test matches were rewritten to `match &node`/`match &n` and extract children via
  `mem::replace`/`Option::take`/`mem::take`. No behavior change — purely how ownership is
  threaded.
- New regression tests: `deep_left_nested_tree_drops_without_overflow` (200k-deep `Bin`
  spine), `deep_parsed_chain_drops_without_overflow` (50k-term `+` chain through the real
  parser), `deep_unary_chain_drops_without_overflow` (200k-deep `Unary` spine). 139 tests
  pass; both the default build and `--no-default-features` (zero-dep L0–L5) stay green.

## [0.11.0] — 2026-06-28

### Changed — L6 closes its two honest neutral-AST gaps (± / ∓ and binomials)

The L6 capstone (0.10.0) had to return a spanned `FrontendError` for `\pm`, `\mp`, and
`\binom`, because the `math-frontend` neutral AST could not represent them. `math-frontend`
0.2.0 added `BinOp::PlusMinus`/`MinusPlus` and `MathExpr::Binom`; this release wires the
`latex` adapter to **emit** them, so every LaTeX math construct the L2/L3a grammar parses now
lowers to a faithful neutral node — no faking, no honest-error islands.

- **`\pm` → `BinOp::PlusMinus`, `\mp` → `BinOp::MinusPlus`** (the ± / ∓ pair operators).
  `lower_binop` is now total (infallible) — the two error arms are gone.
- **`\binom{n}{k}` → `MathExpr::Binom(n, k)`** (binomial coefficient, args in source order;
  distinct from `Frac` — no division bar). Lowered via a new `Build::Binom` work-stack step,
  so it stays inside the iterative (stack-safe) trampoline like every other node.
- `capabilities()` stays `Capabilities::all()` — now **honest**, since the adapter genuinely
  emits ± / ∓ and binomials (the shared conformance harness polices this).
- Tests: the former `neutral_gaps_error_honestly` becomes `plusminus_minusplus_and_binom_lower`
  (asserts the three now lower to the right shapes); the conformance corpus keeps `a \pm b`
  and `\binom{n}{k}` (now exercising emission, not error). 136 tests pass; both the default
  build and `--no-default-features` (zero-dep L0–L5) stay green; clippy `-D warnings` clean.
- Crate 0.10.0 → 0.11.0. **LTX01 L6 now has no honest gaps.**

## [0.10.0] — 2026-06-27

### Added — LTX01 L6: `math-frontend` adapter (the ladder's capstone)

- **`LatexMath`** implements `math_frontend::MathFrontend`: `parse(src)` runs the L2/L3a math
  grammar (`parse_math`) and **lowers** the LaTeX-shaped `MathNode` into the notation-agnostic
  `math_frontend::MathExpr`. LaTeX is now the first **pluggable frontend** of the PFE01 framework
  — a consumer (rule engine, CAS, renderer) lowers one neutral tree and gets LaTeX for free.
- **Registration:** `latex::registry()` returns a `FrontendRegistry` with LaTeX installed, and
  `latex::register_latex(&mut reg)` installs it into an existing one. (`math-frontend`'s own
  `with_builtins()` stays empty by design — it cannot depend on this crate without a cycle; the
  wiring lives here.)
- **Neutral lowering** drops presentation, keeps meaning: `\times`/`\cdot`/juxtaposition →
  `Mul`; `\frac`/`\dfrac`/`\tfrac` → `Frac`; every fence style → `Group`; every matrix
  delimiter → `Matrix`; `a^n` → `Pow`, `a_i` → `Subscript`, `a_i^n` → `Pow(Subscript(..),..)`;
  accents (`\hat{x}`) → `Call{Other(kind), arg}`. Numbers stay **exact** (`MathExpr::Number`,
  never `f64`). Declares `Capabilities::all()`; conforms to the shared `check_frontend` harness.
- **Honest gaps:** `\pm`/`\mp` and `\binom` have **no** neutral representation, so they lower to
  a well-formed spanned `FrontendError` rather than being faked. Extending the neutral AST to
  cover them is a future `math-frontend`-crate change, not a hack here.
- **Feature-gated:** the adapter (and the only dependency, `math-frontend`) sit behind the
  default-on **`frontend`** cargo feature. `--no-default-features` builds the zero-dependency
  L0–L5 document/math parser alone (verified: core tests pass under `--no-default-features`).
- Total / panic-free: the lowering walks the tree with an **explicit work stack** (not native
  recursion), so its call-frame usage is O(1) in tree depth. This matters because a LaTeX math
  tree can be arbitrarily deep along a left-associative spine (`a+a+a+…`, juxtaposition `aaa…`,
  a chained relation) that `parse_math`'s nesting `MAX_DEPTH` does **not** bound — a recursive
  lowering would overflow the stack (an uncatchable abort) on such adversarial input. No `unsafe`.
- +16 tests (frac/pow/subscript/root/func/bigop/rel/implicit-mul/normalization/symbol/text/
  group/accent/matrix/exact-number/gap-errors/parse-error-span/registry/conformance, plus a
  deep-chain no-overflow regression at depth 4000). **136 unit + 1 doc test** green with default
  features; core green under `--no-default-features`; clippy `-D warnings` clean both ways. Crate
  0.9.0 → 0.10.0. **This completes the LTX01 ladder (L0–L6).**

## [0.9.0] — 2026-06-27

### Added — LTX01 L5d: document-structure recognition

- **`recognize_structure(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand` / `recognize_accents`) that classifies the *generic* `Node::Command`s produced by
  `parse` (L1) into **semantic** structure nodes. `parse` (L1) is unchanged, so its round-trip
  is preserved; run the pass, or don't.
- Four new `Node` variants (+ a `SectionLevel` enum):
  - **`Node::Section { level, starred, short, title }`** — `\part`/`\chapter`/`\section`/
    `\subsection`/`\subsubsection`/`\paragraph`/`\subparagraph`, including the starred form
    `\section*{T}` (the intervening `Text("*")` sibling is folded) and the optional short TOC
    title `\section[Short]{Title}`.
  - **`Node::CrossRef { command, note, target }`** — `\label`/`\ref`/`\eqref`/`\pageref`/
    `\autoref`/`\nameref`/`\cite`/`\citep`/`\citet`, keeping the `\cite[note]{key}` optional.
  - **`Node::Preamble { command, options, name }`** — `\documentclass`/`\usepackage`/
    `\RequirePackage` with their `[options]`.
  - **`Node::Styled { command, content }`** — argument-form text font commands (`\textbf`,
    `\textit`, `\texttt`, `\emph`, `\underline`, …). Font *declarations* (`\bfseries`,
    `\itshape`, …) stay plain `Command`s — their effect is positional, not a wrapped argument.
- **`to_latex`** renders each recognized node back to the exact shape the pass folds, so
  `recognize_structure(parse(&n.to_latex())) == [n]`. The pass recurses into groups, command
  arguments, environment bodies, and the parts of already-recognized nodes, so it is idempotent
  and composes with itself.
- Total / panic-free: a command that does not match its expected shape (a sectioning command
  with no title, a cross-ref with no key, a styled command with the wrong argument count) is
  left as a plain `Command`, never dropped or mis-folded. Recursion is bounded by the L1 tree
  depth (`MAX_DEPTH`). No `unsafe`; no `MAX_DEPTH` change (Node size still dominated by
  `Environment`).
- +14 tests (plain/starred/short sectioning, all seven levels, title-less command left alone,
  cross-refs, citation note, preamble, styled, declaration-stays-command, recursion into
  titles, surrounding text preserved, idempotence, round-trip corpus). **117 unit + 1 doc
  test** green; clippy `-D warnings` clean. Crate 0.8.0 → 0.9.0.

## [0.8.0] — 2026-06-27

### Added — LTX01 L5c: text accents

- **`recognize_accents(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand`) that folds an accent control sequence and the character it accents into a new
  `Node::Accent { accent, arg }`. `parse` (L1) is unchanged, so its round-trip is preserved.
- Recognizes both spellings: control-symbol accents `\'  \`  \^  \"  \~  \=  \.` (which take no
  L1 argument, so they pair with the next node) and control-word accents `\u \v \H \c \d \b
  \r \t` (captured as `\c{e}`, or `\c e` where the lexer absorbed the space). The argument is
  a single following character (`\'e` → é over `e`, the rest of the run kept as text) or a
  braced group. Recurses into groups, command arguments, and environment bodies.
- **`Node::Accent::to_latex`** renders the braced form `\'{e}`, so
  `recognize_accents(parse(&n.to_latex())) == [n]` whether the source wrote `\'e` or `\'{e}`.
- Total / panic-free: a dangling accent (nothing accent-able after it) is left as a plain
  command, never dropped or mis-folded. No `unsafe`.
- +9 tests (control-symbol over next char, first-char-only with remainder kept, braced arg,
  control-word braced + bare, accent-in-group, non-accent untouched, dangling, round-trip
  corpus). **103 unit + 1 doc test** green; clippy `-D warnings` clean. Crate 0.7.0 → 0.8.0.

## [0.7.0] — 2026-06-27

### Added — LTX01 L5b: `verbatim` environment

- **`\begin{verbatim}…\end{verbatim}`** (and `verbatim*`) read their whole body **raw** —
  catcodes suspended, newlines included — up to the matching `\end{<env>}`. New
  `TokenKind::VerbatimEnv { env, content }` → `Node::VerbatimEnv { env, content }`, with a
  round-tripping `to_latex`. The lexer peeks after `\begin` (consuming nothing) and only
  diverts to raw scanning for `verbatim`/`verbatim*`; every other `\begin{…}` is still parsed
  structurally. `\end{…}` for a different name inside the body stays literal.
- Total / panic-free / spanned: an unterminated `verbatim` environment (or one closed with the
  wrong name) is a spanned `LexError`; the raw scan advances one char per step and terminates
  at EOF. No `unsafe`.

### Fixed

- Lowered the structural-parser nesting cap `MAX_DEPTH` 512 → 256. The new owned-`String`
  token/AST variants enlarged recursive-descent frames enough that 512-deep pathological input
  could overflow a small (2 MB) test-thread stack; 256 trips the spanned "nesting too deep"
  guard well within it. Real documents never nest this deep.

### Tests

- +9 (lexer: verbatim-env raw body incl. `{}$#`+newline, `verbatim*`, non-verbatim `\begin`
  left to the parser, inner wrong-`\end` stays literal, unterminated/wrong-close errors;
  parser: `VerbatimEnv` node + 2 round-trips). **94 unit + 1 doc test** green; clippy
  `-D warnings` clean; no `unsafe`. Crate 0.6.0 → 0.7.0.

## [0.6.0] — 2026-06-26

### Added — LTX01 L5a: inline `\verb` verbatim

- **Inline verbatim** `\verb<delim>…<delim>` and the `\verb*` visible-space variant. The
  tokenizer now intercepts `\verb` and reads its body **raw** — catcodes are suspended, so
  `{ } $ # \` etc. are literal characters (previously L1 mis-tokenized them). New
  `TokenKind::Verb { star, delim, content }` → `Node::Verb { star, delim, content }`, with a
  round-tripping `to_latex`.
- Total / panic-free / spanned: an unterminated `\verb`, a body that runs past the end of the
  line, `\verb` at end of input, or a `*`/space delimiter each return a spanned `LexError` —
  never a panic or a mis-parse. The body is bounded by the input (single line).
- +7 tests (lexer: raw body with `{}$#\`, `\verb*`, the error family; parser: `Verb` node,
  surrounding text undisturbed; +2 round-trip-corpus entries). **87 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`. Crate 0.5.0 → 0.6.0.
- Scope: this is L5a. The `verbatim` environment, text accents (`\'e`…), sectioning/font
  recognition, and cross-refs are later L5 sub-rungs (spec §5).

## [0.5.0] — 2026-06-26

### Added — LTX01 L4a: macro expansion

- **`expand(nodes: Vec<Node>) -> Result<Vec<Node>, ParseError>`** — a new, opt-in pass over
  the structural document tree (`parse` stays purely structural, so its round-trip is
  preserved). It registers user macros and replaces their uses by substituted, recursively
  expanded bodies; definitions vanish from the output (as in LaTeX).
- **Definitions**: `\newcommand`/`\renewcommand`/`\providecommand` with positional arity
  `[n]` and bodies referencing `#1`..`#9`. Handles L1's argument-capture quirk (it stops the
  greedy `{…}` run at the `[n]` arity bracket) by re-scanning the definition's sibling nodes.
- **Substitution** walks the tree (groups, command arguments, environment bodies) so `#n`
  inside `\bar{#1}` works; `##` is a literal `#`; arguments are expanded call-by-value.
- **Bounded & safe**: total, panic-free, spanned errors. Two guards stop runaway expansion —
  a recursion-depth cap (`MAX_EXPANSION_DEPTH`) and a work-budget cap (`MAX_EXPANSION_STEPS`)
  — so a self-recursive macro or an expansion bomb errors instead of hanging/overflowing.
  Bad calls (too few args, parameter out of range, malformed definition, or an unsupported
  `[n][default]` optional-with-default) are spanned errors.
- **Honest scope (L4a)**: positional args only. Deferred: optional arguments with a default,
  TeX-style `\def`, a built-in starter set, and `#n` substitution inside math islands.
- +16 macro tests (zero/one/two-arg, reordering, macro-calls-macro, param-in-group,
  redefinition, unknown-command pass-through, extra-group retention, `##`, recursion &
  too-few-args & out-of-range & default-arg & malformed-definition errors). **80 unit + 1 doc
  test** green; clippy `-D warnings` clean; no `unsafe`. Crate 0.4.0 → 0.5.0.

## [0.4.0] — 2026-06-26

### Added — LTX01 L3: math environments

- **`MathNode::Matrix { env, rows: Vec<Vec<MathNode>> }`** — math environments with
  row/column structure. `parse_math` now handles `\begin{env} … \end{env}` inside a math
  island: `&` separates columns, `\\` separates rows, and each cell is a full math
  expression. Supported environments (case-sensitive — `bmatrix` ≠ `Bmatrix`): `matrix`,
  `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`, `Vmatrix`, `smallmatrix`, `cases`, `dcases`,
  `aligned`, `gathered`, `align`, `align*`, `split`.
- Environments **nest** (a cell may itself be an environment — depth-guarded via the
  enclosing atom), and a `Matrix` is an **atom**, so postfix scripts attach
  (`\begin{pmatrix}…\end{pmatrix}^2`).
- **`MathNode::to_latex`** renders the grid back and **round-trips**
  (`parse_math(&m.to_latex()) == m`); a trailing `\\` before `\end` is tolerated and does
  not create an empty final row.
- Total / panic-free / spanned: `\begin`/`\end` name mismatch, unterminated environment,
  unknown environment, a missing `{` after `\begin`, and a stray `\end` each return a
  spanned `ParseError`. Empty cells (`a & & b`) are a documented limitation (spanned error,
  never a silent empty node), as are `array`/`tabular` column-specs and document-mode list
  environments — those arrive in a later layer.
- +9 environment tests + 5 round-trip-corpus entries; **64 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`.

## [0.3.0] — 2026-06-26

### Added — LTX01 L2: math grammar

- **`parse_math(&str) -> Result<MathNode, ParseError>`** — a precedence-climbing parser
  over a math island's raw inner source (the string L1 keeps in `Node::Math`). Re-uses the
  L0 `tokenize` and filters space/par/comment tokens, then climbs:
  relations (`= ≠ < ≤ > ≥ \approx \equiv`) < add/sub (`+ - \pm \mp`) < mul/div
  (`\times \cdot \div /` **and implicit multiplication** via adjacency — `2x`, `\pi r`,
  `(a)(b)`) < unary `± ∓` < scripts (`^`/`_`, right-assoc) < atoms.
- **`MathNode` AST** (`math.rs`): `Num`, `Sym`, `Bin`, `Unary`, `Frac`/`\dfrac`/`\tfrac`,
  `Binom`, `Root { degree, radicand }` (`\sqrt[n]{}`), `Script { base, sub, sup }`,
  `Call { func, arg }` (named functions `\sin \log …`), `BigOp { op, lower, upper, body }`
  (`\sum \prod \int \lim` with bound scripts), `Accent` (`\hat \bar \vec …`),
  `Fenced { left, body, right }` (`\left( … \right)`, `\langle`, `|`, …), `Text`, and
  `Rel`. `{…}` groups are **transparent** (grouping only — they do not appear as nodes).
- **`MathNode::to_latex`** — precedence-aware round-trip: `parse_math(&m.to_latex()) == m`.
  Children below the parent's precedence are wrapped in invisible `{…}` so the re-parse
  re-associates identically.
- **`Node::parsed_math`** — parses a `Node::Math` island on demand; the L1 structural tree
  is unchanged (its round-trip stays intact).
- Total / panic-free / spanned; recursion is **depth-guarded** (`MAX_DEPTH`) so adversarial
  nesting (e.g. thousands of `(`) returns a spanned error instead of overflowing the stack.
- +15 math tests incl. the worked corpus (`\frac{12 \times 15}{3}`, `2^{10}`,
  `\sqrt[3]{27}`, `\sum_{i=1}^{n} i`, `\left(\frac{a}{b}\right)^2`), a round-trip corpus,
  relations/functions, and the deep-nesting bound; clippy `-D warnings` clean, no `unsafe`.

## [0.2.0] — 2026-06-26

### Added — LTX01 L1: structural document parser

- **`parse(&str) -> Result<Vec<Node>, ParseError>`** — a recursive-descent parser that
  turns the L0 token stream into a document tree:
  - ordinary characters coalesced into `Text`; `Space`/`Par`;
  - `{ … }` → `Group`;
  - `\cmd[opt]{arg}…` → `Command` (one optional `[…]` if it immediately follows, then a
    greedy run of mandatory `{…}` groups — generic capture; per-command arity is a later
    layer, so `\textbf{a}{b}` captures two args, and a space breaks the run);
  - control symbols (`\,`, `\\`, `\{`) → argless `Command`;
  - `\begin{env}[opt]{arg}… body \end{env}` → `Environment` with a **matched** close
    (a `\begin{a}…\end{b}` mismatch is a spanned error); environments nest;
  - math islands (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) → `Math { display, content }` keeping
    the **raw inner source** for L2;
  - comments, active `~`; in text mode `& # ^ _` are literal characters.
- **`Node` AST** (`ast.rs`) with **`Node::to_latex`** / `document_to_latex` — round-trips:
  `parse(&render(ast)) == ast` (AST-equality; surface spacing and `$`/`\(` delimiter
  choice are normalized). Reserves an `Unsupported { construct, span }` variant for the
  TeX-programmability asymptote (not produced at L1).
- **`ParseError`** — spanned; structural errors (unbalanced braces, env mismatch,
  unterminated env/math) never panic.
- +19 tests (39 unit + 1 doc total), incl. a round-trip corpus; clippy `-D warnings` clean.

## [0.1.0] — 2026-06-26

### Added — LTX01 L0: crate scaffold + catcode tokenizer

- New standalone, **zero-dependency** crate `latex` (added to the Rust workspace members).
  A full-fidelity LaTeX parser for documents *and* math; first frontend of the
  `math-frontend` framework.
- **`catcode(c)`** — TeX category codes (default plain-LaTeX assignments): Escape,
  BeginGroup, EndGroup, MathShift, AlignTab, EndLine, Parameter, Superscript, Subscript,
  Space, Letter, Other, Active, Comment.
- **`tokenize(&str) -> Result<Vec<Token>, LexError>`** — a catcode-driven, **text-mode-
  primary** state machine:
  - mode stack: Text (primary) ↔ Math (pushed by `$`/`\(`/`\[`, display via `$$`/`\[`,
    popped by the matching close); whitespace is significant in text, ignored in math;
  - control words (`\`+letters, with TeX space-absorption) and control symbols
    (`\`+non-letter, incl. `\\` line break, `\{`, `\,`, …);
  - groups `{ }`, math on/off (`MathOn`/`MathOff` with inline/display flag), `&`, `#`,
    `^`, `_`, active `~`, comments (`%` to end of line, eating the newline);
  - whitespace: a run collapses to one `Space`; a blank line (≥2 newlines) is `Par`;
  - ordinary characters emitted one-per-`Char` (faithful to TeX; the parser coalesces).
- **`Token` / `TokenKind` / `Span`** — every token carries a half-open byte span.
- **`LexError`** — spanned; the scanner **never panics** (trailing `\` → spanned error;
  a stray `\)`/`$` in text mode does not underflow the mode stack).
- 20 unit + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Scope is full LaTeX surface; the Turing-complete TeX tail is the documented asymptote
  (see LTX01). The structural parser (L1), math AST (L2), environments (L3), macros (L4),
  text breadth (L5), and the `MathFrontend` adapter (L6) arrive in subsequent layers.
