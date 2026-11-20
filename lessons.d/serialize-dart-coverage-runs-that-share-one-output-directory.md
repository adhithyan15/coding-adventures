category: Testing & coverage
---

# Serialize Dart coverage runs that share one output directory

Two VM test processes wrote `.dart_tool/coverage` at the same time. Their
partial maps were not independently attributable, so both runs were stopped
and the shared coverage output was discarded. Run only one coverage producer per
package output directory, preserve an interrupted receipt as invalid, and
start the next run from a freshly prepared output directory.
