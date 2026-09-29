### Fixed - `--emit-project` Flutter shell supplies constructor inputs

`lib/main.dart` now mounts the generated widget with deterministic sample
values for every declared slot plus a dispatch callback. Previously the
Flutter shell emitted `{Component}()` even though generated widgets always
require `dispatch` and may require slot constructor arguments.

