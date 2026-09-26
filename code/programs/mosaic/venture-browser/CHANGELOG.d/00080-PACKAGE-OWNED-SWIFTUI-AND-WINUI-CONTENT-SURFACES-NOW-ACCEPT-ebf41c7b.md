- Package-owned SwiftUI and WinUI content surfaces now accept native line,
  page, start/end, and platform history shortcuts. Both adapters translate key
  input into shared Rust scroll commands and existing Mosaic Back/Forward
  events instead of owning navigation or scrolling semantics themselves.
