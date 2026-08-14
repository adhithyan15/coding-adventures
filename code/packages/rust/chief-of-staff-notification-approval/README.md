# chief-of-staff-notification-approval

`chief-of-staff-notification-approval` is the D18 Tier 1 boundary between the
transport-independent Trust Checker and an operator-reviewed native notification
helper. The helper executable is selected by an absolute path, launched directly
without a shell, and receives no inherited environment. This lets macOS, Linux,
Windows, and companion-device deployments provide native notification UI without
putting platform UI code or secret values in the Chief daemon.

The provider writes this bounded UTF-8 protocol to standard input:

```text
CHIEF-TIER1-NOTIFICATION/1
request_id <lowercase-hex UTF-8 bytes>
requested_by <lowercase-hex UTF-8 bytes>
effective_tier 1
timeout_ms 5000
resources <decimal count>
resource <tier 0..3> <lowercase-hex UTF-8 bytes>
...
end
```

After parsing the complete prompt and presenting the notification, the helper
must first write exactly `ready\n`. It may then write exactly `approve\n` or
`deny\n`, or keep the acknowledged decision channel open through the supplied
deadline. Only that acknowledged live timeout returns the canonical Tier 1
timeout. Missing acknowledgement, early exit, partial or malformed response,
blocked prompt write, launch/I/O failure, or a Tier 2/3 request is an adapter
error and therefore fails closed in Trust Checker. The provider supplies only
`CHIEF_APPROVAL_PROTOCOL=1` in the child environment. Helpers must not spawn
descendants that retain the protocol pipes.

## Validation

```sh
cargo test -p chief-of-staff-notification-approval -- --nocapture
cargo clippy -p chief-of-staff-notification-approval --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-notification-approval --no-deps
```
