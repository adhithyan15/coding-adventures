# RSA PKCS #1 v1.5 SHA-256 Encoded Messages

## Status

Implemented as the zero-external-dependency Rust package
`rsa-pkcs1-v1-5-sha256`.

## Purpose

Validate one exact RFC 8017 EMSA-PKCS1-v1_5 encoded message for SHA-256 after
a separate repository-owned RSA public operation has produced the
modulus-width byte string. This is a bounded prerequisite for repository-owned
RSA certificate-signature verification; it is not RSA arithmetic and does not
authenticate the source of the encoded message.

For a caller-supplied 32-byte SHA-256 digest `H`, the only accepted form is:

```text
EM = 0x00 || 0x01 || PS || 0x00 || DigestInfo(H)
PS = 0xff repeated at least eight times

DigestInfo(H) =
  30 31
    30 0d
      06 09 60 86 48 01 65 03 04 02 01
      05 00
    04 20
      H
```

The fixed `DigestInfo` prefix is the canonical DER encoding of the SHA-256
AlgorithmIdentifier with present NULL parameters followed by a 32-octet
digest. No alternate BER length form, omitted NULL, alternate algorithm,
trailing byte, or embedded suffix is accepted.

## Input contract

The caller supplies:

- one complete fixed-width encoded message produced by a separate RSA public
  operation; and
- one exact 32-byte SHA-256 digest computed over the intended signed bytes by
  a separate repository-owned hashing boundary.

The encoded message must be at least 62 bytes: two header bytes, eight padding
bytes, one separator, and the 51-byte SHA-256 `DigestInfo`. The expected
`DigestInfo` begins at the position implied by the encoded message's total
length, so every preceding padding byte is required to be `0xff` and the
separator cannot be moved or discovered by an unbounded scan.

The digest is public verification input, not secret key material. The
implementation nevertheless examines all 32 digest bytes and combines their
differences before deciding whether they match; it does not report a mismatch
position.

## Public contract

```rust
pub const SHA256_DIGEST_LENGTH: usize = 32;
pub const SHA256_DIGEST_INFO_LENGTH: usize = 51;
pub const MINIMUM_PADDING_LENGTH: usize = 8;
pub const MINIMUM_ENCODED_MESSAGE_LENGTH: usize = 62;

pub struct ValidatedSha256EncodedMessage { /* private */ }

pub enum Sha256EncodedMessageErrorKind {
    EncodedMessageTooShort,
    InvalidBlockHeader,
    InvalidPadding,
    InvalidSeparator,
    InvalidDigestInfo,
    DigestMismatch,
}

pub struct Sha256EncodedMessageError { /* redacted private field */ }

pub fn validate_sha256_encoded_message(
    encoded_message: &[u8],
    expected_digest: &[u8; SHA256_DIGEST_LENGTH],
) -> Result<ValidatedSha256EncodedMessage, Sha256EncodedMessageError>;
```

`ValidatedSha256EncodedMessage` is opaque evidence that the complete supplied
byte string had the exact encoded-message shape and digest. Its construction
is private so callers cannot forge that evidence without passing every check.
It carries no bytes and grants no authority beyond this formatting and digest
comparison result.

## Limits and errors

Validation is allocation-free and linear in the public modulus-width input.
The fixed trailer makes every range calculation checked and deterministic.
There is no recursion, DER parser, registry, heap allocation, or input-retaining
error state.

Errors contain only one stable category. They retain and format no encoded
message, digest, mismatch index, padding length, certificate, key, signature,
provider, or trace data.

## Tests

Tests must cover:

- exact minimum-length and longer valid encoded messages;
- the RFC 8017 SHA-256 `DigestInfo` bytes and a known SHA-256 digest;
- inputs shorter than the 62-byte minimum, including empty input;
- either invalid block-header byte;
- padding shorter than eight bytes and every non-`0xff` padding position;
- a missing or displaced zero separator;
- every single-byte mutation of the fixed `DigestInfo` prefix;
- mismatches at the first, middle, and last digest byte;
- appended and truncated bytes that cannot be reinterpreted as valid padding;
- success evidence that retains no input; and
- stable redacted errors that expose no supplied bytes or positions.

## Deliberate exclusions

- RSA integer conversion, modular exponentiation, or modulus-width recovery;
- signature-length checks, public-key parsing, or strength policy;
- PSS, SHA-1, SHA-384, SHA-512, or a general algorithm registry;
- hashing the message or choosing which certificate bytes were signed;
- claiming that an arbitrary caller-supplied encoded message came from an RSA
  signature;
- certificate signature composition, path construction or validation, trust
  roots, hostname checks, clocks, revocation, TLS, HTTP, sockets, or OAuth
  transport.
