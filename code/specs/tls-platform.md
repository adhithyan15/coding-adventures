# TLS Platform

## Overview

`tls-platform` is the runtime-facing seam between higher transport
crates (`https-transport`, future `wss-transport`, future
`smtp-transport-tls`) and the **TLS implementation provided by the
operating system**. Every modern OS ships a battle-tested,
maintained-by-someone-else TLS stack that handles cipher selection,
certificate verification against the system trust store, protocol
version negotiation, and the long tail of edge cases that an
in-house implementation would have to discover and re-fix forever.

This crate's job is to wrap those OS APIs behind one `TlsConnector`
trait so the rest of the codebase never depends on Schannel,
Network.framework, or OpenSSL directly. The pattern is the same
as your existing `transport-platform` crate, which wraps kqueue,
epoll, and WSAPoll behind one event-platform trait. We are not
reinventing TLS; we are giving it a Rust-shaped contract.

When we eventually want to ship a pure-Rust TLS implementation
written in this repository (an interesting project in its own
right), it slots in as a fourth backend behind the same trait. No
caller changes.

```
                   higher transport crates
                          │
          uses TlsConnector  ──────────────  uses TlsStream
                          │
                          ▼
                 tls-platform trait
                          │
       ┌──────────────────┼──────────────────┐
       ▼                  ▼                  ▼
  Schannel           Network.framework     OpenSSL FFI    (native Rust later)
  (Windows)          (macOS)               (Linux)
```

The trait surface is small — connect, read, write, close — because
that is all higher layers actually need. Anything more (negotiated
ALPN, peer cert chain, session tickets) is exposed via narrow
methods on `TlsStream` so they remain optional.

---

## Where It Fits

```
   https-transport, smtp-transport, wss-transport, ...
        │
        │  uses TlsConnector::connect → Box<dyn TlsStream>
        │  reads/writes plain bytes through TlsStream
        ▼
   tls-platform (this spec)
        │
        ├── tls-platform-windows  → Schannel (SSPI)
        ├── tls-platform-macos    → Network.framework / Secure Transport
        └── tls-platform-linux    → OpenSSL via FFI
```

**Depends on:**
- `transport-platform` — the underlying TCP socket abstraction (we
  hand the OS TLS stack a connected TCP socket).
- `capability-cage-rust` — the `connect` call goes through
  `secure_net::dial` first; the manifest gates which hosts/ports
  the agent can reach.

**Used by:**
- `https-transport` (the next spec) — HTTP/1.1 over TLS.
- Future `smtp-transport-tls` — SMTPS / STARTTLS.
- Future `wss-transport` — WebSocket over TLS.
- Any other repo crate needing TLS without writing the protocol.

---

## Design Principles

1. **OS owns the TLS implementation.** We do not write cipher
   suites, certificate verification, ASN.1 parsers, or session
   resumption. The OS does. We wrap.

2. **Trait is the durable artifact.** Three OS backends today; a
   pure-Rust backend tomorrow when we choose to write it. Callers
   depend only on the trait.

3. **System trust store by default.** TLS validation uses the OS's
   own root certificate store. The user's enterprise CAs, pinned
   internal certs, and OS update mechanism (which patches root
   trust) all just work. Bundled CA stores are an opt-in
   alternative for tightly-controlled environments.

4. **No raw FFI escapes upward.** Every OS API call is encapsulated
   in this crate. Higher layers receive Rust types only.

5. **Capability-cage gated.** The TCP connect that precedes the
   TLS handshake goes through `secure_net::dial`, which checks
   `net:connect:host:port` against the agent's manifest. Without
   that capability, the connection never opens; without an open
   socket, no TLS handshake occurs.

6. **Backend selection is explicit, not magic.** We do not auto-
   detect "best backend." A consumer chooses the one for their
   target OS at construction; defaults are provided per-platform
   via a thin facade.

7. **Sync first.** v1 is synchronous. Async wrappers can be added
   when a real workload demands them.

---

## Trait Surface

