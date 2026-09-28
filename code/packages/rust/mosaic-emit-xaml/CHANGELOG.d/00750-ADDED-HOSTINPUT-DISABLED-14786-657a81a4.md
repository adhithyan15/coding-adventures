### Added — `HostInput.disabled` (#14786)

`HostInput` had `read-only` but no way to say *unavailable*. WinUI spells
availability positively, so `disabled` binds through the `Not` converter
function as `IsEnabled`, `Mode=OneWay` like every other emitted binding.

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

