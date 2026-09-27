### Fixed - Native multiline Input compatibility

The still-supported UI25 `Input` primitive now shares the complete native
`TextBox` lowering, including `AcceptsReturn`, text wrapping, dispatch, and
automation identity. Trestle Notes can therefore compile to WinUI without
losing its multiline body editor.

