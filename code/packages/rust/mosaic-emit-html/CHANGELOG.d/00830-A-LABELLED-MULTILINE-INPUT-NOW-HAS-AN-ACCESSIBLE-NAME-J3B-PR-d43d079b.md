- A labelled multiline `Input` now has an accessible name (J3b-pre, #14416).
  UI25's legacy `Input` was dispatched to a separate `emit_input` that ignored
  `a11y-label`, `disabled` and `auto-focus`, even though the `HostInput` branch
  already lowered `Input ( multiline : true )` to `<textarea>` with all three.
  The duplicate is removed, and `Input` shares `HostInput`'s lowering.
