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

`NavigationHistory` is re-exported from the reusable `browser-navigation`
package and implements the BR01 in-memory navigation model: navigate, Back,
Forward, Home, Reload, and redirect replacement. The same package owns
`VisitedLinks`, whose canonical document identity normalizes schemes, hosts,
default ports, percent escapes, dot segments, and fragments independently of
the page pipeline.

Bookmarks are re-exported from the storage-neutral `browser-bookmarks`
package. `BrowserSession` owns the active catalog and applies add/remove
commands with save-before-commit transaction semantics, while native hosts
inject either the durable `browser-bookmarks-file` adapter or an isolated
in-memory repository for tests. Bookmark URL identity deliberately retains
fragments so separate document anchors can be saved independently.

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
follows the resolved link through the same transactional path. Successful
final URLs are committed to session visited state only after full composition;
failed loads preserve history, viewport, and visited state together. Reflow,
reload, Back, and Forward all project the retained state into blue/purple link
styling without coupling browser history to HTML layout.

`BrowserChromeController` is the matching host-neutral reducer for the shared
Mosaic `VentureChrome` package. It preserves address edits as a draft, maps
navigation, bookmark, and View Source events to host-neutral commands, synchronizes redirects
only after a successful load, and projects one coherent `BrowserChromeProps`
snapshot for the nine MIL slots. `BrowserAuxiliaryDocument::view_source`
escapes the already-retained response text into synthetic preformatted HTML,
and `BrowserHostEventOutcome` carries the resulting platform-owned window
effect without navigation, history mutation, or a network fetch. Generated Mosaic shells expose the native
node seam on all registered backends. The SwiftUI adapter connects this reducer
to the live Metal renderer, WinUI mounts Direct2D pixels, and Qt, Flutter, and
Compose share the Cairo bridge. Platform-native integration gates exercise the
generated shells through those adapters rather than reimplementing browser
state in each toolkit.

## Development

```bash
cargo test -p venture-browser-core -- --nocapture
```
