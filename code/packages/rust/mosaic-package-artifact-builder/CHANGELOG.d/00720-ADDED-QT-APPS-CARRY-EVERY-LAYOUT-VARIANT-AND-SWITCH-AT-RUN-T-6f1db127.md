### Added — Qt apps carry every layout variant and switch at run time (UI48 §7.10)

- Qt variants compile through `mosaic_emit_qt::pipeline::from_pipeline_variant`,
  so `<C>.<variant>.qml` names its QML type, `<C><Variant>`.
- `qt_layout_choices` uses the same `effective_layout_rules` as SwiftUI,
  Compose and Flutter (`[[app.layouts]]` or the conventions), refuses a rule
  for a missing variant or one that cannot name a QML type, keys conditions by
  `EnvironmentAxis::wire_name`, and records each variant root's native table
  model count; the emitter writes them into `main.cpp`.
- The project shell composes the root's variants as it composes the default,
  and a native-complete shell re-emits them strictly (`required property var
  mosaicHost`, `handleRequiredEvent`), exactly as it re-emits the default: the
  flat artifacts are permissive, and a permissive root would have answered
  with unmapped props.
- `qt_cmake_with_layout_variants` compiles every export's variants into the
  QML module, each as its own type, beside the root's (which the emitter
  already lists), and refuses a type the module would register twice: a
  variant named like another export (`Card` + `touch` beside an exported
  `CardTouch`) or like another variant.
- Tests mirror the other backends' four (convention, declared rules under wire
  names, a rule for a missing variant refused, no variants → no selector),
  plus the native-complete shell mounting its variants strictly, every rule
  axis being a key of the Qt binding's `environmentReport`, every public class
  of the binding's headers being reserved, export collisions refused, and
  every export's variants joining the module.
