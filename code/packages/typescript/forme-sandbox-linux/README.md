# forme-sandbox-linux

Production Linux process isolation for third-party Forme plugins.

The checked-in native launcher requires a delegated cgroup-v2 root and must be
started from a writable child of that root. It creates a sibling per-plugin
leaf, restores itself to the original child during cleanup, then enters user,
mount, PID, network, IPC, and UTS namespaces; constructs a private root
containing only the staged snapshot and runtime dependencies; applies memory,
CPU, PID, and descriptor limits; sets `no_new_privs`; and installs a seccomp
filter that denies networking, process creation, tracing, and cross-process
memory access. The filter permits read-only socket metadata inspection for the
already-inherited protocol pipes required by Node/libuv, while socket creation
and network I/O remain denied. It fails closed if any primitive is unavailable.

Use `createLinuxSandboxFactory()` as the `processFactory` passed to
`createPluginHost()`.
