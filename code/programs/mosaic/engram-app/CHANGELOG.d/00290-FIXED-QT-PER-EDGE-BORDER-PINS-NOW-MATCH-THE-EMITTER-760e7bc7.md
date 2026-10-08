### Fixed — Qt per-edge border pins now match the emitter

Qt now consumes the authored bottom-border width and colour on Engram's deck
list entries when it emits their native edge. The native-completeness gate no
longer carries stale allowlist entries for those two properties, so a future
regression will be visible again (#17128).
