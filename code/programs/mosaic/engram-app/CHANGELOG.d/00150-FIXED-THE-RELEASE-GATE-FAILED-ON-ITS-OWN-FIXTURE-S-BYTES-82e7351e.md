### Fixed - the release gate failed on its own fixture's bytes

`Release Engram`'s "Validate release identity" job has failed on every branch
since the `[host_effects]` migration (#15106), and it gates the whole workflow:
every build job downstream reports `skipped`, so no Engram release of any
backend has been publishable. Two `ArchiveFlutterTests` cases died in
`_reject_thin_macos_flutter_engine`, reporting a runtime that "begins
`b'\xcf\xfa\xed\xfe'`, not a fat Mach-O".

Those are the FIXTURE's bytes. `_write_flutter_bundle` hand-writes a thin
Mach-O and no test in that class runs `flutter build`. The check is gated on
the engine STEM, so it lay dormant over these bundles until #15106 renamed the
stem to `mosaic_app` - and the day it began to apply, it judged a stand-in that
had been thin since the day it was written.

The fixture now writes a universal container, which is what the release
actually leaves behind: `flutter build macos --release` has no
`--target-platform`, so it builds arm64 and x86_64 and `lipo`s them (#15128).

**This does not make any macOS artifact universal, and is not evidence that one
is.** #15106's own commit message parks a real defect on that path - the
native-assets hook "hands lipo two arm64 copies of the runtime library under
two framework names" - and nothing here touches it. What this does is let the
workflow run past its gate, so the build job's `archive-flutter` step can apply
the same check to a REAL bundle. Until now the pipeline never reached that
step, so the guard had no verdict on production at all.

