# Changelog

## Unreleased

- Add the native Venture session bridge used by the Mosaic-generated WinUI
  shell, including shared chrome props/events, Direct2D pixel rendering,
  scrolling, and link activation.
- Accept the same host-neutral semantic keyboard-scroll commands as the macOS
  bridge for package-owned WinUI content-surface input.
- Reflow the retained page through the shared Rust pipeline when the generated
  WinUI content surface changes logical size.
