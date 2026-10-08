# Native table pointer focus after authored activation

## Problem

PR #16750 connects focused native cells to authored keyboard activation, but
clicking an authored cell container only dispatches selection. In the generated
VisiCalc candidate, clicking G6 selected/highlighted G6 while a following Right
key did not move selection; UIA continued to report the formula field. This is
an acceptance gap under #14278, not proof that keyboard navigation is complete.

## Contract

For the unique authored container tap inside a native data cell, capture the
owning table and logical cell coordinates before dispatch. After dispatch,
queue focus to the current realization of that cell. Do not retain a discarded
row VM or cell control after adapter updates. Preserve the typed payload and
dispatch exactly once. Non-table container taps retain their existing behavior.
The focus callback must not dispatch selection again or intercept editor keys.

## Validation

Use emitter regressions for scoped wiring, dispatch order, and coordinate-based
reacquisition; run the emitter suite and build a newly generated x64 WinUI host.
In the actual app, click a non-editor cell, press Right/Down repeatedly and
verify adjacent selected coordinates/highlights after each adapter update.
Check formula-editor caret movement and committing to the selected cell.
Keep the PR draft if actual interaction remains unverified. Tab entry, F2,
screen-reader announcements and native release acceptance remain in #14278.
