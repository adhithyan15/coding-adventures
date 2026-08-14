# SE04 — XChaCha20-Poly1305

## Overview

XChaCha20-Poly1305 is the extended-nonce form of the RFC 8439
ChaCha20-Poly1305 authenticated-encryption construction. It keeps the 256-bit
key and 128-bit authentication tag from SE03, but expands the public nonce from
96 bits to 192 bits. A 24-byte nonce is large enough to sample from a secure
random source for long-lived keys with negligible accidental-collision risk.

This specification is the missing crypto profile for D18 Chief of Staff.
SE03 remains authoritative for the ChaCha20 quarter round, ChaCha20 stream
cipher, Poly1305 authenticator, AEAD padding, length framing, and constant-time
tag verification. SE04 adds only:

1. HChaCha20 subkey derivation from a 32-byte key and 16-byte nonce prefix.
2. XChaCha20 construction from HChaCha20 plus the RFC 8439 stream cipher.
3. XChaCha20-Poly1305 construction from HChaCha20 plus the RFC 8439 AEAD.

The construction follows `draft-irtf-cfrg-xchacha-03`. That document is an
expired Internet-Draft rather than an RFC. This repository therefore pins the
exact construction and vectors below instead of implying a standards status
the draft does not have.

## Where It Fits

```text
SE03 ChaCha20-Poly1305
  ├── ChaCha20 quarter round and 20-round permutation
  ├── RFC 8439 ChaCha20 with a 12-byte nonce
  ├── Poly1305
  └── RFC 8439 AEAD
          │
          ▼
SE04 XChaCha20-Poly1305
  ├── HChaCha20(key, nonce[0..16]) -> subkey
  ├── derived_nonce = 0x00000000 || nonce[16..24]
  └── RFC 8439 operation under the derived subkey and nonce
          │
          ├── D18 channel CMK grants
          ├── D18 channel messages
          └── Vault random-nonce encryption at rest
```

No new hash, MAC, field arithmetic, or external dependency is introduced.

## Parameters And Byte Order

| Value | Size | Rule |
| --- | ---: | --- |
| Key | 32 bytes | Uniform secret key. |
| XChaCha nonce | 24 bytes | Public and unique for the key. |
| HChaCha input | 16 bytes | First 16 bytes of the XChaCha nonce. |
| Derived RFC 8439 nonce | 12 bytes | Four zero bytes followed by the final eight XChaCha nonce bytes. |
| Tag | 16 bytes | Poly1305 tag from the RFC 8439 AEAD. |
| Block counter | 32-bit unsigned | Caller-selected for raw XChaCha20; RFC 8439 fixes AEAD use. |

Every 4-byte word loaded into or serialized from a ChaCha state is
little-endian. Byte-slice ranges in this document are half-open: `nonce[0..16]`
contains bytes 0 through 15, and `nonce[16..24]` contains bytes 16 through 23.

## HChaCha20

### Initial State

HChaCha20 uses the same 4-by-4 word state and quarter round as SE03. It replaces
the RFC 8439 block counter plus 12-byte nonce with one 16-byte nonce:

```text
state[ 0.. 4] = "expand 32-byte k" constants
state[ 4..12] = key[0..32] as eight little-endian words
state[12..16] = nonce16[0..16] as four little-endian words
```

In matrix form:

```text
constant  constant  constant  constant
key       key       key       key
key       key       key       key
nonce     nonce     nonce     nonce
```

There is no block counter in HChaCha20.

### Twenty Rounds

Apply ten ChaCha double rounds exactly as SE03 defines them:

```text
repeat 10 times:
    quarter_round(state, 0, 4,  8, 12)
    quarter_round(state, 1, 5,  9, 13)
    quarter_round(state, 2, 6, 10, 14)
    quarter_round(state, 3, 7, 11, 15)

    quarter_round(state, 0, 5, 10, 15)
    quarter_round(state, 1, 6, 11, 12)
    quarter_round(state, 2, 7,  8, 13)
    quarter_round(state, 3, 4,  9, 14)
```

### Subkey Extraction

HChaCha20 MUST NOT add the initial state back into the post-round state. This
absence of ChaCha's normal feed-forward is a defining part of HChaCha20.

Serialize the first and last rows of the post-round state in little-endian
order:

```text
subkey_words = state[0..4] || state[12..16]
subkey = little_endian_bytes(subkey_words)  // exactly 32 bytes
```

Returning the middle two rows, applying feed-forward, changing the word order,
or using a big-endian codec is a different and incompatible construction.

### Reference Pseudocode

