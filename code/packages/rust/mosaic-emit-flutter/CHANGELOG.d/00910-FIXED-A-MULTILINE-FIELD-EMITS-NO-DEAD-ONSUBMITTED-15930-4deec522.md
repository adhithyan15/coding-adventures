### Fixed — a multiline field emits no dead onSubmitted (#15930)

With `textInputAction: TextInputAction.newline` a `TextField` never submits,
so the `onSubmitted` handler a multiline `HostInput` or `Input` carried for
`onCommit` could not fire. It is no longer emitted: Enter is a line break in a
text area on every backend (UI29 §2.1), and the degradation report names an
authored `onCommit` there instead.
