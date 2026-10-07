# CV01 — Compact identities for compiler provenance

## Scope and evidence

This is the first bounded implementation slice of CCR-065 foundation #16868.
CLOC31 merged in #16866 at `84252d038a9fa2eddb5f92833d0de242969e45c8`,
with exact-head security review and macOS/Ubuntu/Windows CI, and its merge was
verified on fetched main. Full compiler and provenance completion remain open.

The current hierarchical derivation embeds the parent ID into each child.
Direct-library chains at depths 128/256/512/1024 serialized to
63,373/224,909/842,893/3,258,509 bytes; depth 1024's ID is 2,058 bytes.
The parser's CLI nesting guard does not protect direct AST/library callers.
Expanding composite node coverage requires constant-size identities first.

This slice covers compact allocation, safe allocation-state round trips and
compiler integration. It does not complete graph resource limits, checked
query/import boundaries, canonical serialization, typed event chronology,
composite AST coverage, exact source spans or output-byte joins. Foundation
#16868 remains open until those acceptance requirements are satisfied.
Remaining successful call/composite fold lineage is separately tracked by
#16875; the four recorded CLI cases are an initial inventory, not coverage.

## Contract

1. Preserve `CVLog::new`'s default hierarchical ID behavior and generic client
   compatibility. Add an explicit opt-in compact mode for the compiler's one
   shared run log. Do not introduce crates.io dependencies or change AST CVs
   away from their existing string identity type.
2. Compact identities are versioned, deterministic for identical allocation
   histories, unique within a log, and bounded in length independently of
   ancestry depth. Use one allocation sequence for roots, derivations and
   merges. Actual parent edges carry lineage; ID spelling does not.
   Identities must not be advertised as globally unique across independent runs.
3. Allocation consumes sequence values even when entry storage is disabled,
   preserving existing enabled/disabled semantics. Origins, parent order,
   contributions, tombstones and pass summaries retain their current meaning.
4. Persist explicit identity-scheme/version and sufficient allocation state to
   continue without reuse after reload, including empty/partially stored logs
   whose IDs were allocated while disabled. Stored-entry scans alone cannot
   reconstruct that watermark. Encode full-width counters without JSON numeric
   precision loss in other languages. Reject unknown versions and inconsistent
   compact identity/state before accepting a compact snapshot.
5. Check allocation exhaustion before mutating a counter or inserting an entry.
   Provide a fallible allocation path that returns a clear error; never wrap,
   overwrite an existing identity or fabricate a replacement. Preserve old
   signatures deliberately and document their failure behavior. Compiler
   integration must not turn an allocation failure into successful incomplete
   provenance. Graph node/edge/event/work/output budgets are separate follow-up
   work and cannot be implied by the counter width.
6. Every JSON and pretty-JSON sidecar and NDJSON metadata footer preserves
   identity scheme/state. Audit the footer, which currently copies only
   pass_order/enabled. NONE intentionally writes no sidecar but computes the
   same provenance. Summary/filter policies must be accurate; a filtered view
   cannot claim complete ancestry or full reloadability merely because it
   retained the allocation metadata.
7. Tracing on/off preserves emitted JavaScript bytes. Primitive result-owned
   lineage assertions from CLOC31 must remain meaningful with compact IDs;
   compare actual graph edges, sources and child histories, not string suffixes.

## Verification

### Wire and API decisions

Compact-v1 IDs are `cv1.` followed by sixteen lowercase hexadecimal digits,
starting at one. The JSON `identity` object declares `scheme: "compact-v1"`
and `last_sequence` as sixteen hexadecimal digits, including zero for an unused
allocator. Legacy snapshots omit this object. `CVLog::new_compact` opts in;
`try_create`, `try_derive` and `try_merge` return allocation errors without
changing state. Existing string-returning APIs fail fast on allocation error;
they never return a success-shaped substitute. These are allocation checks,
not the future bounded graph-validation API. Compact import checks ID/state
shape, matching map keys and counter coverage, including parent identity values.
Filtered compact views carry `view: {filtered: true, complete: false}` and are
rejected as full reloadable logs. JSON and NDJSON preserve that declaration.
Until a complete-view schema is introduced, compact import rejects any declared
`view`, including malformed or contradictory markers, instead of trusting a
false `filtered` field as permission to reload a partial graph.

Presence is also significant for allocator state: legacy snapshots omit
`identity`; a present field must deserialize as a valid versioned object.
`identity: null` is malformed, including in empty or disabled snapshots, and
must not silently select the legacy allocator. The entry map must reject
duplicate JSON identity keys before consuming the duplicate payload instead
of retaining its last record. Apply entry-key rejection to legacy snapshots
too; snapshots produced by either allocator already have unique entry keys.
This prevents ambiguous reloads without claiming full graph or metadata-map
validation. Test empty/disabled null state, both allocator modes, escaped
duplicate-key spellings, identical/conflicting records and rejection before
decoding a malformed duplicate value. Parent-list duplicates retain CV01's
existing semantics. Metadata-map duplicate keys and bounded import remain
subsequent checked-graph/schema work.

- Keep the existing generic CV suite and hierarchical spelling contracts green.
- Verify mixed roots, branching, repeated parents, merges, tombstones,
  contributions, origins, disabled allocation and mode toggling.
- Save/reload an enabled log, a disabled empty log and a partially stored log;
  continue allocating across every operation without collision. Exercise
  malformed/unknown scheme, malformed IDs/state and exact counter exhaustion
  through the fallible path, asserting rejection leaves state unchanged.
- Measure direct-library chain identity length and serialized identity overhead
  at several doubling depths, including at least 1024 and 4096. Assert constant
  identity length and linear overhead, not just a small CLI smoke test.
- Exercise actual SIMPLE/ADVANCED CLI output with all sidecar formats,
  pretty/filter/summary/NONE modes, scheme metadata and graph assertions. Run
  fresh processes to verify identity allocation determinism separately from
  the known object-key serialization defect owned by #16868.
- Run affected CV/lexer/parser/pipeline/folder/compiler tests and strict all-target
  lint, lessons validation, diff check and exact-head independent security review.
  Verify actual required cross-platform CI and fetched-main merge reachability.

## Required subsequent foundation work

The built current library accepts dangling parents, map-key/entry-ID mismatch,
cycles and derivation from unknown parents. Its shared-ancestor lineage order
can place a child before a parent. Default compact JSON output varies across
fresh processes because graph and metadata HashMaps serialize in random order.
Checked, bounded graph operations and canonical serialization must repair these
defects and reject invalid/incomplete graphs explicitly. The CLI currently has
successful empty-object, fallback-format and zero-summary error paths; replace
them with failures and a verified artifact policy. Topological node order is
not actual transformation chronology: typed sequence/pass/sweep events remain
required. Compact allocation alone does not make provenance rock solid.
