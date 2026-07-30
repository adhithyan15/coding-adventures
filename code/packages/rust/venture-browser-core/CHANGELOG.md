# Changelog

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
