# Changelog

## Unreleased

- Add a Cairo-backed native page bridge for Mosaic's generated Qt Quick host.
- Reuse `venture-browser-core::BrowserHostController` for navigation, chrome
  projection, scrolling, link activation, hover, and retained-page reflow.
- Add a generated-project direct-launch test that requires a live HTTP fetch,
  mounted QML surface, and non-empty Cairo frame on provisioned Qt hosts.
- Promote the generated-project gate to real address, history, wheel, hover,
  and link interaction acceptance through the package-owned Qt adapter.
- Accept either CMake's plain macOS executable or an application-bundle shell
  when launching the generated project.
