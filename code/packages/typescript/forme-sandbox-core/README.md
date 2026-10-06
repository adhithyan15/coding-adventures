# forme-sandbox-core

Shared fail-closed launch mechanics for the Forme operating-system sandbox
packages defined by FM02.

The package does not claim to sandbox a process by itself. It verifies and
exclusively stages the exact manifest, entry, and schema snapshots accepted by
the plugin host in a host-owned snapshot subdirectory, selects only trusted
absolute runtime distribution roots, constructs a
minimal environment, starts a platform-owned native helper, and accepts the
process only after a bounded readiness attestation arrives on a private file
descriptor. On Windows, that environment includes only the validated
`SystemRoot` and `LOCALAPPDATA` bootstrap paths required for AppContainer
construction in addition to scratch-directory values; Windows rewrites the
profile path before plugin code runs.

On POSIX, normal termination targets the isolated process group. Forced
termination uses a reserved control signal so the trusted native supervisor
kills and reaps its owned child without retaining a reusable bare plugin PID.

Production callers use one of `forme-sandbox-linux`, `forme-sandbox-macos`, or
`forme-sandbox-windows`. A missing helper, invalid resource limit, unsupported
runtime, malformed readiness record, or identity mismatch fails the launch;
there is no unsandboxed fallback.
When a manifest omits resource overrides, the host applies FM02's bounded
defaults, including a 256-descriptor budget that accommodates normal managed
runtime startup while remaining enforced by the platform launcher.

`SandboxProcessFactory` directly extends `forme-plugin-host`'s
`PluginProcessFactory`, so each OS factory is compile-time checked against the
production host boundary rather than a duplicate local shape.
