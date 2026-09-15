### Added — Chief Host Authenticated Data Plane
- The existing per-spawn secure host session now carries bounded, serialized
  channel receive/publish/acknowledge and provider-neutral text completion
  exchanges with monotonic request IDs and exact response correlation.
- The process supervisor exposes real-pipe child exchange helpers and retains
  authenticated pending requests for injected daemon service adapters, closing
  the missing data-plane seam before the production Chief host composition.

