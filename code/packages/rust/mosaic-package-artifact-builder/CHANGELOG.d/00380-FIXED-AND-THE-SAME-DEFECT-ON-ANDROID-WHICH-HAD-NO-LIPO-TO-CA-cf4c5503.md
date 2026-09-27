### Fixed — and the same defect on Android, which had no `lipo` to catch it

The `.so` hook serves **Android** as well as Linux, and an Android build is
multi-ABI exactly as a macOS release is multi-architecture: `flutter build apk`
packages `armeabi-v7a`, `arm64-v8a` and `x86_64`, running the hook once for
each.

ELF has no fat format, so there is nothing to slice — and, worse, no aggregation
step to notice. Where `lipo` refuses two slices of one architecture loudly,
Android files the same library into every ABI directory and fails at `dlopen` on
a device whose ABI does not match, or appears to work on the developer's own
device while every other one crashes. **A worse failure mode than the loud one
being fixed above.**

So the ELF hook verifies. `e_machine` is two bytes at offset `0x12`, read in
pure Dart rather than shelled out to `readelf`, which would put a binutils
install on the path of every Linux and Android build. Endianness is read from
`EI_DATA` rather than assumed — the same two bytes give different machine
numbers under each order, so assuming would reject a correct library. Verified
against synthesised headers for all five architectures, both byte orders, a
Mach-O (ignored, not misread) and a two-byte file.

This was caught because the comment justifying the copy-whole branch said
"Linux and Windows ask for one architecture per build" — true of Linux and
Windows, false of Android, which that branch also serves. **And the first
version of the test hardened it in place**, asserting `!hook.contains
("_installSlice")` for the `.so` hook, which reads as "this hook must never
consult the target architecture". A future Android fix would have had to begin
by deleting a test arguing against it. The assertion is scoped to `lipo` now —
the genuinely platform-specific part.

Windows is the one that really does take the file as it is: one architecture per
build, no multi-ABI packaging, and no fat format. A test asserts it gains
neither helper.

The emitted README states the requirement, because it is real and invisible: a
`--debug` build works with a single-architecture library and says nothing, so
the first time anyone meets it is a release build failing.

**This is half the fix.** The emitter can now use a universal library; nothing
yet *builds* one. `build-native.sh` passes whatever `cargo build` produced,
which on an arm64 Mac is arm64-only — so the Engram macOS Flutter release still
fails, now with the message above instead of the `lipo` one. The other half is
filed separately rather than folded in, because it belongs to `engram-app`'s
build script and that file has a migration in flight.

- Pass the layout to the SwiftUI style-drop reporter, so `styleDegradations` stops reporting a `gap` the container applies. SwiftUI takes spacing at view-construction time (`HStack(spacing:)`), which the modifier-chain scan cannot see; 22 of the 40 drops reported for Engram were this, and all 22 were false. Mirrors what the Compose reporter already does for `justify-content`/`align-items` (#14834). A gap a `Box`, `Stack` or `HostScroll` genuinely discards is still reported.
- Recognize validated SwiftUI numeric font-size bindings while retaining explicit degradation for unsupported backends and forms.
- Recognize supported Qt font-size bindings in capability reports. Guard the Unix-only host-asset symlink test so Windows builds can compile the test suite.
- Recognize validated Compose typography projections while continuing to report unsupported backends, primitives and binding forms explicitly.

## Unreleased

- Recognize WebComponent text/control numeric typography; retain the degradation
  diagnostic for table-wide inheritance (#15692).

- Support HTML HostTable numeric typography with scoped native-control inheritance,
  row/container overrides and nested-table boundaries (#15677).

- Recognize HTML numeric typography on text, buttons and inputs; retain
  diagnostics for HTML tables and WebComponent bindings (#15647).

