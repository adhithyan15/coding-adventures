# D18V — Chief of Staff `net.fetch`: the host-mediated network operation

## Status

Specified 2026-10-08, from the owner's direction on #13980 ("we should build
net fetch"). Implemented by `code/packages/rust/chief-of-staff-net-fetch`. It is
wired into the daemon by P1.4c.

## Why this exists

D18's leased mode ends with a step nothing implemented:

```text
Agent receives { vault_ref, expires_at_ms }
Agent passes vault_ref to an approved host operation   <-- no such operation
Host atomically consumes the lease
```

`vault.request_lease` worked, but its receipt had nowhere to go. No production
tool accepted a `VaultRef`, so a lease could only expire. Offering the lease
tool to a model on its own would advertise a capability that can only fail.

`net.fetch` is that approved host operation. It is also useful without the
vault: an agent whose signed manifest declares `api.weather.gov:443` can fetch
the forecast with no credential at all. That is the weather reference agent
(#142), which declares Tier 1 because `net.fetch` requires it (see Privilege).

```text
model ── vault.request_lease{secret_name} ──▶ daemon ──▶ { vault_ref }
model ── net.fetch{url, credential:{vault_ref, header}} ──▶ daemon
            │ 1. authorize URL against the host's signed manifest
            │ 2. consume the lease (must be this host's lease)
            │ 3. resolve DNS, refuse non-public addresses
            │ 4. TLS (SNI + verification) to the resolved address
            │ 5. inject the secret into one header, send, read bounded
            │ 6. scrub every echo of the secret from the response
            ▼
model ◀── { status, headers, body }   — never the secret
```

## The tool

`net.fetch`, D18D side effects `External`, idempotency `Never`.

### Input

| Field | Type | Rule |
|---|---|---|
| `url` | string | Absolute `https://` URL. No userinfo, no fragment. The host must be a DNS name, not an IP literal. At most 2 048 bytes. |
| `method` | `"GET"` \| `"POST"` | Required. |
| `headers` | object of string → string | Optional. Names come from the allowlist in V-R4. At most 16 entries. |
| `body` | string | Only with `POST`. At most 64 KiB, UTF-8. |
| `credential` | object | Optional: `{ vault_ref, header, scheme? }`. See V-S1 to V-S5. |

### Output

| Field | Type | Meaning |
|---|---|---|
| `status` | integer | HTTP status code. |
| `headers` | object | Only response headers from the allowlist in V-R6. |
| `body` | string | UTF-8 response body, after scrubbing (V-S4). |
| `truncated` | boolean | Whether the body was cut at the bound in V-R5. |

Failures are bounded, secret-free error kinds: `unauthorized`,
`invalid_request`, `credential_refused`, `address_refused`, `network`,
`tls`, `protocol`, `binary_body`, `timeout`.

## Authorization

**V-A1 — the signed manifest is the allowlist.** A call is authorized only if
the calling host's **signed** agent manifest declares both:

- `net:dns` with the URL's host as target, and
- `net:connect` with `host:port` as target, where port 443 is the default.

Matching is exact, with no wildcards. A malformed `net` target in a signed
manifest, such as a wildcard, is an error and is never skipped silently. Hosts are compared after lowercasing and
removing one trailing dot. This is the same rule `operation-primitives`'
`OperationHttpClient` already applies, and that crate is reused rather than
reimplemented.

**V-A2 — the manifest comes from the registered package, pinned by hash.** The
daemon reads capabilities from `verify_agent_package` over the binding's
registered package path, using the daemon keyring. It refuses the call unless
the package digest equals the registration's `package_hash`. This is the same
check the process supervisor makes at spawn. A host cannot widen its own
allowlist by editing its package after it was registered.

**V-A3 — a host with no `net:connect` capability is not offered the tool.**
`definitions` omits `net.fetch` for such a binding, because a tool it could
never use is noise on the model's surface.

## Network rules

**V-R1 — HTTPS only, certificate-verified.** TLS through `tls-platform`, with
SNI set to the URL host, the bundled roots, and hostname verification. Plain
HTTP is never offered.

**V-R2 — resolve, then check every address, then connect to the checked one.**
For IPv6 this is an allowlist: only global unicast `2000::/3` is accepted, and
inside that, Teredo `2001::/32`, benchmarking `2001:2::/48`, ORCHID
`2001:10::/28` and `2001:20::/28`, documentation `2001:db8::/32` and 6to4
`2002::/16` are refused. Several of those embed or translate an IPv4 address,
as NAT64 `64:ff9b::/96` does outside `2000::/3`; on a DNS64 network NAT64
reaches RFC 1918 hosts. A denylist missed NAT64 in the first draft, which is
why IPv6 is now an allowlist.

The daemon resolves the host itself and refuses unless **every** resolved
address is public. It refuses loopback, private (RFC 1918, ULA `fc00::/7`),
link-local, CGNAT `100.64.0.0/10`, unspecified, multicast, broadcast,
documentation ranges, and IPv4-mapped forms of any of these. It then connects
to the address it checked, not to a fresh lookup. That closes DNS rebinding:
an allowed name cannot be resolved once to pass the check and again to reach
`127.0.0.1`, where the daemon's own control API listens.

**V-R3 — no redirects.** A 3xx response is returned as is, including
`location`. Following it would send the request, and possibly the credential,
to a host the manifest never named. The agent can call `net.fetch` again on
the new URL, and that call is authorized on its own.

**V-R4 — request headers are allowlisted.** The model may set `accept`,
`accept-language`, `content-type`, `user-agent`, `if-none-match` and
`if-modified-since`. The daemon owns `host`, `connection: close`,
`accept-encoding: identity` and `content-length`, and the credential header
(V-S2). Every other name is refused, including `cookie`, `authorization`
outside `credential`, `proxy-*` and `transfer-encoding`. Values must not
contain CR, LF or NUL, so headers cannot be smuggled in.

**V-R5 — everything is bounded.**

| Bound | Value |
|---|---|
| Connect timeout | 10 s |
| Handshake timeout | 10 s |
| Read and write timeouts | 15 s each |
| Response head | 32 KiB |
| Response body | 256 KiB; beyond that, `truncated: true` |

The body is decoded from `Content-Length`, chunked, or read until EOF. A chunk
size is the server's word and the server is not trusted. Every size is checked
with checked arithmetic against the bytes actually present and the remaining
body budget before any slice is taken, and a chunk-size line longer than 128
bytes is a protocol error. Without this, a size near `usize::MAX` would wrap
`size + 2` and panic the daemon. A
compressed body cannot occur, because `accept-encoding: identity` is forced.

**V-R6 — response headers are allowlisted.** `content-type`, `content-length`,
`location`, `retry-after`, `date`, `etag`, `last-modified`, `cache-control`.
`set-cookie` never reaches the model.

**V-R7 — text bodies only.** A body that is not valid UTF-8 fails with
`binary_body`. Base64 for binary content is a possible later addition.

## Credentials (leased mode)

**V-S1 — a credential is a lease, never a value.** `credential.vault_ref` must
be a receipt from `vault.request_lease`. A plaintext secret has no field to go
in.

**V-S2 — the secret goes into exactly one header.** `credential.header` is one
of `authorization`, `x-api-key` or `x-auth-token`. The value sent is
`scheme + " " + secret` when `scheme` is given (`Bearer`, `Basic`, `Token`),
otherwise the secret alone. The secret must be 8–4 096 bytes of printable
ASCII with no space, CR or LF, or the call fails with `credential_refused` and
nothing is sent. The 8-byte minimum exists for V-S4: scrubbing a one- or
two-byte secret from a response would redact ordinary text, and real API keys
are never that short.

**V-S3 — a lease is consumed only by the host it was issued to.** A
`VaultRef` is a bearer reference, and a reference that leaks onto a channel
must not be redeemable by whoever reads it. The vault runtime records the
attested agent at issue time, and `net.fetch` consumes the lease through a
call that requires the same attested agent. A mismatch fails with
`credential_refused` and leaves the lease unconsumed, so its rightful holder
still has it.

**V-S4 — every echo of the secret is masked in place, twice.** Some APIs
reflect request headers, for example in error bodies. Each echo's bytes are
overwritten with `*`, at two points:

1. **On the raw response buffer, before anything parses or copies it.** The
   HTTP parser copies header values into ordinary strings, including headers
   off the allowlist such as an echoed `authorization`. Masking first means
   those copies are already clean.
2. **On the decoded body.** Chunked framing can sit in the middle of an echo
   (`…Bear` · `\r\n9\r\n` · `r tok…`). No raw-byte pass can see that echo,
   and decoding joins it back together, so the reassembled body is masked
   again.

Lengths never change, so `Content-Length` and chunk sizes still describe the
body. The model learns the secret's length, already bounded to 8–4 096 bytes,
and nothing else about it.

An echo is matched **byte by byte, in whatever encoding each byte came back
in**: as itself, as `%XX` in either hex case, as a JSON `\u00XX` escape, as a
backslash escape of `"`, `\` or `/`, or, for the space after a scheme word, as
a form-encoding `+`. This covers mixed encodings as well: Python's `quote`
leaving `/` alone, Go's `\u003c`, PHP's `\/`, and lowercase escapes. It also
never builds an encoded copy of the secret. The matcher tracks every live
reading rather than committing to the first encoding that fits, because a
secret that itself contains `%25` or `\\` would otherwise be missed when echoed
plainly. The set of live readings is capped at 16, so the scan stays linear.
Other encodings, such as base64, are not covered, and V-S7 is the stronger
control.

**V-S4a — a truncated credentialed body loses a fixed tail.** An echo cut off
at the end is only a partial match, which the matcher cannot see. Masking such
a tail only when it looks like a prefix of the secret would be an oracle:
whether the tail was masked would tell the model whether its guess was right,
one byte at a time. So whenever a credentialed response is truncated, a fixed
tail of `6 × (4 096 + 7) − 1` bytes is dropped unconditionally. That is enough
for the longest possible encoded partial echo. The amount depends on neither
the content nor the actual secret, so the tail size does not reveal the
secret's length either. An
unauthenticated response loses nothing beyond the documented bound.

**V-S5 — the secret lives only in zeroizing memory.** The request buffer is
allocated once at its final size and zeroized after the write. The raw
response buffer and the decoded body are both zeroizing, and each is masked
before any copy is made. The matcher never builds an encoded copy of the
secret.

**V-S7 — a secret goes only to the destinations it was provisioned for.**
`CredentialSource::redeem` receives the request's `host:port`. The vault
refuses, without consuming the lease, unless the secret's record allows that
destination. A manifest can name several hosts, and without this rule a key
minted for one API could be written into a request to another host the same
agent may reach. That host could then log it, or reflect it in an encoding
V-S4 does not recognize. The destination list is part of the secret's sealed
record (D18U version 2, P1.4c).

**V-S6 — the lease is consumed only after authorization.** Steps 1 and 3 of
the diagram run first. A refused URL never consumes a lease.

## Privilege

A D18D definition carries one tier. `net.fetch` declares Tier 1, and
`vault.request_lease` declares Tier 2. The daemon's agent tool source (V-D2)
enforces both: it registers a tool for a host only through the host runtime's
`check_registration`, which refuses a tool whose tier is above the manifest's
`privilege_tier`. A network-reading agent therefore declares at least Tier 1,
and one that also leases credentials declares Tier 2.

This is enforced only for the tools this source offers. The smart-home
model-tool source does not check tiers, and VLT06 still records that tier
ceiling as inert there (#13980).

## Daemon composition (P1.4c)

**V-D1: the vault loads at startup and fails closed.** When `[vault] kek_path`
is configured, the daemon opens the vault (`open_chief_vault`) and registers
every sealed record into one `ChiefVaultRuntime` (`ChiefSecretStore::
register_all`) before it serves anything. A bad KEK, an unreadable store or one
corrupt record stops startup. The daemon does not run with part of its vault.
Without `kek_path` there is no vault, and `vault.request_lease` is never
offered.

**V-D2: each host's surface comes from its verified package.** For a binding,
the daemon:

1. verifies the registered package path against the daemon keyring
   (`verify_agent_package`);
2. requires the package digest to equal the registration's `package_hash`;
3. parses the signed `manifest.json`, refuses it when its `privilege_tier` is
   above the signing key's `maximum_tier` (as the host runtime does at spawn),
   and derives a `HostProfile` from it;
4. offers a tool only if the profile's `check_registration` accepts it. That
   means the tool is in `allowed_tools`, its tier is within `privilege_tier`,
   and its `required_capabilities` are all in `tool_capabilities`.

Tool-specific conditions apply on top:

- `net.fetch` also needs a `net:connect` capability (V-A3).
- `vault.request_lease` also needs a configured vault, and a manifest
  `vault_access` whose `mode` is `leased` or `both`.

If any step fails, this source offers that binding nothing, and executing
one of its tools is `Unauthorized`. Other sources, such as smart home, do their
own authorization and are not affected: this source can only add tools to a
surface, so failing closed here means adding none. The supervisor refuses to
spawn a package that fails these checks, so a failure here means the package
changed on disk after registration.

The result is cached under the host name, package path and package hash, at
most 256 entries. A cached entry was computed from bytes whose digest is that
hash, so editing the package on disk cannot change it. Re-registering a host
under a new hash creates a new entry.

**V-D3: identity is the registration's host name.** Every call runs with
`agent_id` set to `registration.host_name()`, the identity the smart-home
source also uses. Leases are recorded against it (VLT06 P8), so
`chief-of-staff vault put --agent` names host names. The model cannot supply
or override this identity.

**V-D4: the manifest's `vault_access` narrows a lease request.** Before the
vault sees a lease request, the source requires `secret_name` to be in
`vault_access.secrets`, and `ttl_ms` to be at most `max_lease_ttl` seconds.
The sealed record's own policy is still checked afterwards. Both must allow a
lease: the signed manifest says what the agent asked to be able to use, and
the record says what the owner granted.

**V-D5: redemption goes through `consume_for`.** The `CredentialSource` that
`net.fetch` receives calls `ChiefVaultRuntime::consume_for(vault_ref,
host_name, destination)`. Every refusal (an unknown or expired reference,
another host's lease, a destination outside the record) becomes
`credential_refused`. Which of these it was is not reported, so the model
learns nothing about leases it does not hold.

**V-D5a: only DNS and the TLS transport are replaceable.** The daemon holds
the network edge as an `Arc<dyn Fetcher>`. `Fetcher` is sealed, so `NetFetch`
is its only implementation. A `Fetcher` that skipped authorization or the
address check would be a way to spend a lease anywhere, and sealing makes that
impossible to write.

- `run` and `compose_host_data_plane` pass `NetFetch::production()`.
- `compose_host_data_plane_with_fetcher` takes a `NetFetch<R, T>`, meaning a
  caller-chosen `Resolver` and `Transport`. The P1.5 end-to-end test passes
  fakes for both.

Authorization, the public-address check, redemption, encoding and masking
live inside `NetFetch`, so that test exercises all of them. TLS certificate
and server-name verification belong to `TlsTransport`, so that test does not
cover them. `tls-platform`'s own tests cover them instead.

**V-D6: failures are tool results, not transport failures.** A refused or
failed `net.fetch` returns `is_error: true` with output `{ kind, message,
details }`. `kind` is the D18D error kind, `message` is fixed text, and
`details.reason` is the V-Output kind. The model can tell `unauthorized` from
`timeout` and act on it, and nothing in the result carries request or
response data.

## Audit

Every call produces a payload-free D18D journal entry: tool id, host, URL
host, method, status or error kind, whether a credential was used, and
duration. It never includes the path or query, headers, bodies, or the
secret. Query strings often carry identifiers.

**Not implemented yet.** The daemon's model-tool path has no D18D journal sink
for any source, smart home included, and P1.4c does not add one. Until it
exists, `net.fetch` calls are not audited. The backlog tracks this as P4.22
on #13980.

## What this does not do

- No cookies, sessions, or connection reuse.
- No proxies.
- No methods other than `GET` and `POST`.
- No streaming.
- It does not sandbox the daemon's own network access. The manifest check
  limits what an agent can make the daemon fetch. D18S (P2) limits what the
  agent process itself can reach.
