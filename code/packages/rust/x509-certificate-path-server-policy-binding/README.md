# x509-certificate-path-server-policy-binding

This repository-owned Rust crate binds a constructed X.509 certificate path to
complete server-certificate policy for the path's exact decoded target object.
The returned non-copyable evidence retains both inputs, preventing policy for a
different certificate—even a byte-equal independent decode—from being attached
to the path.

```rust
use x509_certificate_path_server_policy_binding::
    bind_constructed_x509_certificate_path_to_server_policy;

let bound = bind_constructed_x509_certificate_path_to_server_policy(
    constructed_path,
    validated_server_policy,
)?;
assert!(core::ptr::eq(
    bound.constructed_path().target_certificate(),
    bound.server_policy().certificate(),
));
# Ok::<(), Box<dyn std::error::Error>>(())
```

The crate performs no certificate parsing, issuer-link authentication,
algorithm routing, trust-anchor validation, path-length or name-constraint
processing, revocation, TLS, HTTP, OAuth transport, or network access. It is
one evidence-integrity prerequisite for later complete path validation and does
not make concrete HTTPS available.
