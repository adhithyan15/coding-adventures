### Added — `HostInput` lowering (spec §4.1)

- `HostInput` lowers to `<TextBox>` with the spec's attribute mapping:
  - `value: slot: V` → `Text="{x:Bind V, Mode=TwoWay}"`
  - `value: "..."` → `Text="..."` literal
  - `read-only: slot: R` → `IsReadOnly="{x:Bind R}"`
  - `read-only: true` / `false` keyword → literal `IsReadOnly="True"` / `False`
  - `placeholder: "..."` → `PlaceholderText="..."`
  - `max-length: N` → `MaxLength="N"` (integer-cast from the f64 prop)
  - `multiline: true` → adds `AcceptsReturn="True" TextWrapping="Wrap"`
- Event wiring lands as private code-behind handlers:
  - `onChange: emit: X` → `TextChanged` handler dispatching
    `XEvent.{X}(textbox.Text)` (payload-carrying)
  - `onCommit: emit: X` + `onCancel: emit: Y` → merged `KeyDown`
    handler keyed on `VirtualKey.Enter` / `VirtualKey.Escape`
  - `onFocus: emit: X` → `GotFocus` handler

