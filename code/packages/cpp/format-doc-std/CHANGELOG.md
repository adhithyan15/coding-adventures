# Changelog

All notable changes to the C++ `format-doc-std` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `format-doc-std` crate
  (namespace `ca::format_doc_std`) — reusable pretty-printing templates over
  the header-only `cpp/format-doc`.
- Four templates: `delimited_list` / `delimited_list_with`, `call_like`,
  `block_like` / `block_like_with`, and `infix_chain`, plus the
  `TrailingSeparator` policy and per-template config structs holding
  cheaply-copyable `Doc` values.
- Faithful divergence: an infix arity mismatch (Rust panics) throws
  `std::invalid_argument`.
- 27 checks mirroring the Rust crate's own unit tests (laid out and rendered
  through `format-doc`), run under every available C++ compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer.
