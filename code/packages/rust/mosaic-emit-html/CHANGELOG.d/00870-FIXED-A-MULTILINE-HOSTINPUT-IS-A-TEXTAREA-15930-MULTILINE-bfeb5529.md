- **Fixed: a multiline `HostInput` is a `<textarea>` (#15930).** `multiline:
  true` was read only on the legacy `Input`, so the kernel `HostInput` became a
  one-line `<input>` (the note-type editor's card templates). Both primitives
  now share the text-area lowering, with the value as element content and the
  name, placeholder, `disabled`, `auto-focus` and change/cancel markers intact.
  A multiline field carries no `data-on-commit`: Enter is a line break there
  on every backend (UI29 §2.1).
