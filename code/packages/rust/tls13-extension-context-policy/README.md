# tls13-extension-context-policy

`tls13-extension-context-policy` is an allocation-free policy layer over the
borrowed, structurally validated extension blocks produced by
`tls13-extension-block`.

It contains the complete 22-row TLS 1.3 placement table from RFC 8446 section
4.2 as ordinary rule data. A caller may instead supply a narrower rule slice.
The policy validates a future client's outbound ClientHello and the server's
ServerHello, EncryptedExtensions, HelloRetryRequest, Certificate,
CertificateRequest, and NewSessionTicket extension blocks. It also enforces
that ClientHello `pre_shared_key` is last and that server response extensions
were offered by the client, except for an unsolicited HelloRetryRequest
`cookie`.

```rust
use tls13_extension_context_policy::{
    ExtensionContextPolicy, ServerExtensionContext,
    STANDARD_TLS13_EXTENSION_RULES,
};

let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES)?;
policy.validate_client_hello(&client_extensions)?;
policy.validate_server_message(
    ServerExtensionContext::EncryptedExtensions,
    &server_extensions,
    &client_extensions,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Recognition is not implementation or enablement. The standard table proves
only that a recognized extension type is in an RFC-permitted message context.
This package does not decode extension data, require mandatory extensions,
validate value relationships, sequence a handshake, emit alerts, calculate a
transcript, invoke cryptography, validate certificates or hostnames, establish
trust, check revocation, perform I/O, or make TLS, HTTPS, or live OAuth-provider
transport available.

## Dependencies

- `tls13-extension-block`: exact generic extension-vector framing and borrowed
  entries.

## Development

```bash
# Run tests
bash BUILD
```
