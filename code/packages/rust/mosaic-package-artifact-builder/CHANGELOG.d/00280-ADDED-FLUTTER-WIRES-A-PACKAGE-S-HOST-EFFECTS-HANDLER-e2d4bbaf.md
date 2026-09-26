### Added — Flutter wires a package's `[host_effects]` handler

The fourth backend, after Qt, SwiftUI and Compose. XAML was the last one left,
and landed in the entry above within this same release.

**Flutter needs an `include`, where Compose and SwiftUI refuse one.** That is a
language difference, not a style choice: Dart resolves nothing across files
without an explicit `import`, so a handler merely copied into `lib/` is present,
compiled and unreachable. Kotlin and Swift make every file in the module
visible, which is why declaring an include *there* would be a field silently
dropped. Qt is the same shape as Flutter for the same reason — C++ has no
cross-file visibility either.

Demonstrated rather than asserted: deleting the generated import from a real
emitted project makes `flutter analyze` report
`The method 'installProbeEffects' isn't defined`. Note that failure is **loud**,
so the refusal is not preventing a silent break — it names the problem at build
time instead of surfacing as a Dart analyser error inside generated code the
author never wrote.

#### The two shapes need different calls, and the first version shipped one

The permissive project declares `late final MosaicHost? _mosaicHost`; the
`require_runtime` project declares it **non-nullable**. Emitting the same
null-guarded call for both produced `unnecessary_null_comparison` in the
required shape — and since the generated `analysis_options.yaml` pulls in
`flutter_lints` while `flutter analyze` defaults to `--fatal-warnings`, that
made the emitted project **fail the very command its own generated README tells
the reader to run**.

Security review caught it, and the miss is instructive: my verification analysed
one emitted project, which was the permissive default, and I generalised. The
test did drive both shapes — but asserted only that the install came after the
host assignment, and ordering is identical in both. An assertion that holds for
the broken shape is not a check on it.

The call is now keyed off the field declaration already present in the file, so
the two stay in step by construction. The test pins the emitted *form* per
variant, and both shapes were re-emitted and analysed: `flutter analyze` reports
no issues for each.

This is the second time in this session the `require_runtime` variant has been
the untested one — the SwiftUI end-to-end test had the same gap.

**No downcast**, unlike SwiftUI and Compose. Flutter's `MosaicHost` carries
`effectHandler` directly rather than hiding it behind an effect-unaware
interface, so there is nothing to cast. The install does go through a **local**:
`_mosaicHost` is a `late final MosaicHost?` field and Dart does not promote
fields to non-null, so `if (_mosaicHost != null) install(_mosaicHost)` would not
compile.

**Anchored on the assignment, not the call.** The two emitted shapes load the
host differently — `require_runtime` uses `MosaicHost.loadRequired()` at `runApp`
and assigns the widget's field directly, the other calls `MosaicHost.load()`
inside `initState` — but both lines begin `_mosaicHost = widget.mosaicHost`.
Anchoring on the whole call would have matched one and silently missed the
other, so a test drives the real generator for both.

#### Verified by analysing the emitted project

A probe package declaring a Flutter handler was emitted end to end, and
`flutter analyze` reports **no issues** across the whole project — `main.dart`
included. That is stronger than the Compose check, which needed a stub interface
because the Compose toolchain was not available here.

#### The classification tripwire fired again

Flipping `Backend::Flutter` to `true` failed the three tests pinning it as
unsupported, sending me back to update them deliberately. Second time it has
worked as intended.