```text
hchacha20(key32, nonce16):
    require len(key32) == 32
    require len(nonce16) == 16

    state = [
        0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
        LE32(key32[ 0.. 4]), LE32(key32[ 4.. 8]),
        LE32(key32[ 8..12]), LE32(key32[12..16]),
        LE32(key32[16..20]), LE32(key32[20..24]),
        LE32(key32[24..28]), LE32(key32[28..32]),
        LE32(nonce16[ 0.. 4]), LE32(nonce16[ 4.. 8]),
        LE32(nonce16[ 8..12]), LE32(nonce16[12..16]),
    ]

    repeat 10 ChaCha double rounds over state

    return LE32_BYTES(
        state[0], state[1], state[2], state[3],
        state[12], state[13], state[14], state[15]
    )
```

## XChaCha20

XChaCha20 converts the 24-byte nonce into a new key and an RFC 8439-compatible
12-byte nonce:

```text
xchacha20(key32, nonce24, input, counter = 0):
    require len(key32) == 32
    require len(nonce24) == 24

    subkey = hchacha20(key32, nonce24[0..16])
    derived_nonce = 0x00000000 || nonce24[16..24]
    return chacha20_encrypt(input, subkey, derived_nonce, counter)
```

Raw XChaCha20 is a stream cipher. It provides no authenticity and MUST NOT be
used for D18 messages, channel-key grants, Vault records, or any other surface
that requires authenticated encryption. Its public function exists for
conformance and for protocols that separately own a correct MAC composition.

## XChaCha20-Poly1305 AEAD

The AEAD derives the same subkey and nonce, then delegates without modification
to SE03's RFC 8439 AEAD:

```text
xchacha20_poly1305_encrypt(key32, nonce24, plaintext, aad):
    require len(key32) == 32
    require len(nonce24) == 24

    subkey = hchacha20(key32, nonce24[0..16])
    derived_nonce = 0x00000000 || nonce24[16..24]
    return chacha20_poly1305_encrypt(
        subkey,
        derived_nonce,
        plaintext,
        aad,
    )

xchacha20_poly1305_decrypt(key32, nonce24, ciphertext, aad, tag16):
    require len(key32) == 32
    require len(nonce24) == 24
    require len(tag16) == 16

    subkey = hchacha20(key32, nonce24[0..16])
    derived_nonce = 0x00000000 || nonce24[16..24]
    return chacha20_poly1305_decrypt(
        subkey,
        derived_nonce,
        ciphertext,
        aad,
        tag16,
    )
```

SE03's AEAD uses block counter 0 to derive the one-time Poly1305 key and block
counter 1 for payload encryption. An SE04 implementation MUST reuse those rules
instead of independently composing a MAC and stream cipher.

## Public API Contract

Names may follow language conventions, but each package must expose equivalent
typed operations:

| Operation | Inputs | Output |
| --- | --- | --- |
| `hchacha20_subkey` | 32-byte key, 16-byte nonce | 32-byte subkey |
| `xchacha20_encrypt` | bytes, 32-byte key, 24-byte nonce, `u32` counter | bytes of equal length |
| `xchacha20_poly1305_encrypt` | plaintext, 32-byte key, 24-byte nonce, AAD | ciphertext and 16-byte tag |
| `xchacha20_poly1305_decrypt` | ciphertext, 32-byte key, 24-byte nonce, AAD, 16-byte tag | plaintext or one authentication failure |

Fixed-size key, nonce, and tag types are preferred where a language can express
them. Dynamic-language implementations MUST validate exact lengths before any
round or allocation proportional to untrusted content.

Decryption has one public authentication-failure result. It MUST NOT reveal
whether the key, nonce, AAD, ciphertext, or tag was wrong. It MUST NOT return or
publish plaintext until the tag has been compared in constant time and accepted.

## Nonce Policy

XChaCha20-Poly1305 is not nonce-misuse resistant. Reusing the complete 24-byte
nonce with the same key repeats the derived subkey/nonce pair and destroys the
security assumptions inherited from ChaCha20-Poly1305.

Allowed nonce ownership models are:

1. **CSPRNG random nonce.** Sample all 24 bytes independently for each message.
   The draft estimates a 50% collision probability only near `2^96` messages
   under one key and a `2^-32` collision probability near `2^80` messages.
2. **Protocol-derived unique nonce.** Prove that the complete 24-byte value
   never repeats for the key. D18 channel messages use the canonical 16-byte
   channel UUID followed by the unsigned 64-bit big-endian durable sequence.

Forbidden behavior includes counters that reset after restart, truncating a
larger identity, silently substituting zero when randomness fails, or accepting
caller-supplied nonces without the owning protocol stating its uniqueness rule.

