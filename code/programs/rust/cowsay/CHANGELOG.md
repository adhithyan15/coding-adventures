# Changelog

All notable changes to the Rust `cowsay` program are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **macOS build was broken: `TextContent` was missing its `wrap` field.** The
  Apple-only PNG rendering path (`render_cowsay_png_metal`) constructs a
  `layout_ir::TextContent`. When `layout-ir` gained a `wrap: bool` field, every
  other construction site in the repository was updated, but this one was not,
  because it sits inside `#[cfg(target_vendor = "apple")]` and so does not exist
  for the Linux or Windows compilers. The program therefore could not compile at
  all on macOS:

  ```
  error[E0063]: missing field `wrap` in initializer of `layout_ir::TextContent`
     --> src/main.rs:451:37
  ```

  The value is now set explicitly to `wrap: false`. That is the semantically
  correct choice, not merely the one that compiles: cowsay emits fixed-format
  monospace ASCII art whose speech-bubble borders and cow legs align only
  because each line keeps exactly the spaces it was built with. `layout-to-paint`
  implements soft wrapping as a greedy word wrapper that re-splits on
  `split_whitespace()`, collapsing runs of interior spaces — so `wrap: true`
  could silently re-flow the art into gibberish. Hard `\n` breaks, which are the
  only breaks cowsay produces, are preserved either way.

### Changed

- **The PNG scene description is no longer hidden behind a platform gate.**
  Building the `TextContent` and its `FontSpec` moved out of the Apple-only
  function into two un-gated helpers, `cowsay_text_content` and `png_font`. Only
  the *rendering* genuinely needs Metal and CoreText; *describing* the scene is
  plain data that compiles everywhere.

  This is the durable fix for the class of bug above. Previously the Ubuntu and
  Windows CI legs reported green on a program that could not compile, and the
  breakage surfaced only on a macOS runner long afterwards. Now all three legs
  type-check the struct literal, so the next field added to `TextContent` fails
  within minutes on every platform instead of rotting undetected. `dead_code` is
  allowed on non-Apple targets — the binary really does not call these helpers
  there — but a lint allowance does not suppress type checking, which is the
  whole point.

### Added

- **First test suite for this package** (6 tests), running on Linux, Windows and
  macOS alike. Includes an explicit regression test, `ascii_art_never_soft_wraps`,
  which pins `wrap == false` and `max_lines == None`, plus coverage of byte-exact
  art preservation, font selection and line height, text alignment and colour,
  empty input, and the non-Apple `render_cowsay_png_metal` stub contract.

### Notes

- This package previously had no `BUILD` file, which is why the defect went
  unnoticed: the build tool discovers packages by their `BUILD` file, so a crate
  without one is never built and its tests never run. Adding that file is handled
  separately by the in-flight PR #12153; this changelog entry records the
  dependency so the two are not confused. Until that lands, cowsay's only CI
  coverage comes from the affected-package graph of the shared `layout-ir` crate.
