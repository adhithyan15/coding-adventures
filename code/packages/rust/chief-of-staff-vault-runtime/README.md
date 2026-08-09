# Chief of Staff Vault Runtime

This crate is the trusted host-side broker for short-lived secret leases.
`request_lease` returns the canonical agent-facing receipt:

```text
{ vault_ref, expires_at_ms }
```

`vault_ref` is an opaque bearer capability. Agents may pass it only to an
approved host operation that explicitly accepts a Vault reference; they cannot
resolve it into secret bytes. The trusted host atomically consumes the reference,
uses the zeroizing payload within that operation, and makes the reference unusable
after consumption, release, revocation, or expiry.

The receipt never contains plaintext, ciphertext, or a decryption key. Its `Debug`
implementation also redacts the bearer reference so routine diagnostics do not
turn logs into an alternate secret channel.
