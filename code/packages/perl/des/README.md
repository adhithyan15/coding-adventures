# CodingAdventures::DES (Perl)

DES and Triple DES (TDEA) block cipher — FIPS 46-3 / SP 800-67.

**Warning:** DES is cryptographically broken. Use for education only.

## Usage

```perl
use CodingAdventures::DES;

my $key   = pack('H*', '133457799BBCDFF1');
my $plain = pack('H*', '0123456789ABCDEF');
my $ct    = CodingAdventures::DES::des_encrypt_block($plain, $key);
my $pt    = CodingAdventures::DES::des_decrypt_block($ct, $key);

# ECB mode
my $ct2 = CodingAdventures::DES::des_ecb_encrypt("Hello, World!", $key);
my $pt2 = CodingAdventures::DES::des_ecb_decrypt($ct2, $key);
```

## Running Tests

```bash
prove -l -v t/
```
