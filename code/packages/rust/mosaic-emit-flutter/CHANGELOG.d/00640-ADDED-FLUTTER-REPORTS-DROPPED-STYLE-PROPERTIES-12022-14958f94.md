### Added — Flutter reports dropped style properties (#12022)

The emitter now records the style keys consumed by each real widget-lowering
path and exposes `dropped_style_properties`. A property is reported when any
layout-node occurrence using its part fails to consume it, so reuse cannot hide
a loss. Unused stylesheet parts remain outside the report.

The recorder is active only during analysis and shares the same style reads as
normal emission; it does not maintain a parallel list that can drift from the
generated Dart.

