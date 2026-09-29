### Fixed - complete TaskApp Dart compilation

Generated widgets now normalize Mosaic value truthiness before values enter
Dart boolean positions, including `If`/`Else`, conditional styles,
`HostButton.disabled`, and `HostInput.read-only`. Native text-input callbacks
also synthesize the declared zero- or one-field event payload for both change
and commit handlers. Together these fixes remove the non-boolean conditions
and missing required constructor argument that prevented the package-expanded
TaskApp from passing the Dart analyzer and building as a native Flutter app.

