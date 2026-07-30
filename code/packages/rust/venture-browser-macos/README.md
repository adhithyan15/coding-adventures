# venture-browser-macos

The first runnable native host for Venture, the educational Mosaic-era
browser. It composes:

- `venture-browser-core` for transactional navigation and page loading;
- CoreText for native measurement and shaping;
- AppKit for the native window and `CAMetalLayer`;
- `paint-metal` for presenting the viewport scene.

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

This first host slice loads and presents one page. Native controls, input-event
translation, scrolling, and link activation are the next integration layer.
