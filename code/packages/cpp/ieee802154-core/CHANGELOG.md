# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `ieee802154-core` crate in
  namespace `ca::ieee802154_core`: a dependency-free parser/encoder for IEEE
  802.15.4 MAC frames.
- `FrameControl::parse/encode`; `MacFrame::parse_without_fcs`/`parse_with_fcs`/
  `encode`/`summary` (throwing `MacError` where the Rust returns `Result`),
  including the auxiliary security header (32/40-bit frame counter, four
  key-identifier modes); `BeaconPayload::parse` and
  `PanDescriptor::from_beacon_frame` (throwing `BeaconError`); `PanScanSummary`
  filtering/ranking; and the superframe / security-level accessors.
- `std::optional` for optional fields, `std::vector` for payloads/addresses,
  `std::array` for the key source — RAII throughout, every read bounds-checked.
  Verified clean under ASan + UBSan.
- 82 checks mirroring the crate's unit tests run under every ISO C++ compiler via
  the shared `iso-harness`.
