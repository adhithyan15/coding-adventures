# @coding-adventures/forme-plugin-host

The FM02/FM-B014 host boundary for third-party Forme stages. It discovers and
validates plugin manifests, resolves `StageRef`s to manifest-authored proxies,
and owns a strict bounded JSON-RPC connection for handshake, announcement,
lifecycle, streaming, diagnostics, cancellation, crash isolation, and mediated
`StageContext` calls.

The package also persists trust and grant decisions through strict bounded
TOML codecs and fail-closed filesystem helpers. Grants bind the exact manifest
hash, so a changed plugin receives no stale authority. Reads reject linked or
oversized files; writes use restrictive same-directory staging and atomic
publication.

The package deliberately does **not** spawn an unsandboxed subprocess. Callers
must inject a `PluginProcessFactory`, and each returned process must attest that
the FM02 isolation boundary was established and that it staged the exact
manifest-plus-entry identity the host verified. Runtime adapters, signed
installation and per-OS sandbox launchers remain FM-B049–FM-B052.

```ts
const host = await createPluginHost({
  roots: [projectPluginRoot, userPluginRoot],
  loadPersistentGrants: true,
  signal: cancellation.signal,
  capabilityApis: { storage: boundedProjectStorage },
  processFactory: sandboxedLauncher,
});

const stage = await host.loadStage({
  kind: "stage-ref",
  packageName: "@example/markdown",
  export: "parse",
}, "parse-posts");
```

`loadPersistentGrants` reads the host-owned `grants.toml` beside each exact
discovery snapshot and is mutually exclusive with injected `grants`. Missing
or stale authority grants nothing; malformed files abort host creation.
An optional `signal` cooperatively cancels bounded discovery, snapshot reads,
and persistent-authority loading before any stage can resolve.

Discovery never launches plugin code. A process begins only when the proxy is
initialized or run. Missing grants, announcement drift, malformed/oversized
frames, denied host APIs, crashes, cancellation timeouts, and missing sandbox
attestation all fail closed.

Discovery roots are host-managed snapshot inputs and must remain quiescent
during `createPluginHost()`. The host rechecks containment and opened-file
identity while reading, then owns private byte snapshots. Concurrent mutation
by another same-user process is outside FM-B014; FM-B015 installation must use
atomic staging and enforce immutable, owned install roots before discovery.
Untrusted config schemas are meta-validated and reject `pattern` so native
backtracking regular expressions cannot consume unbounded trusted-host CPU.
Schema announcement identity is `sha256` over the exact bounded schema file
bytes, so runners do not depend on language-specific JSON object ordering or
serialization.
Schemas are snapshotted during discovery and bound into the stage identity and
launcher attestation. Signed plugins with external schemas fail closed until
FM-B015 introduces signatures that bind every package file. Output queues have
independent decoded-byte and item caps, while per-process logs have lifetime
byte and entry budgets.

`capabilityApis` contains trusted host implementations. Storage and absolute
filesystem implementations must honor the FM01 complete-or-reject bounded-read
contract. Supplying an API does not grant it: the host still intersects the
manifest declaration, policy grants, and instance grants for every request,
and every request must carry the currently active run ID. A storage watch is a
live bounded stream: the plugin runner starts its opaque handle explicitly and
cancels it on early return. The host caps concurrent handles and closes all
remaining iterators when the active run ends or the session retires.
