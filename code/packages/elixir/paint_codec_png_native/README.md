# paint_codec_png_native

Elixir wrapper over the Rust `paint-codec-png` crate.

It takes a `CodingAdventures.PixelContainer` and returns PNG bytes without
mixing codec logic into the Paint VM bridge.
