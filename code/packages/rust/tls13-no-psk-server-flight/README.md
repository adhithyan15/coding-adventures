# tls13-no-psk-server-flight

`tls13-no-psk-server-flight` is an allocation-free message-type sequencing
policy for the TLS 1.3 server authentication flight after `ServerHello` on the
repository's no-PSK client path. It requires `EncryptedExtensions`, accepts one
optional `CertificateRequest`, and then requires `Certificate`,
`CertificateVerify`, and `Finished` exactly once and in order.

It does not make TLS or HTTPS available. The caller must separately establish
that the already processed `ServerHello` selected no PSK. This package does not
sequence either hello or the client flight, decode or validate a message body,
update a transcript, authenticate `CertificateVerify` or `Finished`, validate
certificates, establish trust, check identity or revocation, access credentials,
sign, derive keys, emit alerts, protect records, perform I/O, or authorize HTTP
or OAuth transport.

## Boundary

The state starts immediately after a separately validated no-PSK
`ServerHello`. Each accepted recognized handshake type advances one exact
position. An unexpected type produces a stable payload-free error before any
state mutation. Once complete, every further message fails as already complete
because post-handshake sequencing belongs to a later state owner.

The policy retains whether the optional `CertificateRequest` was accepted and
makes that value available to later client-flight enforcement. This package
does not force a downstream caller to consume the getter; a future composition
must bind the retained requirement to client-flight construction. Completion
proves ordering only. It does not prove that any accepted body was well formed
or authenticated, that the transcript includes it, or that the connection is
secure.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-no-psk-server-flight
cargo clippy -p tls13-no-psk-server-flight --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-no-psk-server-flight --no-deps
```