```rust
pub trait TlsConnector: Send + Sync {
    /// Open a TLS-encrypted stream to (host, port). The TCP
    /// connection is opened internally via secure_net::dial; the
    /// TLS handshake then runs against the OS implementation.
    fn connect(
        &self,
        host:   &str,
        port:   u16,
        config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError>;
}

/// A TLS-encrypted bidirectional stream. Implements std::io::Read
/// and std::io::Write so higher layers can treat it like any
/// stream of bytes after handshake.
pub trait TlsStream: std::io::Read + std::io::Write + Send {
    /// The peer's certificate chain in DER form, root-most first.
    fn peer_certificates(&self) -> Result<Vec<Vec<u8>>, TlsError>;

    /// The ALPN protocol negotiated (e.g., "h2", "http/1.1"), if any.
    fn negotiated_alpn(&self) -> Option<String>;

    /// The TLS version actually negotiated for this session.
    fn negotiated_version(&self) -> TlsVersion;

    /// Send TLS close_notify and shut down the stream cleanly.
    fn close_notify(&mut self) -> Result<(), TlsError>;
}

pub struct TlsConfig {
    /// Minimum acceptable TLS version. Default: TLS 1.2.
    pub min_version:   TlsVersion,

    /// Maximum acceptable TLS version. Default: TLS 1.3.
    pub max_version:   TlsVersion,

    /// ALPN protocols to advertise, in preference order. Empty = no ALPN.
    pub alpn_protocols: Vec<String>,

    /// Where to find root certificates for verification.
    pub root_store:    RootStore,

    /// Server name for SNI. Defaults to the `host` arg of `connect`
    /// if None.
    pub server_name:   Option<String>,

    /// Hostname verification mode. Default: Strict.
    pub verify_mode:   VerifyMode,

    /// Total timeout for the TLS handshake (after TCP connect).
    /// Default: 10 seconds.
    pub handshake_timeout: std::time::Duration,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

pub enum RootStore {
    /// Use the OS's own trust store (recommended).
    SystemDefault,

    /// Use a bundled set of root certificates (e.g., for systems
    /// where the OS store is unreliable).
    Bundled,

    /// Trust only the given certificate(s). For pinned-trust
    /// scenarios.
    Custom(Vec<Vec<u8>>),  // each entry is a DER-encoded cert
}

pub enum VerifyMode {
    /// Verify chain to a root, hostname must match the cert.
    Strict,

    /// Verify chain only; do not check hostname (rare, only for
    /// well-justified cases like SNI-less servers).
    NoHostname,

    /// Skip ALL verification. Test only — never in production.
    /// Available behind a `unsafe-no-verify` cargo feature so
    /// production builds cannot accidentally use it.
    #[cfg(feature = "unsafe-no-verify")]
    NoVerify,
}

pub enum TlsError {
    /// TCP connect failed (DNS, refused, unreachable, etc.).
    TcpConnect      { source: std::io::Error },

    /// TLS handshake failed (cipher mismatch, version mismatch, etc.).
    HandshakeFailed { message: String, alert: Option<u8> },

    /// Certificate chain did not verify against the root store.
    CertVerifyFailed{ message: String, chain_summary: String },

    /// Hostname did not match the certificate's SANs.
    HostnameMismatch{ requested: String, cert_names: Vec<String> },

    /// Handshake exceeded the configured timeout.
    Timeout         { elapsed_ms: u64 },

    /// Connection was closed mid-stream.
    ClosedUnexpectedly,

    /// The capability cage refused the underlying TCP connect.
    CapabilityDenied{ detail: String },

    /// Some other backend-level error (Schannel SEC_E_*, OpenSSL
    /// SSL_ERROR_*, etc.) that doesn't map to a more specific case.
    Backend         { code: i64, message: String },

    /// I/O error during read or write after handshake.
    Io              { source: std::io::Error },
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_version:        TlsVersion::Tls12,
            max_version:        TlsVersion::Tls13,
            alpn_protocols:     vec![],
            root_store:         RootStore::SystemDefault,
            server_name:        None,
            verify_mode:        VerifyMode::Strict,
            handshake_timeout:  std::time::Duration::from_secs(10),
        }
    }
}
```

