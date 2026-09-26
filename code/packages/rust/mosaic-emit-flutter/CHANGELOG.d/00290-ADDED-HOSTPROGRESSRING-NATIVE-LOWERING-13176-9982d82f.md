### Added - `HostProgressRing` native lowering (#13176)

`HostProgressRing` now lowers to Flutter's native
`CircularProgressIndicator` in its determinate form (`value: fraction`)
instead of reporting `primitive.progress-ring-unimplemented`. The same
widget already renders indeterminate for `Icon(glyph: "spinner")`; the
determinate form is the identical widget with an explicit `value:`
argument.

`CircularProgressIndicator` has no size parameters of its own (unlike
WinUI's `ProgressRing`, which has real `Width`/`Height` DependencyProperties)
— it sizes to fill whatever constraints its parent gives it. `emit_host_progress_ring`
wraps it in `SizedBox(width:, height:, child: ...)`, reading the same
`width`/`height` style props every other primitive's part-style lookup
already resolves, and omitting them from the `SizedBox` args entirely
when unset (matching how every other Box-style sizing prop in this
file only emits what's actually styled).

`value` is required and supports the same `Number`/`SlotRef`/`Expr`
three-way binding every other numeric prop has — unlike `Path`'s
geometry props, this needed live binding from day one, since the whole
point is rendering a live `ring-percent-value`. Mosaic's `value` is a
0..100 percent; Flutter's is a 0.0..1.0 fraction, so the generated Dart
divides by 100.0. `a11y-label` wraps the ring in `Semantics(label:
...)`, matching `HostSlider`'s own accessibility pattern.

Narrows `mosaic-package-artifact-builder`'s `HostProgressRing`
degradation arm to also exclude `Backend::Flutter`.

Verified against the real toolchain: a `mosaic-compile pkg --backend
flutter --profile native-complete` build of a `HostProgressRing` bound
to a `ring-percent-value` slot produces `nativeComplete: true` with
zero degradations, and the generated `Ring` widget — both standalone
and mounted with real constructor args inside a `MaterialApp` — passes
`dart analyze` cleanly via the same minimal one-time `pubspec.yaml`
scaffold used for the `Path` Flutter PR.

