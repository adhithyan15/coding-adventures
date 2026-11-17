# Changelog

## 0.1.0

- Add a non-copyable proof that one exact decoded certificate's Subject
  Alternative Name matches the same DNS name used by TLS SNI and HTTP/1 Host.
- Retain the exact certificate, SNI, Host authority, and certificate-identity
  evidence without granting TLS, HTTP, OAuth, I/O, or external-effect authority.
- Keep stable failures and Debug output free of hostnames and certificate bytes.
- Bind the complete destination proof to one strict HTTP/1 Host field only when
  its value is the retained authority's canonical lowercase-host and
  shortest-decimal-port encoding.
- Retain both non-copyable inputs with payload-free failures and redacted Debug
  output without claiming complete request-head uniqueness or transport
  authority.
- Bind that canonical Host proof to an exact caller-borrowed slice of
  additional validated fields under a caller-selected total-field ceiling
  hard-capped at 64.
- Reject every additional case-insensitive Host spelling while retaining the
  complete field slice, caller order, and limit policy with payload-free
  failures and redacted Debug output.
- Bind the exact HTTPS field-set proof to one exact fixed-body and canonical
  Content-Length proof, counting that field inside the retained total limit.
- Reject every additional case-insensitive Content-Length spelling and every
  Transfer-Encoding field while retaining both complete proofs with
  payload-free failures and redacted Debug output.
- Bind one exact provider-policy-classified public request line to the complete
  fixed-body destination proof under a hard-capped complete-head byte ceiling.
- Encode the exact public request line, Host, Content-Length, and terminating
  CRLF into caller-owned storage without allocation or partial output on
  capacity failure; retain but do not write the body.
- Require a non-copyable exact request-line proof classified by data-driven
  provider policy as public and query-free; reject every additional field until
  a later provider/trace-audited credential-release composition exists.
- Delegate request-line and field serialization to their repository-owned
  encoders so the composition cannot drift from the owning wire primitives.
- Add a provider/trace-audited request-head release for a non-empty ordered
  public additional-field slice under exact data-driven method, path,
  destination, and field name/value policy; admit only `Accept` and `Content-
  Type` while credential-bearing and custom fields remain blocked.
- Preflight every policy, length, and output condition before durable audit and
  every output mutation; return one redacted non-copyable encoded wrapper that
  retains the exact audited provider and trace only after the gate succeeds.
