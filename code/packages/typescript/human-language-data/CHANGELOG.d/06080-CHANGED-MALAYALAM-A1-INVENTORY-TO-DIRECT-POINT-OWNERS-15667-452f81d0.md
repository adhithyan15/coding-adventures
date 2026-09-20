### Changed — Malayalam A1 inventory points have direct owners (#15667)

- Replace the shared 4,839-line Malayalam A1 inventory with canonical metadata
  and one self-binding owner per ordered exam point. The loader reconstructs
  the exact public inventory while strict completeness and filesystem checks
  reject missing, extra, malformed, linked, or resurrected state.
