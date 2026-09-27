### Added — the Compose platform library (UI87 §7)

`compose_platform_effects()` returns `MosaicPlatformEffects.kt`, which the
artifact builder writes into every Compose project beside the runtime binding.
It answers the standard effect kinds for every app, so no app carries its own
file-dialog code:

- `files.open` and `files.save` (UI59, UI87 §3.1) through `java.awt.FileDialog`,
  the platform's own panel. A name is returned, never a path. Opening reads at
  most 50 MiB, bounded while reading; saving takes at most 16 MiB, refuses a
  suggested name that is a path, and writes a temporary file beside the
  target before moving it into place.
- `MosaicPlatformRouter` routes each effect by kind: kinds the app claims go to
  its handler, standard kinds to this library, anything else to the app when it
  claimed nothing (the original meaning) or to nobody (the host fails an
  unanswered Await). One file operation at a time. Each standard effect is
  deferred and answered from the AWT event thread; installing twice is a no-op.
- Hardened after security review: a suggested name also may not contain `:`
  (a Windows drive or alternate data stream), control or format characters (a
  right-to-left override can disguise `.exe` as `.pdf`), or end in a dot or
  space, and it must end in an extension of an accepted type. The temporary
  file is created owner-only and given the replaced file's POSIX permissions,
  so saving over a private file never leaves it readable by others.
- `conformance/compose/MosaicPlatformEffectsTest.kt`: 9 tests with fake dialogs
  (routing, save, cancel, path- and disguise-shaped names, mismatched type,
  kept permissions, bad and oversized bytes, open, non-file). CI runs them in
  the Journal Compose lane.

