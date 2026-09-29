### Added - Native activation for MSL hover states

WinUI output now activates UI15's built-in `state hover` blocks on Mosaic
controls that lower to the native ButtonBase family: `HostButton`,
`HostCheckbox`, `HostRadio`, and `HostLink`. The generated `StateTrigger`
binds directly to the control's native `IsPointerOver` dependency property.
Bindings inside a `For` remain in the DataTemplate namescope, so each repeated
row owns independent pointer state. Existing explicit `state-when-hover`
predicates remain author-controlled and do not install pointer tracking.

