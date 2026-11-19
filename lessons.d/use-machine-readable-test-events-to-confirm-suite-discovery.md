---
category: Testing & coverage
---

# Use machine-readable test events to confirm suite discovery

Concurrent test runners can interleave or suppress human-readable prefixes, so
an aggregate pass count does not show whether a particular file ran. Before
changing discovery paths, inspect the runner's structured event stream and
confirm the relevant test IDs reached their terminal `testDone` events with
success. Keep the repository's normal discovery command when that evidence
shows the suite is included; explicit paths can accidentally narrow later CI
coverage.
