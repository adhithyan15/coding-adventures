# CodingAdventures::AESModes (Perl)

AES modes of operation: ECB, CBC, CTR, and GCM. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What This Package Does

AES operates on fixed 16-byte blocks. This package provides four modes of operation that extend AES to handle arbitrary-length messages:

| Mode | Security | Use Case |
|------|----------|----------|
| ECB  | INSECURE | Educational only — identical blocks produce identical ciphertext |
| CBC  | Legacy   | XOR-chains blocks; vulnerable to padding oracle attacks |
| CTR  | Modern   | Turns block cipher into stream cipher; parallelizable |
| GCM  | Best     | CTR + authentication tag; used in TLS 1.3 |

## Dependencies

- `CodingAdventures::AES` — AES block cipher (aes_encrypt_block / aes_decrypt_block)

## Usage

```perl
use CodingAdventures::AESModes;

# ECB (INSECURE — educational only)
my $ct = CodingAdventures::AESModes::ecb_encrypt($pt, $key);
my $pt = CodingAdventures::AESModes::ecb_decrypt($ct, $key);

# CBC
my $ct = CodingAdventures::AESModes::cbc_encrypt($pt, $key, $iv);
my $pt = CodingAdventures::AESModes::cbc_decrypt($ct, $key, $iv);

# CTR
my $ct = CodingAdventures::AESModes::ctr_encrypt($pt, $key, $nonce);
my $pt = CodingAdventures::AESModes::ctr_decrypt($ct, $key, $nonce);

# GCM (authenticated encryption)
my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, $aad);
my ($pt, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, $aad, $tag);
```

## Running Tests

```bash
perl Makefile.PL && make test
```
