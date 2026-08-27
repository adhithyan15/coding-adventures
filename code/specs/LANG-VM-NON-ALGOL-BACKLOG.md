# LANG VM non-ALGOL completion backlog

Status date: 2026-08-27

This is the execution backlog for completing the shared LANG VM platform while
the ALGOL campaign is owned separately. It complements
`LANG-FULL-IMPLEMENTATION.md`; when the two disagree about landed behavior,
executed tests and current package changelogs are authoritative until the older
roadmap is reconciled.

## Prioritization policy

Re-rank before selecting every work item, and whenever current work discovers a
new gap. Apply this order:

1. Red executed conformance cells or silent backend skips.
2. Missing CI protection for already-working conformance.
3. Incorrect roadmap/status documentation that could send work down a dead path.
4. Missing backend parity for an already-implemented language feature.
5. New frontend semantics and historical-machine fidelity.

Each item must add or strengthen an executed cross-backend proof. One item uses
one fresh worktree and one PR; remove the worktree after merge.

## Ranked backlog

| Rank | ID | Status | Work item | Completion proof |
|---:|---|---|---|---|
| 1 | VM-001 | in progress | Preserve COBOL scale-12 decimal intermediates on JVM. | The existing `A / B + C` LANG matrix cell prints `000533` on real Java and the full matrix is green. |
| 2 | VM-002 | queued | Re-enable `lang_matrix` and `e6d7a_wasm_closures` in `lang-aot/BUILD`; remove stale red-suite comments. | The normal build command runs both suites on CI with no exclusions. |
| 3 | VM-003 | queued | Reconcile `LANG-FULL-IMPLEMENTATION.md` with landed Twig E6 and McCarthy work. | Roadmap statuses match current executed suites and name only genuine gaps. |
| 4 | VM-010 | queued | Bring Twig dynamic lists, records, unions, closures, symbols, and globals to VM/JIT parity. | Dynamic Twig programs run on VM and JIT in addition to the five code-generation backends. |
| 5 | VM-011 | queued | Close Dartmouth BASIC's non-ALGOL semantic tail: remaining math builtins and dynamic string data paths. | `SIN`, `COS`, `LOG`, `EXP`, `RND`, and remaining string `READ`/`DATA` cases have seven-backend executed cells. |
| 6 | VM-012 | queued | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| 7 | VM-013 | decision required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `adc`, `sbb`, rotations, carry, parity). | The accepted semantics are documented and every intrinsic has executed portable and 8008 proofs. |
| 8 | VM-014 | queued | Extend the unified matrix to every wired frontend, especially FLOW-MATIC and Macsyma/JIT. | Every `Language` variant has an executed baseline on every applicable standard backend, with explicit exclusions only where architectural. |

## Discovery log

- **VM-D001 — confirmed 2026-08-27:** JVM scalar concretization narrowed
  COBOL's explicit `i64` constant `10^12` to `i32`, so nested fixed-point
  division printed `000000` instead of `000533`. Promoted to VM-001.
- **VM-D002 — confirmed 2026-08-27:** `lang-aot/BUILD` still describes the
  closure suite as red even though all five tests pass, and excludes the full
  LANG matrix. Promoted to VM-002.
- **VM-D003 — confirmed 2026-08-27:** the LANG-FULL roadmap still marks Twig
  lists/closures/records/unions and McCarthy backend completion as open despite
  landed executed tests. Promoted to VM-003.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.
