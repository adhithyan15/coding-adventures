---
category: Mosaic compiler pipeline
---

# Changing an emitter's output shape breaks release-lane source contracts that no emitter test runs

**What happened (#15434, 2026-09-17).** #15427 changed how Qt, Flutter,
Compose and SwiftUI lower a dynamic `HostButton.a11y-label`: an empty name now
falls back to the visible label. All four emitter suites passed locally,
the emitted TaskApp passed `dart analyze` and `qmllint`, and the PR still
went red on five jobs: the four `Build native payload` release lanes and
`build (macos-latest)`.

The reason was `code/scripts/taskapp_native_control_contract.py`. It greps
the *generated* TaskApp for exact source strings, including the old
`Accessible.name: ( row [ 16 ] )`, `Semantics(label: ( row [ 16 ] ), …`,
`contentDescription = ( row [ 16 ] )` and
`.accessibilityLabel(_mosaicText(( row [ 16 ] )))`. No emitter test runs
that script, so the local suites could not see it.

**Fix.** The four markers were updated to the new form, copied verbatim from a
fresh `mosaic-compile pkg code/programs/mosaic/task-app --backend <b>` emit.
The script was then run against those emits to confirm those markers
were no longer reported missing.

**Do differently.** Before pushing any change to an emitter's output text:

1. `git grep -F '<old emitted fragment>' -- ':!*.rs'` for the exact string you
   changed. Source contracts live in `code/scripts/*contract*.py`, release
   scripts, and app-level `package_compiles.rs` tests.
2. Emit TaskApp and Engram on the affected backends, and run
   `taskapp_native_control_contract.py --backend <b> --generated-dir <dir>/<b>`.
   Only the markers you touched matter locally; the rest need CI's
   `--runtime-library` / `--profile native-complete` flags.
