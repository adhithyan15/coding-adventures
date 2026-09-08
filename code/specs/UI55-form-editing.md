# UI55: Browser Form Editing and Feedback

## Status

Implemented for Venture's retained form-control model and available native
host bridges.

## Contract

- Text selections use Unicode scalar indexes with independent anchor and focus
  offsets. All replacement and deletion operations convert those indexes to
  UTF-8 byte boundaries internally and never split a scalar value.
- Left/Right collapse or extend selection. Home/End target line boundaries for
  multiline controls. Backspace/Delete remove the selection or one adjacent
  scalar. Textarea Enter inserts a newline.
- Composition updates retain provisional text without mutating the control
  value. Commit replaces the current selection; cancel discards the provisional
  text. Hosts route normalized text and semantic keys but do not own editing
  policy.
- Password controls retain their real value for editing and submission while
  layout and paint consume only the masked display projection.
- Constraint-validation diagnostics attach to retained editor state. The first
  invalid control receives focus, and render-tree synchronization projects
  `aria-invalid` and a bounded accessible description. Any successful edit
  clears stale validation feedback before reflow.
- macOS `WindowEvent`, SwiftUI, WinUI, and Cairo-compatible host bridges route
  through the same `BrowserSession` methods. FFI key names are the stable
  `ControlKey::name()` values.

## Acceptance

Unit coverage exercises UTF-8 range replacement, scalar deletion, shifted
selection, multiline boundaries, composition commit/cancel, password masking,
first-invalid focus, and idempotent accessibility projection. Core and native
host tests verify retained reflow and common key/text routing.
