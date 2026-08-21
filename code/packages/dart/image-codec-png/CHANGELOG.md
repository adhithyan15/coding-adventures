# Changelog

## 0.1.0 - 2026-08-20

- Add the native Dart IC18 PNG encoder, decoder, and `ImageCodec` adapter.
- Consume all 85 language-neutral fixtures and the normative 29-code error
  taxonomy through public APIs.
- Reuse repository PixelContainer and ZIP CRC/raw-RFC-1951 primitives with
  exact inflate limits and byte-consumption checks.
- Add deterministic filter selection, independent encoder-oracle tests,
  focused allocation/precedence regressions, coverage enforcement, BUILD
  metadata, and an empty capability manifest.
