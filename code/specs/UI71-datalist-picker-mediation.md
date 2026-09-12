# UI71: Datalist Suggestions And Picker Mediation

## Status

Implemented for Venture's shared native/web control pipeline.

## Option Contract

The HTML parser resolves each control's `list` reference and retains datalist
option value, label, fallback text, disabled state, and document order on the
browser content and render trees. Missing references remain empty and never
require a host-side DOM lookup.

`browser-form-controls` owns filtering and normalization. Queries are bounded
by bytes, source options, and result count. Matching is Unicode-lowercase
containment over normalized value, label, and fallback text. Disabled,
duplicate, malformed, and constraint-invalid typed candidates are omitted.
Diagnostics report truncation without sending the unfiltered source list to a
host.

## Interaction Contract

Arrow Up and Arrow Down open or move the active option, Escape cancels, and
Enter commits without triggering implicit form submission. Accessibility
adapters use equivalent show, move, commit, and dismiss actions. Commit enters
edit history, marks the value dirty, clears validation presentation, and emits
`input` followed by `change` as one suggestion-picker transaction.

Native bridges expose only a bounded JSON state projection and semantic action
names. SwiftUI/AppKit, WinUI, Qt, Flutter, and Compose may choose presentation,
but they do not own filtering, value validity, active-option policy, mutation
ordering, or form submission behavior.
