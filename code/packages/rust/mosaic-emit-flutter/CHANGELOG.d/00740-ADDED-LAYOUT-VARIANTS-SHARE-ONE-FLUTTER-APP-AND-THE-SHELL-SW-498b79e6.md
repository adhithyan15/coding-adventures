### Added — layout variants share one Flutter app, and the shell switches between them (UI48 ENV2/ENV3, §7.9)

- `from_pipeline_variant` emits a layout variant as its own widget class,
  `<Component><Variant>` (`EngramApp.touch.mll` → `class EngramAppTouch`),
  named by the new `variant_widget_name` (`-` and `_` separate words; the
  same rule as Compose and SwiftUI). The file declares none of the
  interface: no `<C>Event` sealed class or `<C>Event<Case>` subclasses. It
  imports the default layout's file for them (`import 'EngramApp.dart';`)
  and takes the default widget's constructor arguments. Its private helpers
  are file-scoped in Dart, so they may repeat. A variant whose widget would
  take one of the default file's public names (`event-tap` → `CardEventTap`)
  is refused, and so is one that would take a public name of the shell's
  own files (`SHELL_RESERVED_NAMES`: `MosaicApp`, `MosaicHost`, ...;
  component `Mosaic` + variant `host`). Before this, a variant file repeated
  the whole event union, so two layouts could never be imported into one app.
- `EmitOptions::layout_variants` (`LayoutChoice { variant, conditions }`, in
  rule order, conditions keyed by `mosaic-app-runtime`'s wire names) makes
  both project shells select their root at run time. `main.dart` imports each
  selectable variant, carries the rules as `mosaicLayoutRules`, and exposes
  `mosaicLayoutVariant(environment)` (first match, else null for the
  default). The root becomes `Builder(builder: _mosaicLayoutRoot)`, which
  reads `MediaQuery`'s size, platform brightness and animation setting,
  reduces them with `MosaicHost.environmentReport`, and switches on the
  selector, so a resize across a threshold swaps the root on that frame. The
  choices are validated before any Dart is written (new
  `ProjectShellError::InvalidLayoutChoice`); each widget may be chosen once,
  so `touch` and `Touch`, or `task-list` and `task_list`, are refused
  together, and none may be a shell name. The sample shell's placeholder
  host gains `environmentReport` when the shell selects.
- With no layout variants every generated file is byte-for-byte unchanged.
- Fixture `fixtures/layout-variants` and its widget test
  `fixtures/layout-variants-test/widget_test.dart`: the generated shell is
  resized across 600 logical pixels and the test asserts the root swaps
  (the §7 resize gate), run by CI's Linux Flutter lane.
- Tests: variant widget names and refusals, a variant reusing the default's
  interface with the same constructor, name collisions with event classes
  and shell names (every public class in `main.dart` pinned as reserved),
  two variant spellings naming one widget,
  both shells' imports, `Builder`, `switch`, rules and selector, rule order
  and unconditional rules, the placeholder host, refused choices, and the
  plain shell equal to the variant shell minus the selector.
