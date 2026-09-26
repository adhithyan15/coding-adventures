### Added - native HostSlider

`HostSlider` now lowers to Flutter Material's native adjustable `Slider`, with
controlled value/range, discrete or continuous steps, disabled state,
per-tick `onChange`, and release-value `onCommit`. A strict fixture must remain
native-complete, analyzer-clean, and widget-tested in CI; its test also proves
disabled sliders expose no interactive callbacks.

