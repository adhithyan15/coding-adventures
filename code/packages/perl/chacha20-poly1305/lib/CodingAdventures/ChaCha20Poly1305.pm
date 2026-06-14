package CodingAdventures::ChaCha20Poly1305;

# ============================================================================
# CodingAdventures::ChaCha20Poly1305 — ChaCha20-Poly1305 AEAD (RFC 8439)
# ============================================================================
#
# ChaCha20-Poly1305 is a modern authenticated encryption with associated data
# (AEAD) construction designed by Daniel J. Bernstein. It became a TLS 1.3
# mandatory cipher suite and is the preferred alternative to AES-GCM when
# hardware AES acceleration is unavailable (e.g., mobile devices, IoT).
#
# The construction combines two primitives:
#
#   1. ChaCha20  — a stream cipher that generates a keystream from a 256-bit
#                  key and 96-bit nonce. XOR the keystream with plaintext to
#                  encrypt; XOR again to decrypt (symmetric).
#
#   2. Poly1305  — a one-time MAC (message authentication code) that produces
#                  a 128-bit authentication tag. It operates in GF(2^130 - 5),
#                  a prime field that fits 16-byte blocks with one extra high bit.
#
# Together they provide:
#   - Confidentiality  — ChaCha20 keystream hides the plaintext
#   - Integrity        — Poly1305 tag detects any tampering with ciphertext
#   - Authenticity     — AAD (Additional Authenticated Data) is MAC'd but not
#                        encrypted, so headers can be authenticated without being
#                        hidden (e.g., IP addresses, protocol version numbers)
#
# Algorithm Flow (Encrypt)
# ────────────────────────
#
#   key (32 bytes) + nonce (12 bytes)
#          │
#          ├─→ ChaCha20 block(counter=0) → first 32 bytes = Poly1305 key
#          │
#          ├─→ ChaCha20(counter=1..n) XOR plaintext → ciphertext
#          │
#          └─→ Poly1305(AAD ‖ pad16 ‖ ciphertext ‖ pad16 ‖ len(AAD) ‖ len(CT))
#                              └──────────────────────────────────────────┘
#                                                = tag (16 bytes)
#
# Output: (ciphertext, tag)
#
# Algorithm Flow (Decrypt)
# ────────────────────────
#   Regenerate Poly1305 key and expected tag, then constant-time compare.
#   If tags match: XOR keystream with ciphertext to recover plaintext.
#   If tags differ: die — never return unauthenticated plaintext.
#
# Reference: RFC 8439, "ChaCha20 and Poly1305 for IETF Protocols"
#            https://www.rfc-editor.org/rfc/rfc8439

use strict;
use warnings;
use Math::BigInt lib => 'GMP,Pari,FastCalc';

our $VERSION = '0.1.0';

# ─────────────────────────────────────────────────────────────────────────────
# ChaCha20 Constants
# ─────────────────────────────────────────────────────────────────────────────
#
# The four magic constants are the ASCII bytes of "expand 32-byte k" — a
# nothing-up-my-sleeve number that proves the designers didn't choose them
# to create a backdoor.
#
#   0x61707865 = "expa"  (little-endian bytes: 65 78 70 61 → 'e','x','p','a')
#   0x3320646e = "nd 3"
#   0x79622d32 = "2-by"
#   0x6b206574 = "te k"
#
# Together: "expand 32-byte k" — a reminder that ChaCha20 uses a 32-byte key.
# (There is also a "expand 16-byte k" variant for 128-bit keys, not used here.)

