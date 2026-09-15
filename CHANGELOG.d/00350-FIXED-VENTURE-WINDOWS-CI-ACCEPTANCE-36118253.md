### Fixed — Venture Windows CI Acceptance
- The pull-request Windows runner now derives a dedicated Venture acceptance
  flag from the shared build plan, installs MSVC and .NET only when that slice
  is affected, and executes the package-owned Rust/WinUI integration test
  instead of reporting a green job whose general build step was skipped. A
  focused detector test ratchets force, package, unrelated, and malformed plans.

