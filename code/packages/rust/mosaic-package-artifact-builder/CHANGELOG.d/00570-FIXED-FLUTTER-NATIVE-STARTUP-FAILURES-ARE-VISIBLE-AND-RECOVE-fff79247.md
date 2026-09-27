### Fixed — Flutter native startup failures are visible and recoverable (#16084)

Strict generated Flutter projects now open on a system-theme-aware loading
surface, keep runtime and initial-props failures visible with selectable detail
and saved-data reassurance, and offer an in-place retry that disposes partial
state before loading a fresh host. Package-owned effect handlers are installed
inside each fresh startup attempt, so Engram keeps its import/export effects
without replacing generated files. TaskApp's real-runtime widget acceptance
forces failure and drives the generated retry control through recovery.

