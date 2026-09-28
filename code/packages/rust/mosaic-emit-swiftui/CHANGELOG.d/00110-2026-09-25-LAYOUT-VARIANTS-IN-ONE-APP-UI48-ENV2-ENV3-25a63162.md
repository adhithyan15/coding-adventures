## 2026-09-25 (layout variants in one app, UI48 ENV2/ENV3)

- `from_pipeline_variant` emits a layout variant as `<Component><Variant>View`
  (`variant_view_type`: `touch` → `EngramAppTouchView`) without the
  component's event enum or extension, which the default layout declares once.
  Two variants used to declare the same `View` and event type, so an app could
  hold only one. New error `UnsafeVariantName`.
- `EmitOptions::layout_variants` (`LayoutChoice`s, in rule order) makes the
  project shell observe its environment and switch views:
  `MosaicEnvironmentReader` measures the window (size class by UI48's default
  thresholds, the same on iPhone, iPad split view and macOS), reads pointer and
  hover from the platform, orientation from the aspect, and color scheme and
  reduced motion from SwiftUI; `MosaicLayoutSelector` holds the rules. With no
  variants the shell is byte-for-byte unchanged. Variant and condition text is
  re-checked before it is written into Swift.

