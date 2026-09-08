# Changelog

## Unreleased

### Added -- an `Await` effect a host never completes is now visible in props

The fixture reports `awaitedEffects`: how many awaited effects it has emitted
and not seen answered.

This is the check UI47 §5.4 step 3 calls for, and it exists because of what it
guards against. `Effect` was serialised onto the wire and read by **no** host
for as long as it existed -- and nothing anywhere said so. The app simply
waited. `MosaicRuntime::pending_effects()` did track them, but that is
Rust-side, and a native host drives this fixture through the C ABI and sees only
props, so nothing a host could assert on ever mentioned it.

Now an acceptance run asserts the count returns to zero, and a host that
receives an effect and never calls `mosaic_app_complete_effect` leaves it
non-zero and fails visibly.

Three details, each of which would make the report wrong in a different
direction, and each with its own test:

- The update **carrying** the effect already reports it outstanding. Recomputing
  the props before the push would let a host read a stale zero and conclude it
  had nothing to answer.
- `Notify` is not counted. A host is never expected to answer one, so counting
  it would fail every correct host.
- **All three** completion outcomes clear it, not just `Ok`. A cancellation and
  a failure both discharge the obligation -- and cancellation is the single most
  likely thing to happen to an `importAnki` effect, since it is what a closed
  file dialog produces.

- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## Unreleased

- Added a minimal Mosaic package driven by the conformance engine so native
  packaging tests cover Rust engine plus MIL/MLL/MSL composition.
- Added the shared counter application and fixed C ABI exports used by native
  host-binding conformance tests.
