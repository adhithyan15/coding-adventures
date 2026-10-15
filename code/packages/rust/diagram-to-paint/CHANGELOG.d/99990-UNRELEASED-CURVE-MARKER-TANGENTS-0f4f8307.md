## Unreleased curve marker tangents

- Fix directed marker collapse on aligned Bump and Step swimlane routes by
  selecting the first noncoincident endpoint control/waypoint, then the opposite
  endpoint, without changing curve commands, metadata or resource limits.
- Run authored parser -> layout -> paint exact marker geometry controls on every
  platform using existing local parser and hierarchy dev dependencies. Cover
  both endpoint triangles, Bump axes, Basis/Linear and all Step variants, and
  preserve explicit zero-length compatibility. Tracked in #17373.
- Extend the same authored exact route/triangle/metadata controls to MonotoneX
  and MonotoneY after current-main integration, retaining all older assertions.
