### Changed — `styleDegradations` now covers SwiftUI as well as XAML (#12022)

The degradation analyzer asks each backend for the style properties its
lowering dropped. Until now only XAML answered, so an empty `styleDegradations`
meant "XAML found nothing" on XAML and "nobody looked" on the other seven.

SwiftUI now answers too. The dispatch is an explicit `match` on the backend
with a commented `_ => {}` arm, so the six that still do not report are visible
in the code rather than implied by an `if backend == Xaml`.

The test asserting that a non-XAML backend reports nothing was inverted rather
than deleted: it now asserts SwiftUI DOES report a `box-shadow` drop, and a
second test keeps the original guarantee for a backend that still has no
reporting (Qt), because conflating "nobody looked" with "nothing was lost" is
what this issue is about. A drop still does not affect `nativeComplete`, and a
test pins that too.

- Report unsupported layout font-size bindings as explicit backend degradations,
  keeping native-complete acceptance honest while React/Electron support lands.

