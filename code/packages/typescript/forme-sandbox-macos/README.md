# forme-sandbox-macos

Production macOS process isolation for third-party Forme plugins.

The checked-in native launcher starts one child under a deny-default Seatbelt
profile, grants only the staged working tree, exact runtime, and system dynamic
libraries, explicitly denies writes to identity snapshots, denies network and
process creation, applies CPU and descriptor limits, and monitors resident
memory and wall time from the trusted parent. Readiness is attested only after
Seatbelt is installed, snapshots are re-hashed, and runtime exec commits.
A descriptor-closed trusted `kqueue` watcher kills the isolated plugin session
if the native supervisor exits unexpectedly and retires when the plugin exits.

The factory derives Homebrew formula roots used by the current trusted Node
runtime from Node's loaded-library report. Other explicitly configured
runtimes must provide their dependency roots through `runtimeReadPaths`; the
launcher validates those bounded absolute roots before adding them to the
deny-default profile.

Use `createMacosSandboxFactory()` as the `processFactory` passed to
`createPluginHost()`.
