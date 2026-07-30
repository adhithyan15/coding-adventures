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

## Development

```bash
cargo test -p venture-browser-core -- --nocapture
```
