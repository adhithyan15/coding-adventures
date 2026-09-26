### Added - Native activation for MSL pressed states

WinUI output now connects UI15's built-in `state pressed` blocks on
`HostButton`, `HostCheckbox`, `HostRadio`, and `HostLink` directly to
`ButtonBase.IsPressed`. DataTemplate instances remain row-local, pressed takes
precedence over simultaneous focused or hover states, and explicit
`state-when-pressed` predicates remain author-controlled. A Task App
acceptance gate proves its Mosaic-authored add-task button feedback reaches
generated XAML without handwritten Win32 UI.

