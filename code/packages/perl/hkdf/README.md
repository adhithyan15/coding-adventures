# HKDF (Perl)

HMAC-based Extract-and-Expand Key Derivation Function, implementing [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869).

## What is HKDF?

HKDF derives cryptographic keys from input keying material (IKM) in two stages:

1. **Extract** — condenses IKM into a fixed-length pseudorandom key (PRK) using HMAC
2. **Expand** — stretches the PRK into output keying material (OKM) of any desired length

HKDF is used in TLS 1.3, Signal Protocol, WireGuard, and many other protocols.

## API

```perl
use CodingAdventures::HKDF qw(hkdf hkdf_extract hkdf_expand);

# Full HKDF (extract + expand combined)
my $okm = hkdf($salt, $ikm, $info, $length, 'sha256');

# Separate stages
my $prk = hkdf_extract($salt, $ikm, 'sha256');
my $okm = hkdf_expand($prk, $info, $length, 'sha256');

# Hex-output variants
my $hex = hkdf_hex($salt, $ikm, $info, $length, 'sha256');
my $hex = hkdf_extract_hex($salt, $ikm, 'sha256');
my $hex = hkdf_expand_hex($prk, $info, $length, 'sha256');
```

All functions accept `'sha256'` (default) or `'sha512'` as the hash algorithm. All return binary strings.

## Dependencies

- `CodingAdventures::HMAC` (which depends on SHA-256, SHA-512, MD5, SHA-1)

## Running Tests

```bash
PERL5LIB=../md5/lib:../sha1/lib:../sha256/lib:../sha512/lib:../hmac/lib prove -l -v t/
```
