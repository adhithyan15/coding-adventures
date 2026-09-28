### Added — `HostInput.disabled` (#14786)

`HostInput` had `read-only` but no way to say *unavailable*. Flutter spells
availability positively, so `disabled` lowers to a negated `enabled`
argument, distinct from `readOnly`.

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

- Emit one Expanded wrapper for a direct Row input with explicit flex-grow.
  Preserve the declared flex weight without nesting the input's default wrapper,
  which caused Flutter ParentDataWidget assertions in Venture's live shell.

