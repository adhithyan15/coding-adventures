### Changed — Android's installMosaicPlatformEffects takes the app's kinds (UI89 §3.12)

- `installMosaicPlatformEffects(host, picker, appKinds)` on Android gains
  `appKinds`, defaulting to `null`. It wraps whatever handler the app
  installed first, the package's `compose-android` handler, and routes the
  kinds that handler claims to it, as the desktop library does.
