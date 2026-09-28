### Fixed — the macOS Flutter release artifact could not be built at all

The last hard blocker on `engram-v0.3.0`, and it fails closed, which is why it
blocks rather than degrades: the publish job `needs:` every build job **and**
asserts the artifact set on disk exactly equals what `artifact-names` declares.
`engram-flutter-macos-v0.3.0.zip` is in that declared set, so a release cannot
be cut while it cannot be produced.

`flutter build macos --release` has no `--target-platform`. It always builds
arm64 **and** x86_64 — running the native-assets hook once per architecture and
`lipo`ing the results into a universal binary — and `build-native.sh` handed it
whatever a plain `cargo build` produced, which on an arm64 Mac is arm64-only.

Two halves, and this is the second. The first is the hook learning to slice per
architecture; without it a universal library still fails. Together they were
proven end to end before either was written: a universal library plus a sliced
hook builds `mosaic_task_app.app` with an `x86_64 arm64` framework, where the
unsliced hook fails with `lipo: … have the same architectures`.

**Keyed on the build's behaviour, not on the backend being Flutter.** The
condition is "does the emitted build ask for more than one architecture", which
today only Flutter on macOS does — stated that way so whoever adds the next
multi-architecture target knows what to look for.

**Qt and SwiftUI on macOS are deliberately excluded, not overlooked.**
`swift build -c release` and the generated CMake both build for the host only,
so a fat library there is bytes in the bundle that nothing loads. Their macOS
artifacts are host-architecture; whether Engram should ship universal apps on
those backends is a separate question, and not one to answer by accident here.

The `lipo -create` output is **asserted** to carry both slices rather than
assumed. Over two copies of one architecture `lipo` fails loudly, but over a fat
file and a thin one it succeeds — so the check is on what came out, not on what
went in. It also writes beside the slices rather than over either, so a re-run
cannot lipo a fat file into itself.

**Dormant until the Flutter migration lands**, and verified anyway. Flutter is
not in `STANDARD_RUNTIME_BACKENDS` until its `[host_assets]` override comes off,
so the new block is unreachable on `main` today. Verified by removing that line
the way the migration does: the derived list moves from `qt swiftui` to
`qt swiftui flutter`, the universal path activates, and the emitted project
carries an `x86_64 arm64` runtime with `nativeComplete: true` and zero
degradations.

That is #15089's precedent, which fixed Flutter's and XAML's release checks
"though neither has migrated", on the grounds that holding the mechanism while
leaving the known trap is the same partial wiring that caused the original bug.

The release workflow installs **both** Apple targets on the macOS leg only.
Naming just the non-host one and relying on the runner to supply the other
encodes "macos-latest is arm64" where nothing states it — and the script
requires both, so an Intel runner would fail closed complaining about a target
the workflow was never asked to install. Conditional, because installing an
Apple target on the Linux and Windows legs of the same matrix would be a
download neither will ever use; an empty value is byte-identical to the key
being absent.

**And the shipped artifact is now gated on being universal, rather than
measured by hand.** The `x86_64 arm64` result above was established by running
`lipo -archs` once and writing it down; nothing checked it, and
`LIBRARY_MAGIC["macos"]` *accepts* the fat magic without requiring it — so a
regression to a thin library would have published successfully and crashed on
an Intel Mac.

`archive_flutter` refuses one now, reading the file's own bytes rather than
shelling out to `lipo` so the check works wherever the archiver runs. It tests
`nfat_arch`, not just the magic: a fat container holding **one** architecture is
legal and is exactly the artifact a magic-only check waves through. Gated on the
migrated engine, since a backend still binding `engram-capi` is built host-only
by a path this says nothing about.

Mutation-tested three ways — the check doing nothing fails the three refusal
tests; dropping the `nfat_arch` test fails only the one-architecture case; and
dropping the platform/stem gate starts refusing correct Linux, Windows and
unmigrated artifacts, which is what shows the gate is load-bearing rather than
decoration.

Two further review findings, both acted on. The `rustup target list` check now
runs inside the same `cd "$RUST"` subshell the build uses, so it cannot answer
for a different toolchain than the one that compiles. And the `lipo -create`
abort-on-failure depends on `set -e`, which is now stated where someone moving
this block into a function would read it.

A reviewer also confirmed the architecture assertion empirically rather than by
reasoning, against a real `x86_64 arm64e` binary: the space-padded `case`
pattern correctly rejects `arm64e`, `arm64_32` and `x86_64h`, which is the
substring trap this change could plausibly have had.

