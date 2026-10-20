### Fixed — host comments no longer name which backends are migrated

`mosaic-package.toml` said the backends other than Qt "still ship a
hand-written `MosaicHost`", and `scripts/build-native.sh` said "Compose, XAML
and Flutter still ship a hand-written host bound to `engram-capi`". Compose,
Flutter and SwiftUI have since moved to the generated runtime, and only XAML
keeps a `[host_assets]` override. Both comments now describe the rule instead:
a backend with a `MosaicHost` override binds `engram-capi`, and every other
backend uses `libmosaic_app`. They point at `STANDARD_RUNTIME_BACKENDS`, which
reads that rule from the manifest, so the next migration can't make them stale.
