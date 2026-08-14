# chief-of-staff-biometric-approval

`chief-of-staff-biometric-approval` is the D18 Tier 2 boundary between the
transport-independent Trust Checker and an operator-reviewed native biometric
helper. The helper executable is selected by an absolute path, launched directly
without a shell, and receives no inherited environment. This lets macOS, Linux,
Windows, and companion-device deployments bridge Touch ID, Face ID, Windows
Hello, fingerprint, or another reviewed native authenticator without putting
platform UI or credentials in the Chief daemon.

The provider writes this bounded UTF-8 protocol to standard input:

```text
CHIEF-TIER2-BIOMETRIC/1
request_id <lowercase-hex UTF-8 bytes>
requested_by <lowercase-hex UTF-8 bytes>
effective_tier 2
timeout_ms 30000
resources <decimal count>
resource <tier 0..3> <lowercase-hex UTF-8 bytes>
...
end
```

After parsing the complete prompt and presenting the native authentication UI,
the helper must first write exactly `ready\n`. It may then write exactly
`approve biometric\n` only after the native biometric policy succeeds, or
`deny\n` after denial. The provider starts a fresh helper for every exact request,
so a response is accepted only on that request's private process pipe and cannot
be replayed into a later request.

Missing acknowledgement, early exit, partial or malformed response, blocked
prompt delivery, launch/I/O failure, or any non-Tier-2 request is an adapter
error. An acknowledged helper that remains pending through the supplied deadline
returns `TimedOut`; Trust Checker treats every Tier 2 timeout as denial. The
provider supplies only `CHIEF_APPROVAL_PROTOCOL=2` in the child environment.
Helpers must not spawn descendants that retain the protocol pipes.

## Validation

```sh
cargo test -p chief-of-staff-biometric-approval -- --nocapture
cargo clippy -p chief-of-staff-biometric-approval --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-biometric-approval --no-deps
```
