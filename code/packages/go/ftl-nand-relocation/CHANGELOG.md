# Changelog

## Unreleased

- Add `CaptureEraseIntent` to retain ordered stale-page roles before the first
  erase command. Share raw erase eligibility checks and preserve target,
  generation, sequence, and detached stale-mask evidence.
- Add `RestoreWithInterruptedErases` for reopening feasible stale-page erase
  prefixes. Preserve erase progress and whole-block quarantine across shorter
  retries, and release the extent only after full media completion.
- Add value-owned NAND-backed page relocation composed from the local DB05a
  page map and DB04b1 program-cut runtime modules. Preserve explicit media
  progress, ready publication and caller acknowledgement as separate facts.
- Add detached reads and snapshots, refusal rollback and precedence, a strict
  bounded shared vector checked against an independent media/map model, finite
  small-device states, seeded and fuzzed histories, race and coverage checks,
  and four disposable correctness mutations.
- Add the optional DB05c2b block-reclamation profile. Partial erases quarantine
  every block slot, completed media results release the full page-map extent
  even when unacknowledged, and writes fence all quarantined blocks. Add a
  strict model-first reclamation vector, bounded graph and seeded histories,
  fixture/history fuzzing and three named block-reclamation mutation controls.
- Add DB05c3b1 `CopyLivePage`: recheck a retained evacuation assignment,
  reserve exactly its destination, and reuse the shared program transition.
  The eight-case portable vector is validated against the independent model
  before runtime replay. Add all-cut/source-ownership tests, stateful and
  bounded fixture fuzzing, and three clean-controlled copy mutations.
- Add DB05c3b2 single-target whole-block evacuation coordination. The active
  fence excludes ordinary writes, each copy replans current ownership, and
  the target remains locked through partial erase until media completion.
  Add an explicit model-first shared vector with detached states and receipts,
  finite state graphs, seeded and coverage-guided histories, and four named
  mutation controls.
- Add `WriteExcluding` so a caller can supply an allocation fence without
  changing ordinary `Write` behavior. Combine caller exclusions with active
  evacuation and incomplete-erase fences, retaining page-map address,
  reservation, mask-width and capacity refusal order. Add exhaustive small
  mask/state checks, model-checked histories, atomic refusal and clean
  mutation coverage.
- Add `PlanRetirementDrain` as a read-only device view over the admitted
  planner. Combine caller block exclusions with incomplete extents and the
  active reclamation target while allowing a caller-excluded source. Verify
  source and destination ordering, zero-budget planning, ownership and
  refusal precedence against a separate model, fixed seeds and four named
  clean-controlled internal-fence, external-fence and budget mutations.
- Add `Device.LivePagesInBlock` as a read-only map-based occupancy query. Count
  only current Live slots while allowing empty, full, active, incomplete and
  exhausted blocks. Compare two closed model graphs, seeded mixed histories
  and 110,000 fuzz inputs, with signed-boundary and Live-only mutation tests.
- Add `Device.OwnersInBlock` as a detached, ascending-logical inventory of the
  current owners in one physical block. Share status/profile/signed-address
  validation with the live-count query, and check all write, copy, and erase
  cut ownership transitions against bounded, seeded, and fuzzed models.
- Add process-local `OwnerObservation` identities for each media-ready host
  write or live-page copy. `MatchesOwner` requires the same publication stamp
  and current coordinates; aborted cuts, unrelated operations, and erases
  retain unchanged current-owner identities across immutable branches.
- Add `Device.InvalidateOwner` to remove one mapping only when its exact current
  publication observation matches. Clear only that owner's identity while
  preserving media bytes, erase metadata, other owners, and active evacuation.
- Add `Device.Lookup` for a logical page's current physical owner. Preserve
  geometry-before-address refusal and distinguish absence from a live mapping
  to physical page zero. Route `Read` through the same lookup and compare all
  logical owners after relocation, copies, cuts and erases.
- Add `Device.ObserveBlock` for a detached physical-order block image with
  exact page bytes, program markers, completed generation and incomplete flag.
  Active and incomplete extents remain observable as raw evidence.
- Add `ProgramUnownedPage` to apply deterministic program cuts to a caller-
  selected free physical page without changing logical ownership. Consumed
  pages become stale and retain bytes, marker and acknowledgement evidence.
- Add `Device.ObserveMedia` to capture a whole block-enabled physical image in
  ascending order. It shares the detached page-copy path with `ObserveBlock`,
  preserves active and incomplete erase evidence, and returns zero on the same
  geometry/profile refusals.
- Add `RestoreQuiescent` to validate and reopen a detached raw-media and page-map
  pair. Check geometry, block erase evidence, erased-byte invariants and page
  role/marker parity before copying media, then issue fresh process-local owner
  identities without restoring an active evacuation.
- Add `Device.ObserveReclamation` as a detached role/progress view for victim
  selection. It returns ordered physical roles, block erase progress, the
  geometry and ceiling, and an explicit active-target bit without copying NAND
  payload bytes. Validate all lifecycle states, maximum geometry, bounded
  seeded/fuzzed histories, and clean-controlled alias/progress mutations.
- Add `BeginEvacuationExcluding` to preflight a stale source against caller
  destination fences and incomplete-erase quarantine. Preserve legacy
  validation order and prove exact spare capacity, mask ownership, bounded
  masks, seeded histories, and clean-controlled refusal and victim-bit cases.
- Add `CopyNextEvacuationPageExcluding` to replan all current live sources
  against each command's caller destination fence. Keep cut receipts and
  predecessor media exact, and cover a spent sole destination, full-plan
  capacity, validation order, bounded masks, seeded histories and mutations.
- Add `RestoreQuiescent` to bind a settled page-map snapshot to ordered NAND
  bytes and erase progress. Validate the complete paired observation before
  copying media, start with no active evacuation, and regenerate process-local
  owner identities for current mappings.
