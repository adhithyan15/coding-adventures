# Changelog

## 0.1.0 - 2026-08-20

- Add the native Lua IC18 PNG encoder, decoder, and PixelContainer codec adapter.
- Consume all 85 language-neutral cases and the normative 29-code taxonomy.
- Reuse repository ZIP CRC/raw-RFC-1951 primitives with exact inflate limits,
  counted consumption, Adler verification, and APNG precedence.
- Add deterministic filters, LibDeflate and real-image encoder oracles, focused
  resource-boundary regressions, coverage enforcement, build metadata, and an
  empty capability manifest.
- Harden Lua PixelContainer and ZIP byte storage so the normative 32-megapixel
  ceiling does not amplify into multi-gigabyte numeric tables.
