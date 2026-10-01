# forme-sandbox-windows

Production Windows process isolation for third-party Forme plugins.

The checked-in native launcher creates a capability-free AppContainer-derived
low-integrity restricted token, applies process mitigations, starts the plugin
suspended, assigns it to a single-process memory/CPU Job Object, restricts
inherited handles to protocol stdio, and monitors the process handle count.
It pins verified entry/schema handles without delete sharing until process exit
and uses a cryptographically random ephemeral AppContainer profile name that is
deleted on every post-creation exit path. Closing the launcher kills the job.
For Node plugins the launcher preserves the already verified, symlink-free main
path, avoiding a broader AppContainer ACL solely for Node's root-to-entry
canonicalization walk.
Host cancellation travels over a private control pipe so routine termination
still passes through Job and profile cleanup instead of killing the supervisor.
A detached profile janitor watches the stable supervisor handle and removes the
profile after abnormal supervisor death.

Use `createWindowsSandboxFactory()` as the `processFactory` passed to
`createPluginHost()`.
