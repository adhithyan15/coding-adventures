# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added D18D handlers for the D23A smart-home integration catalog tools:
  `smart_home.list_integrations`, `smart_home.describe_integration`,
  `smart_home.list_primitives`, and `smart_home.describe_primitive`.
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
- Added D23 discovery worker retry policy fields to
  `smart_home.observe_supervision`, including configured retry delays,
  multiplier, and the current retry delay during failure pressure.
- Added `smart_home.poll_events` and `smart_home.unsubscribe` handlers so Chief
  of Staff jobs can drain, peek, summarize, and retire runtime event
  subscriptions without bypassing D23 authorization.
- Added `smart_home.list_scenes` and `smart_home.describe_scene` handlers over
  the D23 runtime scene read facade, including Hue-style fixture coverage.
