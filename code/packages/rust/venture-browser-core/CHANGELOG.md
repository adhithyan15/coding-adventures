# Changelog

## Unreleased

- Add target-neutral `BrowserScrollMetrics` and shared-session absolute offset
  control for generated native scrollbar projection.
- Add non-mutating shared-session link hover lookup for native status and
  cursor projection.
- Add retained-document reflow so native surface resizes recompute layout,
  paint, links, image placement, and scroll bounds without refetching HTML.
- Add host-neutral keyboard-scroll commands so native SwiftUI and WinUI page
  surfaces share exact line, page, start, and end scrolling behavior.

## 0.6.0

- Pin the host-owned `content-surface` node slot used to mount native page
  renderers inside the Mosaic-authored Venture chrome.

## 0.5.0

- Add the host-neutral `BrowserChromeController` reducer and
  `BrowserChromeProps` projection for the Mosaic-authored Venture chrome.
- Pin the exact shared MIL slot and event names so host wiring cannot silently
  drift from the package contract.

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
