### Fixed — test scratch directories never collide

Tests named their scratch directories by process id and `SystemTime` nanos.
macOS reports `SystemTime` in whole microseconds, so two tests running in
parallel threads could read the same value and share one directory, each seeing
the other's files: `mosaic-compile`'s `compose_fixtures` failed about one run
in five. Every such name now comes from `unique_nonce()`, which adds a
per-process atomic counter to the clock. Seven effect-harness tests.
