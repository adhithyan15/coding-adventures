### Fixed - preserve HostInput accessible names (#13717)

Flutter text fields now retain `HostInput.a11y-label` through a native
`Semantics` wrapper with the editable-text role.

