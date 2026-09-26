### Known limitations carried forward to later PRs

- **`HostTable`** + section sub-tags still `UnsupportedPrimitive`
  pending PR-4.
- **Component references** still `UnsupportedPrimitive` pending PR-5.
- **`BoolToVisibilityConverter` C# class** still references-only;
  hosts need to ship one. A follow-up emits the converter alongside
  the rest.
- **HostInput event payload** captures the *raw* `tb.Text` of the
  `TextBox` at dispatch time. A future PR may switch to two-way
  bindings for the slot in addition to the dispatch (mirroring
  the React emitter's `e.target.value` pattern).
- **HostButton accelerator-key wiring** (e.g. `accelerator: "Ctrl+S"`
  → `KeyboardAccelerator`) is out of scope for PR-3.

## [Unreleased] — PR-2 — If / Else / For + ExprLowerer

