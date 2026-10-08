# Chief of Staff `net.fetch`

The host-mediated HTTPS operation from
[D18V](../../../specs/D18V-chief-of-staff-net-fetch.md). An agent asks for a
URL. The daemon checks it against the agent's **signed** manifest, does the
network work itself, and returns a bounded, scrubbed response.

It is also where a Chief vault lease is spent. In leased mode the model only
ever holds an opaque `vault_ref`. `net.fetch` redeems it inside the daemon,
for this host and this destination only. It writes the secret into one
request header, and masks every echo of it, in place, out of whatever comes
back.

```text
model ── vault.request_lease ──▶ { vault_ref }
model ── net.fetch { url, credential: { vault_ref, header: "x-api-key" } }
            ▼  authorize against the signed manifest  (exact net:dns + net:connect)
            ▼  resolve once; every address must be public
            ▼  redeem the lease (only now, and only for this host)
            ▼  TLS to the checked address, SNI = host
            ▼  bounded read; mask every echo of the secret in place
model ◀── { status, headers, body, truncated }
```

## Where it sits

| Layer | Owner |
|---|---|
| Which URLs a host may reach | the host's signed agent manifest (`net:dns`, `net:connect`) |
| Reading that manifest from the registered, hash-pinned package | the daemon (P1.4c) |
| Policy, wire format, address checks, scrubbing | **this crate** |
| Redeeming a lease, and refusing one issued to another host | the daemon, through `CredentialSource` |
| TLS | `tls-platform` |

## Usage

```rust
use chief_of_staff_net_fetch::{parse_request, NetAllowlist, NetFetch};

let allowlist = NetAllowlist::from_capabilities(&manifest.capabilities)?
    .expect("this host declares network access");
let request = parse_request(&model_arguments)?;
let response = NetFetch::production().execute(&allowlist, &request, Some(&leases))?;
let tool_output = response.to_json();
```

A long-lived composition that should not carry the resolver and transport
type parameters can hold an `Arc<dyn Fetcher>` instead. Every `NetFetch` is a
`Fetcher`, and `fetch` is `execute`.

## Rules worth knowing

- **HTTPS only, no redirects.** A 3xx response comes back with its `location`.
  Following it could send a credential to a host the manifest never named.
- **No IP literals, no internal addresses.** The refused IPv4 ranges are
  loopback, private, link-local (cloud metadata), CGNAT, multicast,
  documentation and reserved. IPv6 must be global unicast, and NAT64, 6to4,
  Teredo and the IPv4-mapped and translated forms are refused.
- **No request smuggling.** URL paths and queries are restricted to RFC 3986
  characters, so no CR, LF or space can reach the request line.
- **Fixed headers.** A model may set `accept`, `accept-language`,
  `content-type`, `user-agent`, `if-none-match` and `if-modified-since`. The
  daemon owns `host`, `connection`, `accept-encoding` and `content-length`.
  The credential goes into `authorization`, `x-api-key` or `x-auth-token`.
- **Bounded.** Timeouts are 10 s to connect, 10 s for the handshake and 15 s
  per read or write. The response head is capped at 32 KiB and the body at
  256 KiB (`truncated: true` beyond that).
- **Tier.** The definition declares Tier 1, but the daemon's model-tool path
  does not enforce tiers yet. D18V says so in writing.

## Testing

```sh
cargo test -p chief-of-staff-net-fetch
```

The tests never touch DNS or a remote socket. A fake resolver answers lookups,
and a fake transport records the exact request bytes and plays back canned
responses. The tests cover the internal-address refusals, the exact wire
format, scrubbing of echoed secrets, and the guarantee that a refused request
never consumes its lease.
