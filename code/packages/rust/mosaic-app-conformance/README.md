# mosaic-app-conformance

Reusable Rust application library for executing Mosaic host bindings against
the real `mosaic-app-capi` ABI. The fixture starts with `count = 0`, accepts an
`increment` semantic event, returns revisioned props, and supports snapshots.

Native backend acceptance harnesses load the built dynamic library through the
same generated binding shipped to applications. This keeps conformance focused
on lifecycle, event sequencing, JSON projection, buffer ownership, and teardown
without introducing an app-specific platform reducer.

The adjacent `package/` directory is a minimal MIL/MLL/MSL UI whose required
`platform` and `status` props come from this Rust engine. Packaging acceptance
uses the pair to prove a strict generated native application can carry and load
its engine without application-owned platform glue or a global library install.
