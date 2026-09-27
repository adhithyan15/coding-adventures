### Fixed — the Qt release artifact shipped a library the app never opens

Qt migrated off `engram-capi` in #13728, so the generated `MosaicHost.cpp` opens
`libmosaic_app` and resolves `mosaic_app_create` / `mosaic_app_dispatch`. Three
separate places were still written for the retired architecture, and each one
blessed the next:

1. **`build-native.sh` emitted Qt without the runtime flags.** Only SwiftUI got
   `--profile native-complete --runtime-library`. Measured on a real run before
   fixing: the Qt emission reported `nativeComplete: false` with a
   `runtime.sample-fallback` degradation — the emitter saying, in its own words,
   that the app has no engine.
2. **It then placed `libengram_capi.dylib` beside the project**, and copied that
   into `Contents/MacOS` of the `.app`. So the bundle carried a library nothing
   loads, and no `libmosaic_app` at all.
3. **`archive_qt` verified the presence of `engram_capi`** and passed it. The
   release check was aligned with the bug rather than with the app.

Meanwhile CI emits Qt *with* both flags, so **the artifact being verified was
not the artifact being shipped** — which is precisely what the release epic's
"claiming only artifacts that were actually verified" line exists to prevent.

Fixed in all three, and verified end to end against a real Qt build rather than
by reasoning:

- `STANDARD_RUNTIME_BACKENDS` says which hosts use the standard runtime, and is
  **derived from the manifest rather than written down**. A backend has
  migrated exactly when Engram stops overriding its generated host, so the
  `[host_assets]` entry coming off *is* the migration — the manifest already
  knows, and asking it means the list cannot drift from what it describes.

  The first version was the literal `" qt swiftui "` with a comment saying
  Compose would join "with its own migration". That is an unpaid promise, and
  the day the override came off this script would have kept emitting Compose
  for the retired architecture and bundling `engram-capi` beside a host that
  opens `libmosaic_app` — this same bug, shipped again on the next backend.

  Proven rather than asserted: removing the Compose `[host_assets]` line the
  way #15057 does moves the derived list from `qt swiftui` to
  `qt swiftui compose` with no edit to the script.

  Parsed with `tomllib`, not grepped. `[host_effects]` and
  `[host_assets].dependencies` carry their own `backend = ` lines, so a regex
  over the file sweeps up Qt — which has effect handlers and no asset override
  — and concludes it still needs `engram-capi`. Wrong in the silent direction.
- Emission now reports `nativeComplete: true`, zero degradations, empty
  `replacedGeneratedFiles`. The app builds and links, and the bundle carries
  `libmosaic_app.dylib` beside the executable, exporting the six `mosaic_app_*`
  symbols the host resolves.
- The post-build check and the `.app` copy name the engine the host opens. The
  first attempt used the *cargo artifact* name, `libengram_mosaic_app.dylib`,
  and failed on a correct build — `--runtime-library` installs it under the
  ABI's conventional `libmosaic_app.dylib`, and the source name is never the
  name on disk.
- `archive_qt` checks `mosaic_app`, and `_find_engine` takes which engine to
  look for rather than hard-coding one. Falsified rather than assumed: a bundle
  carrying only the retired `libengram_capi.dylib` is now refused, and running
  the real archiver over the real built bundle produces
  `engram-qt-macos-v0.3.0.zip`.

- **`archive_compose` was the same trap, armed.** It requires an `engram_capi`
  engine, and #15057 stops Compose shipping one. That workflow runs on any pull
  request touching `engram-app/**`, so the break was not hypothetical or
  distant — it was one merge away, on a PR already open.

  It now derives the expected engine from the manifest through
  `_engine_stem_for`, so it follows the migration instead of a constant. Proven
  in both directions against the real function: today a distribution carrying
  `libengram_capi` is accepted and one carrying `libmosaic_app` refused; with
  the Compose `[host_assets]` line removed the way #15057 removes it, that
  inverts exactly.

  The accompanying test pins only the **stable** ends — Qt, which cannot
  un-migrate, and Flutter, which is not migrating. Compose is deliberately left
  unpinned: asserting its current answer would make the test a tripwire that
  fails the very PR completing the migration.

- **Flutter's and XAML's checks derive their engine too**, though neither has
  migrated and both are correct today. That is the point: the failure being
  fixed here is that Qt's migration *did not touch this file*, so "the
  migration will update it" is precisely the assumption that already failed
  once. Leaving two known future traps while holding the mechanism that closes
  them would be the same partial wiring in a new place.

  Proven across the whole family by removing each `[host_assets]` override in
  turn: every backend flips from `engram_capi` to `mosaic_app` exactly when its
  own override comes off, one at a time, with no edit to the release script.
  Their error messages name the derived engine rather than a hardcoded one, so
  a future failure reads correctly instead of naming a library that is no
  longer involved.

Flutter and XAML are untouched and still correctly expect `engram_capi`,
confirmed by re-running the script for Flutter.

Also corrected while in the file: `--help` claimed that "backends other than qt
emit and place the engine, but their compile step is not wired yet". All five
have had a toolchain arm for some time, and the Engram release workflow builds
every one of them with `--build` — so a reader trusting that text would have
concluded the release path could not work.

