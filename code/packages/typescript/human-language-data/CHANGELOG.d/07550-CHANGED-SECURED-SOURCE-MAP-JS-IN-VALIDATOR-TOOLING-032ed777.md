### Changed — secured source-map-js in validator tooling

- Updated the locked transitive development dependency `source-map-js` from
  1.2.1 to 1.2.2. Vitest/Vite/PostCSS and magicast now resolve the fixed
  version without changing the package's direct dependencies or lesson data.
- Kept the package's build, test, and validation gates intact. The broader
  sibling-lockfile remediation is tracked separately in #16874.
