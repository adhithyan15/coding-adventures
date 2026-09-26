### Added - the universality check is pinned where the release calls it

`_reject_thin_macos_flutter_engine` had five tests and all five invoked it
directly, so all five still pass with its call site deleted - verified by
deleting it. That is exactly how it sat dormant across two commits while
reading as covered. A new `ArchiveFlutterTests` case drives a thin engine
through `archive_flutter`, the entry point the release step runs, and fails
when the call site is removed.

