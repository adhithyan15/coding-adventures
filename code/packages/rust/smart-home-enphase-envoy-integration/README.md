# Smart Home Enphase Envoy Integration

This package implements read-only local telemetry for Enphase IQ Gateway and
Envoy devices using Enphase's documented token-authenticated local APIs.

Production configuration requires a credential-free HTTPS origin, a known
gateway serial number, a Vault reference for a pre-generated access token, and
certificate trust configured through the shared TLS platform. Plain HTTP is
accepted only for loopback protocol tests.

The integration authorizes D23 smart-home reads before transport I/O, requests
`/ivp/meters` and `/ivp/meters/readings`, bounds response sizes and meter
counts, matches native records by EID, and installs confirmed aggregate meter
telemetry. The bearer token is zeroized and materialized only while the
transport encodes the Authorization header.

Per-microinverter inspection additionally targets Enphase's documented
`/api/v1/production/inverters` endpoint. It is deny-by-default and requires a
host-installed data-use grant for device-identifier inspection, an explicit
purpose and consent receipt, and ephemeral raw-identifier retention before any
credential or network I/O. A separate Vault-leased 32-byte key derives stable
128-bit host-scoped pseudonyms. Raw microinverter serials are held only in a
zeroizing response tree and never enter entity IDs, metadata, state, logs, or
debug output. Confirmed inverter entities expose only last/max reported active
power, device type, and last-report time under the pseudonym.

Identifier-key rotation consumes the source and destination 32-byte leases as
one atomic batch, reads the inverter endpoint exactly once, and derives only
old/new pseudonym pairs from the zeroizing response. The host requires exact
correspondence with every installed inverter before building a whole-device
replacement, including deterministic replacement identities for the gateway's
meter entities. Both keys and all native serial values are disposed before the
revision-guarded runtime-store migration begins. Opaque automation definitions
or state that still reference a source identity cause the durable write and
live runtime swap to fail closed.

Cloud login, token generation or renewal, legacy gateway authentication,
live battery or relay topology, and control APIs remain outside this package.
