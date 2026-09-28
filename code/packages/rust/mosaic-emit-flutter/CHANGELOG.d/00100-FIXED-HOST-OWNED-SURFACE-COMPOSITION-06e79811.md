### Fixed - host-owned surface composition

`HostSurface ( content: slot: ... )` now mounts the supplied `Widget` instead
of silently emitting the unresolved-component `SizedBox` placeholder.
The package builder mirrors the component into `lib/` so Dart imports stay
inside the package boundary, and the generated README documents the one-time
`flutter create --platforms=... .` runner bootstrap. That exact flow now
produces a native macOS app from Venture's generated browser chrome.

