# CLOC32: run-scoped array occurrence observation at the typed bridge

## Bounded purpose

CLOC27 already copies physical token CVs into primary typed leaves. CLOC31
now retains available array-length operands, but arrays without such operands
still lack composite occurrences. A later trusted CLI adapter must create fresh
array identities with exact shared SourceRange provenance. Before that adapter,
provide a bridge construction hook at the actual typed occurrence, avoiding
posthoc value/coordinate matching or a second incompatible span representation.

This slice is library infrastructure only. It does not wire CLI array identities,
certify sink-returned strings, record source span metadata, change member/optional
suffix handling, introduce a new serialized namespace or complete CCR-065.
The immutable design audit at 1dbe72de recommends this separation.

## API and conversion contract

- Preserve grammar_to_program(node,version), its BridgeError type, diagnostics,
  existing leaf IDs, AST fields and serialization. Its no-observer route must
  not allocate a registry, clone syntax trees or do extra source traversal.
- Add ArrayOccurrenceObserver: Send and
  grammar_to_program_with_array_observer(node,version,&mut observer). The sink
  receives the exact borrowed GrammarASTNode for each successfully constructed
  ArrayExpression, exactly once in deterministic child-before-parent order.
  Empty arrays, holes, nested arrays and repeated identical source occurrences
  are included. The sink returns Result<Option<String>,String>; only the new
  array.cv field may be assigned from that result. No semantic AST mutation.
- Add an observed-entry error wrapper distinguishing ordinary Bridge(BridgeError),
  Observer(String) and Worker(String); do not add a variant to legacy BridgeError.
  Context is borrowed/run-scoped; no TLS, global identity state, pointer-based
  serialized IDs, unrelated source lookup or generic GrammarASTNode/Token fields.
- Thread context only through the existing conversion call graph reaching array
  construction. Preserve conversion bodies modulo that plumbing and the array
  hook. A context first-error latch owns the original sink error, stops later
  sink calls and is checked at the observed public boundary. Even if a future
  converter catches an internal unwind sentinel, a partial AST cannot succeed.
  Current code has no production convert_*.ok() swallowing defect; the latch is
  a deliberate boundary guarantee, not a claim that one was found.
- The observed entry uses a scoped 128 MiB worker with borrowed Send sink/CST.
  Creation/join failure returns Worker. On observer failure, dispose of any
  partial candidate on the worker stack before returning an error to a small
  caller. Retain existing grammar/depth guards. The successful returned AST and
  arbitrary caller-constructed CST still have the caller's existing ownership/
  depth responsibilities; this API does not add a whole-program admission bound.
- Observer code, returned string validity, graph allocation, source certification
  and its own work are caller responsibilities. This plumbing adds no registry
  or source/AST walk; it does not promise to sandbox arbitrary Rust callbacks.
  A trusted adapter must independently bound metadata/resolution and validate
  its closed source/node domain before making certified provenance claims.

## Tests before implementation

Commit tests first; an explicit delegating API scaffold may then provide a
compiled behavioral red before context implementation. Mark scaffold evidence
honestly; absence-of-API compile failure is not a behavioral regression.

Use real CLOC27 lexer IDs and a test sink allocating distinct CV nodes. Assert
exact typed array.cv ownership, complete ordered token parents, repeated-text
occurrence separation and postorder calls. Compare complete Program equality
for a no-op sink and native CV-erased equality for annotated results, preserving
all non-CV fields. Cover arrays under expression, declaration, function/default,
arrow, class, computed-property and statement parents supported by the bridge.
Reject a sink error under those same paths, preserving its exact first error
and refusing a Program. Test checked graph exhaustion from a sink, ordinary
unsupported/wrong-root error separation, legacy no-hook controls, and a nested
independent observed run proving context isolation. A subprocess with a 128 KiB
caller and a 4,096-member child must reject an array sink failure without moving
the deep candidate destructor onto the caller stack; leak an unexpected scaffold
success in that child before failing, so the red is a normal assertion failure.

Run the complete javascript-parser, constant-fold and closurec suites, strict
all-target lint for affected crates, and native compiler/downstream parser
consumers where applicable. Preserve all upstream/parity/source expectations,
production limit tests and no new crates.io dependencies. Obtain independent
exact-source and terminal security review. Spec-first trusted array SourceRange
adapter and then per-suffix member prefix identities are the next core slices.

Miri remains DEFERRED/PENDING under the October 11 delivery sequencing override.
Frozen checkout/run/inputs/receipts/target stay untouched and unpolled; genuine
final-source acceptance follows implementation delivery. Full objective OPEN.
