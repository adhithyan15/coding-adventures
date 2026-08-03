# Changelog

## Unreleased

- Move shared browser/controller/Cairo ownership into the backend-neutral
  `venture-browser-cairo` crate and retain this package as the thin Qt facade.

- Add a Cairo-backed native page bridge for Mosaic's generated Qt Quick host.
- Reuse `venture-browser-core::BrowserHostController` for navigation, chrome
  projection, scrolling, link activation, hover, and retained-page reflow.
- Add a generated-project direct-launch test that requires a live HTTP fetch,
  mounted QML surface, and non-empty Cairo frame on provisioned Qt hosts.
- Promote the generated-project gate to real address, history, wheel, hover,
  and link interaction acceptance through the package-owned Qt adapter.
- Export Flutter-named C ABI wrappers over the same host-neutral controller and
  Cairo renderer so Venture's generated Flutter shell can share the live page
  implementation.
- Export Compose-named C ABI wrappers over that same session, using explicit
  64-bit frame lengths for stable JNA interop on macOS, Windows, and Linux.
- Accept either CMake's plain macOS executable or an application-bundle shell
  when launching the generated project.
