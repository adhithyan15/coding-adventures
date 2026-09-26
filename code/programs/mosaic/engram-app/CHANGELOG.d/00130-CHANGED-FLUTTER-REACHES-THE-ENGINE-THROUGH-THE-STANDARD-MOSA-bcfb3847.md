### Changed — Flutter reaches the engine through the standard Mosaic runtime

The fourth backend to make this migration, after Qt (#13728), SwiftUI and
Compose. The 730-line `host/flutter/mosaic_host.dart` — a `dart:ffi` binding
that opened the library, marshalled every event, owned snapshot persistence and
drove the file pickers — is replaced by the generated `MosaicHost` plus a
handler that answers two effects and does nothing else.

It emits with **`nativeComplete: true`, zero degradations, and an empty
`replacedGeneratedFiles`**, and the emitted project builds.

#### This one was not a duplicate runtime. It was a broken build.

The other three overrides reimplemented what the generated host already did.
This one also failed to compile. It defined `load()` but not `loadRequired()`,
and the native-complete `main.dart` calls `loadRequired()`:

```
error • The method 'loadRequired' isn't defined for the type 'MosaicHost'.
      • lib/main.dart:9:43 • undefined_method
```

Measured, not inferred: `flutter analyze` on the project emitted from the
pre-migration manifest reports exactly that, and the same command on the
migrated one reports `No issues found!`. The same shape as the Qt host's
missing `registerTypes`/`attach` — and, in a detail worth keeping, the retired
file's own doc comment describes that exact failure mode for a *different*
method while missing this one.

**Nothing caught it because nothing ever looked.** `mosaic/programs/engram-app`
was not in `mosaic_flutter_runtime_ci_acceptance.py`'s acceptance set, so no
build had ever emitted Engram on Flutter with `--profile native-complete`; the
lane was green because it built task-app. The package is now in the set, with a
test that fails without it, and the lane emits Engram, asserts
`nativeComplete`/`degradations`/`replacedGeneratedFiles`, checks the handler is
imported *and* installed after the host assignment, and runs
`flutter analyze` + `flutter build linux`.

#### Deferred, like SwiftUI and Compose — but for a third reason, and nothing to marshal

Those two defer because the host holds a lock across the handler call, and they
marshal because their answer would otherwise land on the wrong thread. Neither
applies here, and the generated host says so itself: a Dart isolate is
single-threaded, so an answer arrives on a later turn of the event loop rather
than concurrently.

The reason here is that `effectHandler` is a **synchronous** callback while
`openFile` and `getSaveLocation` return `Future`s. There is no inline answer to
give. `deferEffect` is not the better of two options; it is the only one that
does not lose the effect.

That makes the ordering load-bearing in a way it is not elsewhere: the sweep
tests `_deferred` immediately after the handler returns, so `deferEffect` has to
happen *before the first `await`*. A reordering that awaited first would compile
and fail every dialog. A test pins the exact line.

#### The anchor question, asked of a fifth engine — and the first one where the previous answer was wrong

The Kotlin and Swift handlers use `\A`/`\z` because `$` in ICU and Java concedes
a trailing line terminator. Carrying that over to Dart would have been silently
wrong in both directions, and this was measured on Dart 3.9.4 rather than
assumed:

- `^...$` **without** `multiLine` refuses a trailing LF, CRLF, CR, NEL, U+2028
  and U+2029 — all six, matching Rust and beating all three of the others. So
  the plain anchors are already correct here.
- With `multiLine: true` it concedes LF, CRLF, CR, U+2028 and U+2029 but **not**
  NEL — a seventh distinct answer across five engines. The pattern is built
  without that flag, deliberately.
- `\A` and `\z` **compile**, which is the trap. Dart's `RegExp` is
  ECMAScript-derived, where both are *identity escapes*: `\A` is a literal `A`
  and `\z` is a literal `z`. The carried-over pattern rejects `"apkg"` and
  `".apkg"`, accepts `"A.apkgz"`, and does not anchor at all —
  `firstMatch("xxAapkgzyy")` returns `Aapkgz`. Every real extension would be
  refused and the payload's list silently ignored forever.

So the absence of `\A`/`\z` is the load-bearing assertion, and it is checked
over code lines only — a first cut scanned the whole file and failed on the
comment that explains the trap.

#### Twenty-two substring assertions retired with the file

They read the `engram-capi` binding — `eg_engram_app_props`,
`ENGRAM_SNAPSHOT_PATH`, `_withHostStatusProps` and the rest. None of that ships
for Flutter now, exactly as with Qt's sixteen, SwiftUI's twenty-six and
Compose's. What replaces them covers the three outcomes, the deferral line, the
bounded read, the zip-signature check, and a gate that the handler dispatches on
*exactly* the two kinds the application mints.

#### Two release-path checks written for the retired architecture

#15089 found three of these for Qt and fixed them. Migrating Flutter exposes the
same shape in two more places, and neither is theoretical — both were run
against a real built bundle.

**`build-native.sh` would have copied `engram-capi` into the bundle.** The
`flutter` arm placed `$LIB_NAME` unconditionally, so a migrated Flutter app
would ship a library nothing loads while the engine it does open sits
elsewhere. The migrated path now *verifies* instead of placing, and the
verification searches two names: measured on a real `flutter build macos`, the
runtime lands as `Contents/Frameworks/mosaic_app.framework/mosaic_app` — a
framework binary with no `lib` prefix and no extension — so a search for
`libmosaic_app.dylib` alone finds nothing on a perfectly good build. Falsified
both ways: an empty bundle and one carrying only the retired `engram_capi` name
are both refused.

`STANDARD_RUNTIME_BACKENDS` needed no edit at all, which is #15089's manifest
derivation paying off exactly as claimed — removing the `[host_assets]` line
moves the derived list from `qt swiftui` to `qt swiftui flutter` on its own.

**`archive_flutter` refused a correct bundle** — the mirror image of the
`archive_qt` bug. That one verified `engram_capi` and *accepted* a bundle the
app could not use; this one looked for a bare `mosaic_app.dylib` and returned
`None` for the framework Flutter actually installs, so a good artifact failed
the release check. `_find_engine` now recognises a binary named exactly the stem
inside the matching `.framework`, and the search recurses **only where the
engine directory is a real subdirectory**.

That scoping is not a detail. The first version recursed unconditionally, and on
Windows — where the engine directory *is* the bundle root — that turned "beside
the executable" into "anywhere in the bundle" and the layout check collapsed. It
was caught by `test_each_platform_rejects_the_others_layout`, which exists for
precisely that, and both halves are now pinned by tests that fail without them.

**Known and untouched:** `flutter build macos --release` fails in the
native-assets hook, which hands `lipo` two arm64 copies of the runtime library
under two framework names. Reproduced identically on the pre-migration tree, so
it predates this change and is filed separately. `--debug` builds fine on both.

