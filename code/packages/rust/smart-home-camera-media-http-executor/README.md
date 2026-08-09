# smart-home-camera-media-http-executor

This package is the native HTTP(S) snapshot transport behind
`smart-home-camera-media`. The policy broker lends a secret endpoint only for
one lease redemption; this executor connects only to the broker-retained,
reviewed socket address while preserving the canonical host for HTTP `Host`,
TLS SNI, and certificate verification.

Production delivery requires HTTPS and a pinned connection target. An explicit
loopback-only policy switch exists for transport tests. Responses are bounded
before allocation, redirects and content encodings are rejected, HTTP framing
is parsed strictly, and successful payloads must be JPEG, PNG, or WebP with a
matching signature. Stream delivery remains unsupported until a concrete
supervised stream host owns resource teardown.

Optional Basic or RFC 7616 Digest credentials remain in zeroizing,
process-local executor state keyed by normalized camera entity. Delivery first
probes without credentials, prefers advertised SHA-256 Digest over MD5 and
Basic, uses a fresh CSPRNG client nonce, and permits one refreshed Digest
challenge retry. Endpoint URIs, credentials, authorization values, and response
bytes are absent from errors and debug output.

Credential registration fails when an entity already has credentials. Hosts
must explicitly remove the old value before replacement, which keeps one
delivery from silently overwriting another host-owned credential lifetime.
