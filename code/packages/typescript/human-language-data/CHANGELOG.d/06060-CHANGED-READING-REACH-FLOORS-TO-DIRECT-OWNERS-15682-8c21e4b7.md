### Changed — reading-reach floors have direct language/level owners (#15682)

- Replaced the shared `core/reading-reach-floor.json` ratchet with one canonical,
  self-binding owner per task-shape identity. Independent language/level gains
  no longer collide, while exact completeness, canonical bytes, safe paths, and
  the rule that absent public entries floor at zero remain enforced.