The D18 summary phrase “eliminates reuse risk” means that random collisions are
negligible at realistic volumes. It does not waive the uniqueness requirement.

## Security And Failure Rules

- Authenticate the exact AAD bytes supplied by the caller. Any AAD change must
  fail decryption.
- Compare all 16 tag bytes in constant time. Do not return on the first unequal
  byte.
- Reject a malformed key, nonce, or tag length before cryptographic work.
- Reject ciphertext or AAD lengths that cannot be represented by SE03's RFC
  8439 64-bit little-endian length fields.
- Do not expose the derived subkey, Poly1305 one-time key, keystream, plaintext,
  or tag-comparison intermediate in errors, logs, traces, or debug output.
- Zeroize the derived subkey and Poly1305 one-time key when the language/runtime
  provides a reliable repository-owned mechanism. Document honest degradation
  where it does not.
- Never invent plaintext recovery after authentication failure.
- Treat the Internet-Draft as a pinned construction reference, not as a final
  IETF standard.

## Test Vectors

### HChaCha20 — Draft Section 2.2.1

```text
key:
000102030405060708090a0b0c0d0e0f
101112131415161718191a1b1c1d1e1f

nonce16:
000000090000004a0000000031415927

expected subkey:
82413b4227b27bfed30e42508a877d73
a0f9e4d58a74a853c12ec41326d3ecdc
```

### XChaCha20-Poly1305 — Draft Appendix A.3.1

```text
key:
808182838485868788898a8b8c8d8e8f
909192939495969798999a9b9c9d9e9f

nonce24:
404142434445464748494a4b4c4d4e4f5051525354555657

aad:
50515253c0c1c2c3c4c5c6c7

plaintext (UTF-8):
Ladies and Gentlemen of the class of '99: If I could offer you only one
tip for the future, sunscreen would be it.

expected ciphertext:
bd6d179d3e83d43b9576579493c0e939
572a1700252bfaccbed2902c21396cbb
731c7f1b0b4aa6440bf3a82f4eda7e39
ae64c6708c54c216cb96b72e1213b452
2f8c9ba40db5d945b11b69b982c1bb9e
3f3fac2bc369488f76b2383565d3fff9
21f9664c97637da9768812f615c68b13
b52e

expected tag:
c0875924c1c7987947deafd8780acf49
```

The plaintext above is one byte string. The visual line break after “one” is
prose wrapping and MUST NOT be inserted into the vector.

## Required Tests

Every implementation must prove:

1. The HChaCha20 gold vector matches exactly.
2. The Appendix A.3.1 ciphertext and tag match exactly.
3. The gold ciphertext decrypts to the exact plaintext.
4. Flipping each tag byte causes the same authentication failure.
5. Ciphertext, AAD, nonce, and key changes fail authentication.
6. Empty and multi-block plaintexts round-trip.
7. Raw XChaCha20 is XOR-symmetric at counters 0 and 1.
8. Invalid key, nonce, and tag lengths fail before processing.
9. No plaintext is returned on authentication failure.
10. Cross-language fixtures encrypt in one implementation and decrypt in every
    other implementation with byte-identical results.

The versioned `se04-xchacha20-poly1305-v1` corpus added in #11623 closes item
10. All six package suites consume the same HChaCha20, raw XChaCha20, AEAD,
empty, multi-block, and mutation records.

## D18 Six-Language Matrix

| Language | Package | SE03 | SE04 |
| --- | --- | --- | --- |
| Python | `code/packages/python/chacha20-poly1305` | Complete | Complete in #11591 |
| Go | `code/packages/go/chacha20-poly1305` | Complete | Complete in #11593 |
| Ruby | `code/packages/ruby/chacha20-poly1305` | Complete | Complete in #11595 |
| TypeScript | `code/packages/typescript/chacha20-poly1305` | Complete | Complete in #11594 |
| Rust | `code/packages/rust/chacha20-poly1305` | Complete | Complete in PR #1029 |
| Elixir | `code/packages/elixir/chacha20-poly1305` | Complete | Complete in #11596 |

Issue #129 is complete with the six shared-fixture consumers in #11623. D19 is
the Actor model specification; it is not renamed or duplicated by this crypto
profile.

## Dependencies

- SE03 ChaCha20-Poly1305
- A source of unique nonces owned by the calling protocol
- No third-party runtime dependency

## References

- `code/specs/SE03-chacha20-poly1305.md`
- RFC 8439, ChaCha20 and Poly1305 for IETF Protocols:
  `https://www.rfc-editor.org/rfc/rfc8439`
- `draft-irtf-cfrg-xchacha-03`, XChaCha and
  AEAD_XChaCha20_Poly1305:
  `https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha-03`
