### Fixed — an unresolved component reference returned Ok and emitted a hole (#14892)

Flutter was the only backend of the eight that accepted a component reference it
could not resolve, emitting

```dart
/* TODO: component reference 'Cell' not yet resolved */ const SizedBox.shrink()
```

and returning `Ok`. Every gate downstream of `from_pipeline` then passed: the
build was green, `flutter analyze` was clean, and the component was simply
absent, occupying zero pixels.

That is how `mosaic-pkg-grid` appeared to work on Flutter while six backends
refused it (#14867). Flutter was not the one backend that supported the package;
it was the one that could not report the failure.

The placeholder was deliberate when written — its comment said package
resolution was a follow-up and a labelled placeholder let the file type-check
meanwhile. **That follow-up landed.** UI34's resolver inlines every `pkg::P::C`
node before emit and leaves backends a tree with no qualified tags, so the
reason for the placeholder is gone while the hole it can ship is not.

An unresolved reference now falls through to `UnknownPrimitive`, matching the
other seven backends, all of which were measured rejecting the same input
(#14886).

**Nothing shipping relied on it**, measured rather than assumed: Trestle, Engram
and VisiCalc were each generated on Flutter before the change and emitted
**zero** placeholders, and all three still generate afterwards. No checked-in
generated output contains the string either.

`clean_pascal_case_component_reference_emits_placeholder` was retargeted rather
than deleted — the rule it protects, that a non-kernel PascalCase tag is handled
deliberately rather than falling through by accident, is unchanged. A second
test asserts no successful emit can produce the placeholder by another route.

