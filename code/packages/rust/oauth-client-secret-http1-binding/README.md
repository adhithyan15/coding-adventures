# oauth-client-secret-http1-binding

Provider/trace-audited identity binding between one custody-built OAuth
client-secret request and the strict repository-owned HTTP/1 evidence that a
later HTTPS composition must consume.

`bind_client_secret_http1_request_material` publishes an attempted event before
this binder inspects credential-bearing material. Inside that audited boundary
it constructs the fixed-body proof over the request's original form-body
allocation, writes and binds canonical Content-Length in a bounded caller
buffer, and, for Basic authentication, constructs an `Authorization` field
over the request's original Authorization allocation. Body authentication
constructs no Authorization field. Callers cannot supply alternate credential
bytes or prebuilt evidence as inputs. The already custody-released request
remains independently accessible to its owner, so this proof attests only to
the reads and construction performed by this call. A rejected or successful
outcome must be durable before the call returns an error or releases the
non-copyable proof. Validation or audit failure leaves the caller buffer
unchanged, and deterministic output binding after success does not reread
credential-bearing input.

The proof retains the exact request reference, fixed body and canonical
Content-Length binding, optional Authorization field, provider, and trace.
Byte-equal copied material is never accepted as input. Audit events, stable
errors, and Debug output contain no endpoint, credential, body, field, or
observed-length payloads.

This crate does not obtain or edit credentials, encode or copy request bytes,
authorize methods or endpoints, bind an HTTPS destination, perform TLS or HTTP
I/O, open a socket, or authorize transport. Complete request release,
destination validation, repository-owned TLS/HTTP composition, response
policy, and external-effect audit remain separate boundaries. It does not make
OAuth HTTPS or live providers ready.

## Validation

```bash
bash BUILD
cargo clippy -p oauth-client-secret-http1-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p oauth-client-secret-http1-binding --no-deps
```
