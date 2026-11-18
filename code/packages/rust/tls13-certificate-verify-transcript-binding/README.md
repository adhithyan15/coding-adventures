# tls13-certificate-verify-transcript-binding

`tls13-certificate-verify-transcript-binding` closes the detached transcript
hash boundary before repository-owned RSA-PSS `CertificateVerify` verification.
It accepts an immutable `SelectedTls13Transcript`, snapshots it internally,
maps its suite-bound SHA-256 or SHA-384 choice into the exact RFC 8446 framing
metadata, and constructs the caller-selected client or server signed input.

A second stage accepts only that bound input plus an already policy-authorized
RSA-PSS scheme. It delegates exact-input hashing to
`tls13-rsa-pss-certificate-verify` and retains both proof objects together.
Callers cannot inject detached transcript bytes or transcript-hash metadata,
cannot advance the selected transcript while the evidence exists, and cannot
mutate the caller-owned framed bytes while they are borrowed.

A third stage accepts only that outer transcript-bound hash evidence together
with the exact decoded `CertificateVerify` message, a validated RSA public key,
typed verifier limits, and caller-owned scratch. It delegates the RSA-PSS
operation to the existing repository-owned verifier and returns a non-copyable
verdict retaining both the complete outer proof and the lower signature proof.
The stronger verdict therefore cannot be constructed from detached framing or
hash evidence.

This package does not decide whether the selected transcript contains the
right ordered handshake messages or is at the right state. It does not
authorize the caller-selected endpoint role, parse or validate a certificate,
bind the validated key to a certificate or reviewed server path, check
Finished, process records, perform I/O, or make TLS, HTTPS, OAuth transport, or
live-provider access available.

```rust
use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
use tls13_certificate_verify_transcript_binding::bind_certificate_verify_transcript;
# use tls13_transcript::SelectedTls13Transcript;
# fn example(selected_transcript: &SelectedTls13Transcript) -> Result<(), Box<dyn std::error::Error>> {
let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
let bound = bind_certificate_verify_transcript(
    selected_transcript,
    CertificateVerifyRole::Server,
    &mut output,
)?;
assert_eq!(bound.snapshot(), selected_transcript.snapshot());
# Ok(())
# }
```

## Development

```bash
bash BUILD
```