---

## Per-OS Backends

Each backend lives in its own crate, mirroring the
`transport-platform` package layout. A user crate enables exactly
one backend via cargo features (`default-features` selects the right
one for the target OS).

### `tls-platform-windows` — Schannel

Windows ships **Schannel** (Secure Channel), accessed through the
**SSPI** (Security Support Provider Interface) Win32 API. Schannel
implements TLS up through 1.3 (Windows 11 and Server 2022; earlier
versions cap at 1.2). It uses the **Windows Certificate Store** for
trust, which means CAs added by the user, the enterprise, or
Windows Update are honored automatically.

The implementation:

- Calls `AcquireCredentialsHandle` to get an SSPI credentials handle
  configured for client TLS.
- Calls `InitializeSecurityContext` repeatedly to drive the
  handshake, feeding TCP bytes in and out via the provided socket.
- Uses `SECPKG_ATTR_REMOTE_CERT_CONTEXT` to extract the peer's
  certificate chain.
- Uses `SECPKG_ATTR_APPLICATION_PROTOCOL` for ALPN negotiation.
- Calls `EncryptMessage` / `DecryptMessage` for the data path.
- Calls `ApplyControlToken` with `SCHANNEL_SHUTDOWN` to send
  close_notify.

Win32 calls go through the `windows` crate (Microsoft's official
Rust bindings). All FFI is encapsulated; the public surface is the
trait.

**Schannel notes:**
- Cipher selection is the OS's responsibility — controlled by
  Group Policy and OS version. We do not pass cipher lists.
- TLS 1.3 was added in Windows Server 2022 and Windows 11. On
  earlier Windows we transparently negotiate down to 1.2; if the
  caller's `min_version` is 1.3 and the OS cannot do it, `connect`
  returns `HandshakeFailed`.
- Hostname verification is done by Schannel via
  `SECPKG_ATTR_TARGET_INFORMATION`; we set the SNI hostname there.
- Custom root stores (`RootStore::Custom`) require us to load the
  certs into a temporary in-memory `HCERTSTORE` via
  `CertOpenStore(CERT_STORE_PROV_MEMORY, ...)` and configure
  Schannel to use it.

### `tls-platform-macos` — Network.framework (preferred) / Secure Transport (fallback)

macOS 10.14+ ships **Network.framework** (`nw_*` C APIs and the
Swift `NWConnection` wrapper). It is Apple's modern replacement
for Secure Transport (which is deprecated but still works on
older systems).

The implementation:

- Uses `nw_parameters_create_secure_tcp` to configure a TLS-over-TCP
  connection.
- Sets TLS options via `sec_protocol_options_*`:
  `set_tls_min_version`, `set_tls_max_version`,
  `add_tls_application_protocol` (ALPN),
  `set_verify_block` (custom verification when needed).
- Uses `nw_connection_set_state_changed_handler` to drive the
  handshake to completion.
- Reads / writes via `nw_connection_send` / `nw_connection_receive`.
- ALPN is read via `sec_protocol_metadata_get_negotiated_protocol`.
- Peer cert chain via `sec_protocol_metadata_access_peer_trust`.

For systems where Network.framework is unavailable (older macOS,
unusual configurations), `tls-platform-macos` falls back to
**Secure Transport** (`SSLContext`, `SSLHandshake`, etc.). The fallback
path is documented but discouraged.

**Network.framework notes:**
- Trust evaluation uses the macOS Keychain by default. User-added
  trust anchors and enterprise profiles are honored.
- ATS (App Transport Security) defaults are bypassed because we
  set TLS parameters explicitly; we do not require Info.plist
  configuration.

### `tls-platform-linux` — OpenSSL via FFI

Linux has no single OS-blessed TLS API. The pragmatic choice is
**OpenSSL** (or its API-compatible siblings BoringSSL and
LibreSSL), which is preinstalled on essentially every distribution.
We wrap it via direct FFI (`libssl.so` and `libcrypto.so`).

