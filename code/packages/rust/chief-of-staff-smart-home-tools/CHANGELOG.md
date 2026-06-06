# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added D18D smart-home tool definitions and in-memory handlers over
  `SmartHomeRuntime`.
- Added an end-to-end Hue-style fixture test that lists devices, commands a
  light, reads optimistic state, and records a D18D execution journal entry.
- Added D18D handlers for `smart_home.subscribe`,
  `smart_home.pair_bridge`, `smart_home.describe_capabilities`, and
  `smart_home.get_health` so Chief of Staff jobs can reach the existing D23
  subscription, pairing, capability, and health runtime paths.
- Added the `smart_home.discover` D18D handler over
  `RuntimeDiscoverToolRequest`, including discovery filters, bridge-candidate
  output, and end-to-end journal coverage.
- Added scheduled discovery worker observability to
  `smart_home.observe_supervision`, including worker status, due time, last run
  counts, and failure pressure from the D23 runtime.
