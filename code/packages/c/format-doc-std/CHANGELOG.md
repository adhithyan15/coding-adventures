# Changelog

All notable changes to the C `format-doc-std` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `format-doc-std` crate — reusable
  pretty-printing templates over `format-doc`. Depends on the sibling
  `c/format-doc` (compiled in via `run.sh`).
- Four templates: `fds_delimited_list` / `fds_delimited_list_with`,
  `fds_call_like`, `fds_block_like` / `fds_block_like_with`, and
  `fds_infix_chain`, plus the `FdsTrailingSeparator` policy and per-template
  config structs. Templates consume their content documents and return an owned
  `FdDoc *`; configs borrow their delimiter documents (cloned as needed).
- Faithful divergence: an infix arity mismatch (Rust panics) frees the arguments
  and returns `NULL`.
- 33 checks mirroring the Rust crate's own unit tests (laid out and rendered
  through `format-doc`), run under every available C compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer.