The implementation:

- `SSL_CTX_new` with `TLS_client_method`.
- `SSL_CTX_set_min_proto_version` / `SSL_CTX_set_max_proto_version`.
- `SSL_CTX_set_alpn_protos` for ALPN.
- `SSL_CTX_set_default_verify_paths` to use the system trust store
  (typically `/etc/ssl/certs/ca-certificates.crt`).
- `SSL_new` to create a session, `SSL_set_fd` to attach our TCP
  socket, `SSL_set_tlsext_host_name` for SNI, `SSL_connect` to
  drive the handshake.
- `SSL_get0_alpn_selected` for negotiated ALPN.
- `SSL_get_peer_cert_chain` for the peer chain.
- `SSL_read` / `SSL_write` for the data path.
- `SSL_shutdown` for close_notify.

We bind the OpenSSL ABI directly (no `openssl` crate dependency in
v1; we want first-party control over the binding surface). The
binding is small — fewer than a hundred functions — and is itself
encapsulated behind the `TlsConnector` / `TlsStream` traits.

**OpenSSL notes:**
- We probe at runtime for `OPENSSL_VERSION_NUMBER` to handle the
  occasional API drift between 1.0.x, 1.1.x, and 3.x.
- We do **not** use BoringSSL-specific features even when running
  against BoringSSL; the contract is the lowest common denominator.
- The system trust store path can vary by distro; we probe a small
  fixed list (`/etc/ssl/certs/ca-certificates.crt`,
  `/etc/pki/tls/certs/ca-bundle.crt`, `/etc/ssl/cert.pem`) and use
  the first that exists. `RootStore::SystemDefault` resolves to
  whichever was found.

### Backend selection facade

```rust
pub fn default_connector() -> Box<dyn TlsConnector> {
    #[cfg(target_os = "windows")]
    return Box::new(tls_platform_windows::SchannelConnector::new());

    #[cfg(target_os = "macos")]
    return Box::new(tls_platform_macos::NetworkFrameworkConnector::new());

    #[cfg(target_os = "linux")]
    return Box::new(tls_platform_linux::OpenSslConnector::new());

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    compile_error!("tls-platform has no backend for this OS yet");
}
```

A test crate can swap in a `MockConnector` for unit tests of higher
layers.

---

## Capability-Cage Integration

The TCP connect that precedes the TLS handshake goes through
`secure_net::dial(manifest, "tcp", "<host>:<port>")`. If the
manifest does not include `net:connect:host:port`, the dial returns
`io::Error::other(CapabilityViolationError)` and the TLS handshake
never begins.

`tls-platform` accepts the manifest at construction:

```rust
let connector = SchannelConnector::new(manifest.clone());
```

It uses that manifest for every internal `secure_net::dial` it
performs. The manifest is **the agent's manifest**; the connector
itself has no policy.

---

## Verification and Trust

By default, `RootStore::SystemDefault` is used. This means:

| OS      | Trust source                                                        |
|---------|---------------------------------------------------------------------|
| Windows | Windows Certificate Store (Local Machine and Current User stores)   |
| macOS   | macOS Keychain (System and User keychains, plus admin profiles)     |
| Linux   | Whatever `/etc/ssl/certs/ca-certificates.crt` (or equivalent) holds |

Hostname verification is on by default (`VerifyMode::Strict`). The
hostname presented to `connect(host, port, ...)` is matched against
the cert's Subject Alternative Names per RFC 6125.

Pinned-trust scenarios use `RootStore::Custom(vec![der_bytes])`. The
backend loads those into a per-connector trust store and uses it
exclusively (the OS store is not consulted).

The `unsafe-no-verify` cargo feature exists for tests only. It is
gated so a production build that does not opt in cannot construct a
`VerifyMode::NoVerify` config. The feature flag's name is
deliberately ugly to make accidental use show up in code review.

---

## Test Strategy

### Unit Tests (per backend)

