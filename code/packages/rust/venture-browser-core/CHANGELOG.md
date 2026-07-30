# Changelog

## 0.4.0

- Add transactional `BrowserSession` navigation across page loading, redirect
  history replacement, and viewport updates.
- Add viewport-coordinate link activation and native-control commands for
  Navigate, Back, Forward, Home, and Reload.

## 0.3.0

- Add `BrowserViewport`, binding the current page to clamped scrolling,
  viewport-space link hit-testing, and translated render-scene projection.
- Preserve viewport height and reset scroll when replacing the loaded page.

## 0.2.0

- Add finite, clamped `ScrollState` geometry with resize re-clamping.
- Add scroll-aware link hit-testing.
- Add `scrolled_viewport_scene`, which wraps document instructions in a
  negative-Y translated group on a viewport-sized scene.

## 0.1.0

- Add `NavigationHistory` with Back, Forward, Home, Reload, and redirect
  replacement semantics.
- Add the injectable `BrowserResourceFetcher` boundary and concrete
  `HttpBrowserFetcher` adapter.
- Add `BrowserPagePipeline::load`, composing fetched HTML through
  document-URL-aware parsing, layout, paint, image resolution, and Mosaic
  broken-image fallback.
- Add end-to-end acceptance coverage from a redirected canned page and fetched
  GIF resource to Cairo RGBA pixels.
