# Native table keyboard selection and focus

Related: #14278, #14276, epic #14267. Baseline bbac63d317.

The shared XAML table cell currently moves focus with arrow keys, but the target
cell does not dispatch its authored navigation event. Rust-owned selection can
therefore disagree with focus. The previous native audit also saw a Tab probe
retain formula-field focus; repeat with a fresh post-action snapshot before
classifying that as a separate defect.

Implement an explicit reusable bridge between native cell keyboard activation
and the same authored event/payload used by pointer navigation. Never infer
spreadsheet coordinates from labels, prepend a row-header column to logical
coordinates, or hard-code VisiCalc event names in the emitter. Ambiguous event
sources must not silently pick an arbitrary descendant. Resolve payloads from
the current row VM at activation time, not a stale realization-time closure.

Keep text editing isolated: arrows inside an editor move the caret, Enter/F2
editing and Escape cancellation preserve their current contracts. After adapter
updates replace row VMs, restore keyboard focus to the corresponding realized
cell without stealing focus from an active editor. Retain noninteractive native
table focus navigation for consumers without an authored activation callback.

Validation must cover generation, safe payload lowering, no-event consumers and
nested input guards, then a fresh generated Windows build and actual Tab/arrow
interaction. Verify cell highlight, selection summary and subsequent formula
commit refer to the same logical cell. Record any limits of UIA inspection and
keep full accessibility acceptance open until verified.

## Implementation and validation, 2026-10-05

The native wrapper exposes a keyboard navigation event wired to the unique
authored container tap action. Its handler reads the wrapper's current typed
row VM Tag using the same lowered payload expressions as pointer dispatch.
Multiple candidate actions produce a compiler error. Arrow handling checks that
the wrapper itself owns focus and reacquires the target after adapter updates.

368 emitter unit/integration tests passed; one doctest is ignored. The generated
VisiCalc x64 WinUI project built with zero errors and 20 binding warnings.
Security review passed. Standard Rust runtime DLL and isolated fixture retained
at C:/Users/adhit/worktrees/visicalc-keyboard-20261005-output/.

Native keyboard acceptance is NOT complete: the app launched and UIA reported
the restored B2 formula state, but activation failed and the captured screen
showed the Windows lock screen. Do not treat the background UIA tree as keyboard
interaction proof. After the desktop is unlocked, verify Tab into the grid,
arrow selection/highlight/summary synchronization, repeated navigation after
row VM replacement, editor caret behavior and a formula commit to the selected
cell. Keep the PR draft until that validation passes.
