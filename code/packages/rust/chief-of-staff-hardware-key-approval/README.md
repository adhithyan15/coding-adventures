# chief-of-staff-hardware-key-approval

`chief-of-staff-hardware-key-approval` is the D18 Tier 3 boundary between the
transport-independent Trust Checker and an operator-reviewed native hardware-key
helper. The helper executable is selected by an absolute path, launched directly
without a shell, and receives no inherited environment. This lets deployments
bridge FIDO2, WebAuthn, YubiKey, or another reviewed physical authenticator
without putting platform UI, PINs, credentials, or key material in the Chief
daemon.

The provider writes this bounded UTF-8 protocol to standard input:

```text
CHIEF-TIER3-HARDWARE-KEY/1
request_id <lowercase-hex UTF-8 bytes>
requested_by <lowercase-hex UTF-8 bytes>
effective_tier 3
timeout_ms 60000
resources <decimal count>
resource <tier 0..3> <lowercase-hex UTF-8 bytes>
...
end
```

After parsing the complete prompt and presenting the native hardware-key
challenge, the helper must first write exactly `ready\n`. It may then write
exactly `approve hardware-key\n` only after the physical authenticator policy
succeeds, or `deny\n` after denial. The provider starts a fresh helper for every
exact request, so a response is accepted only on that request's private process
pipe and cannot be replayed into a later request.

Missing acknowledgement, early exit, partial or malformed response, blocked
prompt delivery, launch/I/O failure, or any non-Tier-3 request is an adapter
error. An acknowledged helper that remains pending through the supplied deadline
returns `TimedOut`; Trust Checker treats every Tier 3 timeout as denial. The
provider supplies only `CHIEF_APPROVAL_PROTOCOL=3` in the child environment.
Helpers must not spawn descendants that retain the protocol pipes.

## Validation

```sh
cargo test -p chief-of-staff-hardware-key-approval -- --nocapture
cargo clippy -p chief-of-staff-hardware-key-approval --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-hardware-key-approval --no-deps
```
