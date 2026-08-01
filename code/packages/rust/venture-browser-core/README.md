# venture-browser-core

The host-neutral orchestration layer for Venture, the educational Mosaic-era
browser described by BR01.

```text
requested URL
  -> BrowserResourceFetcher
  -> final fetched URL + HTML bytes
  -> html-parser
  -> html-to-paint
  -> synchronous GIF/JPEG resource resolution
  -> BrowserPage { document, source, links, scene, image_failures }
```

`BrowserPagePipeline::load` uses the final fetched document URL as the base for
relative links and images. Image failures are recoverable: successful images
remain decoded while failures become Mosaic-style bordered `alt` text.

The default `HttpBrowserFetcher` adapts `http1-client`, but tests and platform
hosts can inject any transport. Font measurement, shaping, metrics, resolution,
and the final paint backend also remain caller-owned.

`NavigationHistory` implements the BR01 in-memory navigation model: navigate,
Back, Forward, Home, Reload, and redirect replacement.

`ScrollState` clamps vertical offsets against content and viewport geometry,
performs scroll-aware link hit-testing, and feeds `scrolled_viewport_scene`.
That function preserves the document scene beneath a group translated by the
negative scroll offset and sizes the returned scene to the visible viewport.

`BrowserViewport` binds a loaded `BrowserPage` to that scroll policy. Native
content-area hosts can resize or scroll it, hit-test links in viewport
coordinates, and request the exact viewport scene for each paint event. Page
replacement resets scroll while preserving the current viewport height.

`BrowserSession` is the reducer a native shell keeps for browser behavior. It
dispatches Navigate, Back, Forward, Home, and Reload through the page pipeline,
replaces redirect history with the final URL, and updates the viewport only
after a successful load. Pointer activation uses viewport coordinates and
follows the resolved link through the same transactional path.

`BrowserChromeController` is the matching host-neutral reducer for the shared
Mosaic `VentureChrome` package. It preserves address edits as a draft, maps the
six MIL events to `BrowserNavigation`, synchronizes redirects only after a
successful load, and projects one coherent `BrowserChromeProps` snapshot for
the six MIL slots. Generated Mosaic shells now expose the native node seam on
all registered backends. The first concrete adapter connects this reducer and
the live Metal page renderer to the generated SwiftUI shell; Windows and the
remaining native hosts still require equivalent direct acceptance.

## Development

```bash
cargo test -p venture-browser-core -- --nocapture
```
