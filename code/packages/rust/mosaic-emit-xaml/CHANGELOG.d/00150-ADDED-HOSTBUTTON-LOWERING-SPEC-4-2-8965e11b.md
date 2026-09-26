### Added — `HostButton` lowering (spec §4.2)

- `HostButton` lowers to `<Button>` with:
  - `label: slot: L` → `Content="{x:Bind L}"`
  - `label: "..."` → `Content="..."` literal
  - `disabled: slot: D` → `IsEnabled="{x:Bind Not(D)}"` plus a generated
    `private bool Not(bool b) => !b;` helper added once per component
  - `disabled: true` / `false` keyword → literal `IsEnabled="False"` / `True`
  - `onClick: emit: X` → `Click` handler dispatching `XEvent.{X}()`