use constant CONSTANTS => [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

# U32 is the bitmask used to keep all arithmetic within 32-bit unsigned range.
# Perl integers are 64-bit signed on most platforms, so we must mask after
# every add/shift to emulate 32-bit wrapping arithmetic.
use constant U32 => 0xFFFFFFFF;

# ─────────────────────────────────────────────────────────────────────────────
# _rotl32($x, $n) — Rotate left 32-bit integer
# ─────────────────────────────────────────────────────────────────────────────
#
# Left rotation by $n bits, modulo 32. Rotation (rather than shift) means bits
# shifted off the top wrap around to the bottom. This is essential for ChaCha20's
# diffusion: the quarter-round uses rotations of 16, 12, 8, and 7 bits.
#
# Example with 16-bit rotation for clarity (same idea):
#   0b1100_0000_0000_0001  rotl 1  →  0b1000_0000_0000_0011
#   The leading 1 wraps to bit 0.
#
# Implementation:
#   Left shift by n:    ($x << $n) & U32  — produces the upper bits
#   Right shift by n:   ($x >> (32 - $n)) — produces the lower bits
#   OR together to combine.

sub _rotl32 {
    my ($x, $n) = @_;
    return (($x << $n) | ($x >> (32 - $n))) & U32;
}

# ─────────────────────────────────────────────────────────────────────────────
# _quarter_round(\@state, $a, $b, $c, $d) — ChaCha20 quarter-round
# ─────────────────────────────────────────────────────────────────────────────
#
# The quarter-round is ChaCha20's core mixing function. It takes four 32-bit
# words from the state matrix and mixes them together using only:
#   - 32-bit addition (wrapping)
#   - XOR
#   - Left rotation
#
# This is the "ARX" (Add-Rotate-XOR) design philosophy. ARX ciphers are:
#   - Fast in software (no lookup tables needed)
#   - Immune to cache-timing attacks (no data-dependent memory access)
#   - Simple to implement correctly
#
# The four rotation amounts (16, 12, 8, 7) were chosen to maximize diffusion
# while keeping the cipher fast. Bernstein proved these constants make the
# cipher's mixing properties optimal.
#
# One quarter-round step:
#
#   a += b;  d ^= a;  d <<<= 16;
#   c += d;  b ^= c;  b <<<= 12;
#   a += b;  d ^= a;  d <<<= 8;
#   c += d;  b ^= c;  b <<<= 7;
#
# After one quarter-round, every output bit depends on every input bit
# (complete diffusion through those four words).

sub _quarter_round {
    my ($s, $a, $b, $c, $d) = @_;
    $s->[$a] = ($s->[$a] + $s->[$b]) & U32; $s->[$d] ^= $s->[$a]; $s->[$d] = _rotl32($s->[$d], 16);
    $s->[$c] = ($s->[$c] + $s->[$d]) & U32; $s->[$b] ^= $s->[$c]; $s->[$b] = _rotl32($s->[$b], 12);
    $s->[$a] = ($s->[$a] + $s->[$b]) & U32; $s->[$d] ^= $s->[$a]; $s->[$d] = _rotl32($s->[$d], 8);
    $s->[$c] = ($s->[$c] + $s->[$d]) & U32; $s->[$b] ^= $s->[$c]; $s->[$b] = _rotl32($s->[$b], 7);
}

# ─────────────────────────────────────────────────────────────────────────────
# chacha20_block($class, $key, $counter, $nonce) — Generate one 64-byte block
# ─────────────────────────────────────────────────────────────────────────────
#
# The ChaCha20 state is a 4×4 matrix of 32-bit words (16 words = 64 bytes):
#
#   ┌─────────────────────────────────────────────────┐
#   │  cccccccc  cccccccc  cccccccc  cccccccc          │  ← 4 constants (words 0-3)
#   │  kkkkkkkk  kkkkkkkk  kkkkkkkk  kkkkkkkk          │  ← key words 0-3 (words 4-7)
#   │  kkkkkkkk  kkkkkkkk  kkkkkkkk  kkkkkkkk          │  ← key words 4-7 (words 8-11)
#   │  bbbbbbbb  nnnnnnnn  nnnnnnnn  nnnnnnnn          │  ← block counter + nonce (12-15)
#   └─────────────────────────────────────────────────┘
#
# (c=constant, k=key word, b=block counter, n=nonce word)
#
# The block function runs 20 rounds (10 "double-rounds"). Each double-round
# applies the quarter-round to the four columns, then to the two diagonals:
#
#   Column round:    QR(0,4,8,12)  QR(1,5,9,13)  QR(2,6,10,14)  QR(3,7,11,15)
#   Diagonal round:  QR(0,5,10,15) QR(1,6,11,12) QR(2,7,8,13)   QR(3,4,9,14)
#
# After 20 rounds, each output word is added back to the corresponding input
# word (this prevents the cipher from being invertible — you can't recover the
# key by reversing the rounds without this final addition).
#
# The counter word allows generating up to 2^32 × 64 = 256 GiB per (key,nonce)
# pair — each counter value produces a different 64-byte keystream block.
#
# Parameters:
#   $key     — 32 bytes (256-bit key), packed binary string
#   $counter — 32-bit block counter, integer
#   $nonce   — 12 bytes (96-bit nonce), packed binary string
#
# Returns: 64-byte keystream block, packed binary string

sub chacha20_block {
    my ($class, $key, $counter, $nonce) = @_;

    # Validate key and nonce lengths before unpacking.
    # unpack() on a too-short string silently produces zeroed words, which would
    # be catastrophic for security: different (key,nonce) pairs could produce
    # identical keystreams if both are silently padded to zero.
    die "ChaCha20 key must be exactly 32 bytes\n" unless length($key) == 32;
    die "ChaCha20 nonce must be exactly 12 bytes\n" unless length($nonce) == 12;

    # Unpack the key into 8 little-endian 32-bit words.
    # 'V8' means 8 unsigned 32-bit LE integers.
    my @key_words   = unpack('V8', $key);

    # Unpack the nonce into 3 little-endian 32-bit words.
    my @nonce_words = unpack('V3', $nonce);

    # Build the initial 16-word state matrix (RFC 8439 §2.3):
    #   words 0-3:   "expand 32-byte k" constants
    #   words 4-11:  key (8 × 32-bit LE words)
    #   word 12:     block counter
    #   words 13-15: nonce (3 × 32-bit LE words)
    my @initial = (@{CONSTANTS()}, @key_words, $counter, @nonce_words);

    # Copy for the mixing rounds. We need @initial unchanged for the final add.
    my @state = @initial;

    # 10 double-rounds = 20 rounds total
    for (1..10) {
        # Column round — mixes each of the four columns of the 4×4 matrix
        _quarter_round(\@state, 0, 4,  8, 12);
        _quarter_round(\@state, 1, 5,  9, 13);
        _quarter_round(\@state, 2, 6, 10, 14);
        _quarter_round(\@state, 3, 7, 11, 15);

        # Diagonal round — mixes the four diagonals of the 4×4 matrix
        # This is what distinguishes ChaCha20 from Salsa20: using shifted
        # diagonals instead of rows ensures every column word is mixed with
        # every row word within one double-round.
        _quarter_round(\@state,  0, 5, 10, 15);
        _quarter_round(\@state,  1, 6, 11, 12);
        _quarter_round(\@state,  2, 7,  8, 13);
        _quarter_round(\@state,  3, 4,  9, 14);
    }

    # Final step: add the initial state back (word-by-word, wrapping mod 2^32).
    # This "feed-forward" addition makes the block function non-invertible:
    # even if an attacker knows the output, they cannot reverse the 20 rounds
    # without guessing the key. The resulting 16 words are the keystream block.
    my @final = map { ($state[$_] + $initial[$_]) & U32 } 0..15;

    # Serialize as 16 little-endian 32-bit words = 64 bytes
    return pack('V16', @final);
}

# ─────────────────────────────────────────────────────────────────────────────
# chacha20_encrypt($class, $plaintext, $key, $nonce, $counter) — Encrypt/decrypt
# ─────────────────────────────────────────────────────────────────────────────
#
# ChaCha20 is a stream cipher: encryption and decryption are identical.
# To encrypt: XOR plaintext with keystream.
# To decrypt: XOR ciphertext with keystream (same operation, same function).
#
# The keystream is generated in 64-byte blocks. For the final block, only
# the first len(remaining) bytes are used; the rest are discarded.
#
# The $counter starts at 1 for data encryption (per RFC 8439 §2.6), reserving
# counter=0 for generating the Poly1305 one-time key.
#
# Parameters:
#   $plaintext — the bytes to encrypt (or ciphertext to decrypt)
#   $key       — 32-byte key
#   $nonce     — 12-byte nonce
#   $counter   — starting block counter (default 1)
#
# Returns: encrypted (or decrypted) bytes, same length as input

sub chacha20_encrypt {
    my ($class, $plaintext, $key, $nonce, $counter) = @_;
    $counter //= 1;

    # Empty input → empty output (no blocks needed)
    return '' unless length($plaintext);

    my $result = '';
    my $offset = 0;

    while ($offset < length($plaintext)) {
        # Generate the next 64-byte keystream block for this counter value
        my $keystream = $class->chacha20_block($key, $counter, $nonce);

        # Take up to 64 bytes of plaintext (the final chunk may be shorter)
        my $chunk = substr($plaintext, $offset, 64);

        # XOR the chunk with the first len(chunk) bytes of keystream.
        # Perl's ^ operator XORs two strings byte-by-byte and returns a string
        # of length equal to the shorter operand — perfect for partial blocks.
        $result .= $chunk ^ substr($keystream, 0, length($chunk));

        $offset += 64;
        $counter++;
    }

    return $result;
}

# ─────────────────────────────────────────────────────────────────────────────
# poly1305_mac($class, $message, $key) — Compute Poly1305 authentication tag
# ─────────────────────────────────────────────────────────────────────────────
#
# Poly1305 is a one-time MAC operating over the finite field GF(2^130 - 5).
# The prime p = 2^130 - 5 was chosen because:
#   - It is just above 2^128, so 128-bit (16-byte) blocks fit with room for a
#     one-high-bit padding sentinel.
#   - It has efficient reduction: (2^130 ≡ 5 mod p), so reducing mod p requires
#     only a multiply-by-5 and subtract, no expensive division.
#
# The 32-byte one-time key splits into two 128-bit parts:
#   r = key[0..15]   — the "rate" (clamped to avoid weak keys)
#   s = key[16..31]  — the "nonce addition" (secret one-time offset)
#
# Clamping r ensures it has the form needed for a valid polynomial hash:
#   CLAMP = 0x0ffffffc0ffffffc0ffffffc0fffffff
#   r &= CLAMP
# This zeros out 4 bits in bytes 3, 7, 11, and 2 bits in byte 15. Without
# clamping, certain r values would make the MAC trivially breakable.
#
# The accumulator a starts at 0. For each 16-byte block of the message:
#   1. Append a 0x01 byte after the last byte of the block (i.e., set bit at
#      position 8*len). This pads the block to 17 bytes (at most 130 bits).
#   2. Interpret the 17-byte value as a little-endian integer n.
#   3. a = (a + n) * r  mod  (2^130 - 5)
#
# After all blocks: tag = (a + s) mod 2^128, serialized as 16 LE bytes.
#
# Why "one-time"? The security proof requires that r and s are never reused
# with different messages. Here, r and s are derived per-message from the
# ChaCha20 keystream (counter=0), so they are never reused.
#
# We use Math::BigInt for the 130-bit arithmetic. Poly1305 numbers are too
# large for native 64-bit integers (they can be up to 130 bits wide).
#
# Parameters:
#   $message — the bytes to authenticate
#   $key     — 32-byte one-time key (r ‖ s), packed binary string
#
# Returns: 16-byte authentication tag

sub poly1305_mac {
    my ($class, $message, $key) = @_;

    # Validate: key must be exactly 32 bytes.
    # A shorter key would silently pad r and s with zeroes, producing a
    # trivially forgeable MAC (r=0 makes the accumulator always 0).
    die "Poly1305 key must be exactly 32 bytes\n" unless length($key) == 32;

    # p = 2^130 - 5 : the Poly1305 prime modulus
    my $PRIME = Math::BigInt->new(2)->bpow(130)->bsub(5);

    # CLAMP mask for r: zeros out specific bits to prevent weak keys.
    # Hex: 0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF
    # Binary breakdown (LE byte order):
    #   byte  3: top 2 bits cleared (& 0xFC)
    #   bytes 7, 11: top 2 bits cleared
    #   byte 15: top 4 bits cleared (& 0x0F)
    my $CLAMP = Math::BigInt->from_hex('0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF');

    # 2^128 for final reduction of (a + s)
    my $MOD128 = Math::BigInt->new(2)->bpow(128);

    # ── Parse r from key bytes 0..15 (little-endian) ──────────────────────
    # Little-endian means byte 0 is the least-significant byte.
    my @r_bytes = unpack('C16', substr($key, 0, 16));
    my $r = Math::BigInt->new(0);
    for my $i (0..15) {
        $r->bior(Math::BigInt->new($r_bytes[$i])->blsft(8 * $i));
    }
    $r->band($CLAMP);  # Apply the clamp

    # ── Parse s from key bytes 16..31 (little-endian) ─────────────────────
    my @s_bytes = unpack('C16', substr($key, 16, 16));
    my $s = Math::BigInt->new(0);
    for my $i (0..15) {
        $s->bior(Math::BigInt->new($s_bytes[$i])->blsft(8 * $i));
    }

    # ── Process message in 16-byte blocks ─────────────────────────────────
    my $acc    = Math::BigInt->new(0);
    my $offset = 0;

    while ($offset < length($message)) {
        my $chunk = substr($message, $offset, 16);
        my $len   = length($chunk);

        # Build the block integer from chunk bytes (little-endian)
        my @bytes = unpack('C*', $chunk);
        my $n     = Math::BigInt->new(0);
        for my $i (0..$#bytes) {
            $n->bior(Math::BigInt->new($bytes[$i])->blsft(8 * $i));
        }

        # Append the mandatory 0x01 sentinel bit above the last byte.
        # For a full 16-byte block: bit at position 128 (= 0x01 at byte 16).
        # For a partial block of length L: bit at position 8*L.
        # This distinguishes blocks of different lengths and prevents
        # length-extension attacks.
        $n->bior(Math::BigInt->new(1)->blsft(8 * $len));

        # a = (a + n) * r  mod  p
        $acc = $acc->badd($n)->bmul($r)->bmod($PRIME);

        $offset += 16;
    }

    # ── Final step: a = (a + s) mod 2^128 ────────────────────────────────
    # Add s (the one-time secret offset), then reduce mod 2^128 to get a
    # 128-bit (16-byte) tag. Note: we use mod 2^128 here, not mod p.
    my $tag_int = $acc->badd($s)->bmod($MOD128);

    # ── Serialize tag as 16 little-endian bytes ───────────────────────────
    my $hex = $tag_int->as_hex;
    $hex =~ s/^0x//i;

    # Zero-pad to 32 hex characters (= 16 bytes)
    $hex = '0' x (32 - length($hex)) . $hex if length($hex) < 32;

    # Convert big-endian hex string to little-endian byte array:
    # byte 0 is the rightmost pair of hex digits (least significant byte).
    my @tag_bytes;
    for my $i (0..15) {
        push @tag_bytes, hex(substr($hex, 30 - 2 * $i, 2));
    }

    return pack('C16', @tag_bytes);
}

# ─────────────────────────────────────────────────────────────────────────────
# _pad16($data) — Pad data to a multiple of 16 bytes with zero bytes
# ─────────────────────────────────────────────────────────────────────────────
#
# RFC 8439 §2.8 specifies that the Poly1305 input is constructed as:
#   AAD ‖ pad16(AAD) ‖ ciphertext ‖ pad16(ciphertext) ‖ len64(AAD) ‖ len64(CT)
#
# pad16 appends zero bytes until the total length is a multiple of 16.
# If the data is already a multiple of 16, no padding is added.
# The zeros are part of the MAC input — they prevent Poly1305 from treating
# "hello" + "world" the same as "hellowor" + "ld".

sub _pad16 {
    my $data = shift;
    my $rem  = length($data) % 16;
    return $rem == 0 ? $data : $data . "\x00" x (16 - $rem);
}

# ─────────────────────────────────────────────────────────────────────────────
# _constant_time_eq($a, $b) — Constant-time string comparison
# ─────────────────────────────────────────────────────────────────────────────
#
# Standard string comparison (eq) short-circuits as soon as it finds a
# difference. This creates a timing side-channel: an attacker who measures
# how long tag verification takes can learn which prefix of the tag is correct,
# eventually reconstructing the full tag byte by byte.
#
# Constant-time comparison runs for the same duration regardless of where the
# first difference is:
#   1. Check lengths with a plain equality check (lengths are not secret).
#   2. OR together the XOR of each byte pair into $diff.
#   3. If any byte differs, $diff will be non-zero → return 0 (not equal).
#
# The key property: every byte is always XOR'd, even after the first mismatch.
# An attacker's timing measurement sees no difference between a 1-byte match
# and a 16-byte match.
#
# This is critical for AEAD: if verification returned early on mismatch, an
# attacker could perform a padding-oracle-style attack against the MAC.

sub _constant_time_eq {
    my ($a, $b) = @_;
    return 0 if length($a) != length($b);
    my $diff = 0;
    $diff |= ord(substr($a, $_, 1)) ^ ord(substr($b, $_, 1)) for 0 .. length($a) - 1;
    return $diff == 0;
}

# ─────────────────────────────────────────────────────────────────────────────
# aead_encrypt($class, $plaintext, $key, $nonce, $aad) — AEAD encryption
# ─────────────────────────────────────────────────────────────────────────────
#
# Constructs the full ChaCha20-Poly1305 AEAD as specified in RFC 8439 §2.8.
#
# The MAC input is structured to prevent any ambiguity about what was
# authenticated:
#
#   ┌──────────────────┬──────────────────┬────────────┬────────────┐
#   │  AAD (padded     │  Ciphertext      │  len(AAD)  │  len(CT)   │
#   │  to 16 bytes)    │  (padded to 16)  │  8 bytes LE│  8 bytes LE│
#   └──────────────────┴──────────────────┴────────────┴────────────┘
#
# The length fields ensure that even if AAD and ciphertext together produce
# the same total bytes, different splits are distinguishable.
#
# Returns: ($ciphertext, $tag) — both packed binary strings
#   $ciphertext has the same length as $plaintext
#   $tag is always exactly 16 bytes

sub aead_encrypt {
    my ($class, $plaintext, $key, $nonce, $aad) = @_;

    # Step 1: Generate the Poly1305 one-time key using ChaCha20 with counter=0.
    # We only need the first 32 bytes of the 64-byte block.
    my $poly_key  = substr($class->chacha20_block($key, 0, $nonce), 0, 32);

    # Step 2: Encrypt the plaintext with ChaCha20 starting at counter=1.
    # Counter 0 was consumed by the Poly1305 key generation.
    my $ciphertext = $class->chacha20_encrypt($plaintext, $key, $nonce, 1);

    # Step 3: Build the Poly1305 MAC input (RFC 8439 §2.8):
    #   pad16(AAD) ‖ pad16(ciphertext) ‖ LE64(len(AAD)) ‖ LE64(len(ciphertext))
    #
    # pack('Q<', N) encodes N as a little-endian 64-bit unsigned integer.
    # This requires Perl built with 64-bit int support (standard since Perl 5.8).
    my $mac_data = _pad16($aad)
                 . _pad16($ciphertext)
                 . pack('Q<', length($aad))
                 . pack('Q<', length($ciphertext));

    # Step 4: Compute the Poly1305 tag over the MAC input
    my $tag = $class->poly1305_mac($mac_data, $poly_key);

    return ($ciphertext, $tag);
}

# ─────────────────────────────────────────────────────────────────────────────
# aead_decrypt($class, $ciphertext, $key, $nonce, $aad, $tag) — AEAD decryption
# ─────────────────────────────────────────────────────────────────────────────
#
# Decrypts and verifies the ciphertext. The verify-then-decrypt order is
# mandatory — we must NEVER return plaintext if the tag is invalid.
# Returning unauthenticated plaintext is a serious security vulnerability
# (see: "Cryptographic Doom Principle" by Moxie Marlinspike).
#
# Steps:
#   1. Regenerate the Poly1305 one-time key (same as during encryption)
#   2. Recompute the expected tag over AAD ‖ ciphertext
#   3. Constant-time compare expected tag with provided tag
#   4. If tags match: decrypt; if not: die with an error
#
# Parameters:
#   $ciphertext — the encrypted bytes
#   $key        — 32-byte key
#   $nonce      — 12-byte nonce
#   $aad        — additional authenticated data (must match what was used to encrypt)
#   $tag        — 16-byte authentication tag (from aead_encrypt)
#
# Returns: plaintext bytes
# Dies: "Authentication tag mismatch\n" if the tag is invalid

sub aead_decrypt {
    my ($class, $ciphertext, $key, $nonce, $aad, $tag) = @_;

    # Regenerate the Poly1305 one-time key (identical to encryption step 1)
    my $poly_key = substr($class->chacha20_block($key, 0, $nonce), 0, 32);

    # Reconstruct the MAC input (identical to encryption step 3)
    my $mac_data = _pad16($aad)
                 . _pad16($ciphertext)
                 . pack('Q<', length($aad))
                 . pack('Q<', length($ciphertext));

    # Compute the expected tag and compare in constant time
    my $expected_tag = $class->poly1305_mac($mac_data, $poly_key);

    # SECURITY: Always use constant-time comparison for MAC tags.
    # Variable-time comparison (eq / cmp) leaks timing information that
    # can be exploited to forge authentication tags.
    die "Authentication tag mismatch\n" unless _constant_time_eq($expected_tag, $tag);

    # Tag is valid — decrypt the ciphertext (ChaCha20 is its own inverse)
    return $class->chacha20_encrypt($ciphertext, $key, $nonce, 1);
}

1;

__END__

=head1 NAME

CodingAdventures::ChaCha20Poly1305 - ChaCha20-Poly1305 AEAD cipher (RFC 8439)

=head1 SYNOPSIS

    use CodingAdventures::ChaCha20Poly1305;

    my $C     = 'CodingAdventures::ChaCha20Poly1305';
    my $key   = "\x80" x 32;   # 32-byte key (use a cryptographically random key)
    my $nonce = "\x07" x 12;   # 12-byte nonce (never reuse with the same key!)
    my $aad   = "header data";  # Additional authenticated data (not encrypted)

    # Encrypt
    my ($ciphertext, $tag) = $C->aead_encrypt("Hello, world!", $key, $nonce, $aad);

    # Decrypt (dies if tag is invalid)
    my $plaintext = $C->aead_decrypt($ciphertext, $key, $nonce, $aad, $tag);

=head1 DESCRIPTION

Pure-Perl implementation of ChaCha20-Poly1305 Authenticated Encryption with
Associated Data (AEAD) as specified in RFC 8439. The construction provides:

=over 4

=item * Confidentiality via ChaCha20 stream cipher encryption

=item * Integrity and authenticity via Poly1305 one-time MAC

=item * Authentication of Additional Authenticated Data (AAD) without encrypting it

=back

=head1 SECURITY NOTES

=over 4

=item * B<Never reuse a (key, nonce) pair.> Poly1305 is a one-time MAC —
reusing the nonce with the same key completely breaks security.

=item * B<Use a cryptographically secure random key.> Keys must be 32 bytes
of random data from a CSPRNG.

=item * B<The nonce may be a counter> if the counter never wraps, or random
if the message count is low enough to avoid collision (96-bit nonce → safe
for up to ~2^32 messages per key with random nonces).

=back

=head1 METHODS

=head2 aead_encrypt($class, $plaintext, $key, $nonce, $aad)

Encrypt $plaintext and authenticate it along with $aad.

Returns a list: (C<$ciphertext>, C<$tag>).
C<$ciphertext> is the same length as C<$plaintext>.
C<$tag> is always 16 bytes.

=head2 aead_decrypt($class, $ciphertext, $key, $nonce, $aad, $tag)

Verify C<$tag> and decrypt C<$ciphertext>. Returns the plaintext.
Dies with "Authentication tag mismatch\n" if the tag is invalid.

=head2 chacha20_block($class, $key, $counter, $nonce)

Generate one 64-byte ChaCha20 keystream block. Useful for low-level testing.

=head2 chacha20_encrypt($class, $plaintext, $key, $nonce, $counter)

Encrypt/decrypt using ChaCha20 stream cipher. Counter defaults to 1.
Since ChaCha20 is a stream cipher, this function is its own inverse.

=head2 poly1305_mac($class, $message, $key)

Compute a Poly1305 MAC tag over $message using the 32-byte one-time $key.
Returns a 16-byte tag.

=head1 SEE ALSO

RFC 8439: L<https://www.rfc-editor.org/rfc/rfc8439>

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
