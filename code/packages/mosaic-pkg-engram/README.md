# mosaic-pkg-engram

Engram UI components authored as Mosaic source files.

This package is the first pivot point away from target-specific Engram UI code.
The `ReviewCard` component describes a study-session review surface once, then
the Rust Mosaic pipeline can lower it to HTML/React/Electron, SwiftUI macOS/iOS,
XAML, Qt, Compose, and Flutter without forking the application model.

The web app still has a React shell while this package grows. App-owned web
styles now live in Lattice; Mosaic `.msl` styles are the cross-emitter component
style layer.
