### Fixed — Keyboard navigation across virtual row windows

- Request the existing authored viewport shift when an arrow reaches a vertical
  row-window boundary and another logical row exists.
- Validate integral finite geometry and confirm the exact resulting offset
  before dispatching cell activation. Bound retries and cancel stale generations
  or requests; preserve editor focus and reacquire replaced cells.
- Exclude nested tables from outer cell lookup.
- Execute the emitted continuation under controlled scheduling/focus probes;
  all 395 library and 14 integration tests pass. Regenerated Debug WinUI controls
  pass upper/lower edge interaction, repeated crossing and formula restart checks.