1. **Connect to known good server.** Open a TLS connection to a
   public test server (e.g., `tls13.1d.pw` for TLS 1.3, or a local
   test server we ship), verify the stream is usable, read/write
   simple bytes.
2. **ALPN negotiation.** Configure ALPN with `["h2", "http/1.1"]`,
   verify `negotiated_alpn` returns the expected value.
3. **Cert chain extraction.** Open a connection, verify
   `peer_certificates()` returns at least one cert and the leaf
   has the expected CN.
4. **Min version.** Configure `min_version = Tls13`, connect to a
   server that only does 1.2, verify `HandshakeFailed`.
5. **Cert verification failure.** Connect to a server with an
   expired cert (we ship one as a test fixture); verify
   `CertVerifyFailed`.
6. **Hostname mismatch.** Connect to a server whose cert is for a
   different SAN; verify `HostnameMismatch`.
7. **Custom root store.** Configure `RootStore::Custom(test_ca)`;
   connect to a server signed by that CA; verify success even
   though the OS store doesn't know about it.
8. **Capability denial.** Manifest without `net:connect:host:port`,
   call `connect`; verify `CapabilityDenied` and no TCP connection
   is opened.
9. **Handshake timeout.** Use a network simulator (or a tarpit
   test server) that delays handshake completion; verify
   `Timeout`.
10. **close_notify.** Open, write, call `close_notify`; verify the
    server sees a clean shutdown.

### Cross-Backend Conformance

Each backend runs the same test suite (we share fixtures and
expectations). Backend-specific quirks are documented but the
public behavior is uniform.

### Coverage Target

`>=85%` line coverage for backend code (the FFI surface is hard to
exercise with unit tests; we lean on integration tests against real
servers). `>=95%` for the trait wrappers and the config types.

---

## Trade-Offs

**No protocol implementation in this crate.** We rely on three
different OS implementations with three different bug surfaces. A
single underlying TLS bug therefore affects only one OS in our
codebase. This is also a downside: cipher lists, session resumption
behavior, and TLS 1.3 0-RTT semantics may differ subtly across
backends. We document known divergences as we find them.

**OpenSSL ABI compatibility on Linux.** Different distributions
ship different OpenSSL major versions (1.0, 1.1, 3.x). We probe at
runtime and do not assume any specific feature. The cost is some
backend complexity; the gain is portability without static linking.

**Static linking is out of scope for v1.** Each backend dynamically
links to the OS-provided library. This is the right default
(security updates apply automatically), but it means binaries
break if the system library is missing or changed incompatibly. We
document the runtime dependency.

**No async in v1.** Sync only. A future revision adds an async
trait alongside; backends that have native async APIs
(Network.framework) implement it directly, others wrap sync calls
in `spawn_blocking`.

**No connection pooling.** Higher transport crates (`https-transport`)
manage pools if they want them. `tls-platform` is single-shot.

**Cipher selection is OS-controlled.** We do not pass cipher lists.
This means an environment with a hardened OS configuration that
disables certain ciphers will see different negotiated ciphers
across deployments. We accept this — the OS is the source of truth
for crypto policy.

**Failure mode parity is best-effort.** Mapping Schannel's
SEC_E_* codes, Network.framework's `nw_error_t`, and OpenSSL's
`SSL_ERROR_*` to one `TlsError` enum involves judgment calls. We
match the most common failures precisely; rare ones land in
`Backend { code, message }` with the original code preserved for
debugging.

---

## Future Extensions

- **Native Rust TLS backend** (`tls-platform-rust`) implementing
  TLS 1.3 from RFC 8446. A genuine engineering project; satisfies
  the same trait.
- **Async trait** (`AsyncTlsConnector`, `AsyncTlsStream`) for
  high-concurrency servers.
- **OCSP stapling** support, surfaced as a method on `TlsStream`.
- **Session resumption** (TLS 1.3 PSK / 1.2 session IDs) via an
  optional cache.
- **mTLS** (client certificates), surfaced via `TlsConfig`.
- **QUIC and TLS 1.3 0-RTT** when we add HTTP/3.

These are deliberately out of scope for v1.
