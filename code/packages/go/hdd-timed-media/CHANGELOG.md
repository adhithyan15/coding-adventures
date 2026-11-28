# Changelog

## Unreleased

- Add sparse durable sector storage composed from configurable geometry, seek
  timing, and admitted byte-prefix cuts.
- Add read/write timing application, detached snapshots and restart, typed
  preflight precedence, and full-service clock admission before reporting a
  zero-transfer before-media cut.
- Add a zero-allocation head/clock observation that shares snapshot admission
  while leaving sparse media and timing unchanged.
- Add independent stepped models, both 512-sector profiles, a huge sparse
  near-ceiling geometry, seeded histories, and two clean-controlled mutations.
