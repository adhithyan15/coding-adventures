# venture-browser-macos

The first runnable native host for Venture, the educational Mosaic-era
browser. It composes:

- `venture-browser-core` for transactional navigation and page loading;
- CoreText for native measurement and shaping;
- AppKit for the native window and `CAMetalLayer`;
- `paint-metal` for presenting the viewport scene.

The crate also builds as `libventure_browser_macos.dylib`. Venture's generated
SwiftUI shell loads that library through its package-owned `MosaicHost` adapter,
so the MIL/MLL/MSL chrome and this Rust navigation/rendering pipeline form one
native app without handwritten AppKit toolbar controls.

Run the first CERN web page:

```bash
cargo run -p venture-browser-macos
```

Or provide another HTTP URL:

```bash
cargo run -p venture-browser-macos -- http://example.com/
```

Exercise the real AppKit + Metal presentation path and close automatically:

```bash
cargo run -p venture-browser-macos -- --smoke-seconds 1 http://info.cern.ch/
```

Mouse-wheel and trackpad input drive the shared clamped viewport and repaint the
translated scene through Metal. Primary-button input now hit-tests links in
viewport coordinates, loads the resolved destination through the transactional
browser session, updates the title, and repaints repeatedly. Native controls,
text input, and richer pointer behavior remain the next integration layer.
Arrow, Page Up/Down, Home/End, and Space key events now drive the same shared
clamped viewport and Metal repaint path. Command-Left/Right reload Back/Forward
history entries through the transactional browser session, update the title,
and repaint the resulting page.
