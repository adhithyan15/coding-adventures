### Elixir Windows BUILD-front contract

- Provisioned a pinned Erlang/Elixir toolchain on the explicit Windows 2025 CI
  lane and made affected-package builds verify Elixir changes there instead of
  silently omitting the lane.
- Added a closed, machine-readable unsupported-platform protocol to the Go
  build tool. Reviewed native and Metal exclusions now report stable
  `UNSUPPORTED` reason codes, propagate `DEP-UNSUPPORTED` to dependents, never
  invoke a shell, and never enter the success cache.
- Added a language-neutral fixture, specification, and fail-closed repository
  audit for all 285 Elixir package/program BUILD roots. The audited corpus has
  278 native Windows fronts and seven reviewed platform exceptions.
- Replaced inherited POSIX-only `http1` and `zip` fronts with native CMD-safe
  Windows overrides, and removed temporary Atbash, Scytale, and Vigenere skips
  now that CI runs their real Mix tests.
- Fixed platform-plan change detection so deleting a selected platform override
  schedules the package on that platform and validates its canonical fallback.

