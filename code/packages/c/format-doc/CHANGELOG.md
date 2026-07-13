# Changelog

All notable changes to the C `format-doc` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- `fd_clone` (public deep-copy of a document tree) and `fd_is_nil` (is-empty
  predicate), exposed for the `format-doc-std` templates layer.

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `format-doc` crate — a Wadler-style
  document algebra for pretty-printers.
- Document builders: `fd_nil`, `fd_text` (CRLF/CR-normalising, splits embedded
  newlines into hardlines), `fd_concat` (flattens nested concats, drops nils),
  `fd_join`, `fd_group`, `fd_indent`, `fd_line`, `fd_softline`, `fd_hardline`,
  `fd_if_break`, `fd_annotate`, and `fd_free`. Builders take ownership of their
  arguments (returning `NULL` on OOM after freeing them), so documents compose
  bottom-up and free with a single `fd_free`.
- Annotation values (`FdAnnotation`: string / int / bool / null) with
  constructors, `fd_ann_free`, and `fd_ann_equal`.
- Layout: `fd_layout_options_default` (80 / 2 / 1), `fd_layout_doc` (a
  stack-machine interpreter whose `group` flat/broken decision uses an O(work)
  `fits` look-ahead that borrows the parent stack), `fd_layout_free`, and
  `fd_render_text`. Adjacent spans coalesce only when their annotations match;
  `visible_width` counts UTF-8 code points.
- 61 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
