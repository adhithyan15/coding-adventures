### Fixed - preserve dynamic HostButton accessible names (#13754)

Flutter buttons now wrap authored accessible names in native `Semantics`,
including repeated-row expressions, without changing their visible labels.

