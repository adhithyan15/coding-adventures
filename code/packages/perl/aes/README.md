# CodingAdventures::AES

AES (Advanced Encryption Standard) block cipher — FIPS 197 — implemented from scratch in Perl for educational purposes.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Implements

- **AES-128, AES-192, AES-256** — all three key sizes, 10/12/14 rounds
- **Key schedule** — word-based expansion via RotWord, SubWord, Rcon
- **SubBytes / InvSubBytes** — S-box from GF(2^8) inverse + affine transform (polynomial 0x11B)
- **ShiftRows / InvShiftRows**, **MixColumns / InvMixColumns**
- **SBOX / INV_SBOX** — exported 256-element constant arrays

## Usage

```perl
use CodingAdventures::AES qw(aes_encrypt_block aes_decrypt_block);

my $key = pack('H*', '2b7e151628aed2a6abf7158809cf4f3c');
my $plain = pack('H*', '3243f6a8885a308d313198a2e0370734');

my $ct = CodingAdventures::AES::aes_encrypt_block($plain, $key);
my $pt = CodingAdventures::AES::aes_decrypt_block($ct, $key);
```
