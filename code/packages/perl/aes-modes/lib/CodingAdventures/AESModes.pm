package CodingAdventures::AESModes;

# ============================================================================
# CodingAdventures::AESModes — AES Modes of Operation
# ============================================================================
#
# AES operates on fixed 128-bit (16-byte) blocks. To encrypt messages longer
# than one block, you need a "mode of operation" that defines how multiple
# block cipher calls chain together. The choice of mode critically affects
# security:
#
#   ECB — Electronic Codebook (INSECURE, educational only)
#   CBC — Cipher Block Chaining (legacy, vulnerable to padding oracles)
#   CTR — Counter mode (modern, stream cipher from block cipher)
#   GCM — Galois/Counter Mode (authenticated encryption, gold standard)
#
# Why do modes matter?
# ────────────────────
# A raw block cipher is a fixed-width permutation: 16 bytes in, 16 bytes out.
# Real messages are longer. If you encrypt each block independently (ECB),
# identical plaintext blocks produce identical ciphertext blocks — the famous
# "ECB penguin" shows image structure leaking through encryption.
#
# Dependencies: CodingAdventures::AES (provides aes_encrypt_block, aes_decrypt_block)

use strict;
use warnings;
no warnings 'portable';  # 64-bit hex constants used in GF(2^128) arithmetic

our $VERSION = '0.01';

use CodingAdventures::AES;

# ─────────────────────────────────────────────────────────────────────────────
# Utility: XOR two equal-length binary strings
#
# XOR is the fundamental building block of symmetric cryptography. CTR and GCM
# modes generate pseudorandom keystream via AES and XOR it with plaintext.
# ─────────────────────────────────────────────────────────────────────────────

sub _xor_bytes {
    my ($a, $b) = @_;
    return $a ^ $b;  # Perl's ^ on strings does byte-wise XOR
}

# ─────────────────────────────────────────────────────────────────────────────
# PKCS#7 Padding
#
# Block ciphers need input that is an exact multiple of the block size (16
# bytes for AES). PKCS#7 padding appends N bytes, each with value N, where
# N = 16 - (length mod 16). If the input is already aligned, a full 16-byte
# padding block is added (so the unpadder always has something to remove).
#
# Example: "HELLO" (5 bytes) → "HELLO" + 11 bytes of 0x0B
# Example: 16 bytes           → 16 bytes + 16 bytes of 0x10
# ─────────────────────────────────────────────────────────────────────────────

sub pkcs7_pad {
    my ($data) = @_;
    my $pad_len = 16 - (length($data) % 16);
    return $data . (chr($pad_len) x $pad_len);
}

sub pkcs7_unpad {
    my ($data) = @_;
    my $len = length($data);
    die "pkcs7_unpad: data must be non-empty and multiple of 16\n"
        unless $len > 0 && $len % 16 == 0;
    my $pad_val = ord(substr($data, -1));
    die "Invalid PKCS#7 padding\n"
        unless $pad_val >= 1 && $pad_val <= 16;
    # Constant-time padding validation: accumulate differences with OR
    # instead of returning early on the first mismatch (prevents timing attacks)
    my @bytes = unpack('C*', substr($data, $len - $pad_val));
    my $diff = 0;
    for my $b (@bytes) {
        $diff |= $b ^ $pad_val;
    }
    die "Invalid PKCS#7 padding\n" if $diff;
    return substr($data, 0, $len - $pad_val);
}

# ─────────────────────────────────────────────────────────────────────────────
# ECB — Electronic Codebook Mode (INSECURE)
#
# The simplest mode: encrypt each 16-byte block independently.
#
#   C[i] = AES_encrypt(P[i], key)
#
# ECB is deterministic: the same plaintext block always produces the same
# ciphertext block. This leaks patterns — NEVER use for real data.
# ─────────────────────────────────────────────────────────────────────────────

sub ecb_encrypt {
    my ($plaintext, $key) = @_;
    my $padded = pkcs7_pad($plaintext);
    my $ct = '';
    for (my $i = 0; $i < length($padded); $i += 16) {
        $ct .= CodingAdventures::AES::aes_encrypt_block(substr($padded, $i, 16), $key);
    }
    return $ct;
}

sub ecb_decrypt {
    my ($ciphertext, $key) = @_;
    my $len = length($ciphertext);
    die "ecb_decrypt: ciphertext must be non-empty multiple of 16\n"
        unless $len > 0 && $len % 16 == 0;
    my $pt = '';
    for (my $i = 0; $i < $len; $i += 16) {
        $pt .= CodingAdventures::AES::aes_decrypt_block(substr($ciphertext, $i, 16), $key);
    }
    return pkcs7_unpad($pt);
}

# ─────────────────────────────────────────────────────────────────────────────
# CBC — Cipher Block Chaining Mode
#
#   C[0] = IV
#   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
#
# Decryption:
#   P[i] = AES_decrypt(C[i], key) XOR C[i-1]
#
# Requires unpredictable IV. Vulnerable to padding oracle attacks.
# ─────────────────────────────────────────────────────────────────────────────

sub cbc_encrypt {
    my ($plaintext, $key, $iv) = @_;
    die "cbc_encrypt: IV must be 16 bytes\n" unless length($iv) == 16;
    my $padded = pkcs7_pad($plaintext);
    my $prev = $iv;
    my $ct = '';
    for (my $i = 0; $i < length($padded); $i += 16) {
        my $block = substr($padded, $i, 16);
        my $xored = _xor_bytes($block, $prev);
        my $encrypted = CodingAdventures::AES::aes_encrypt_block($xored, $key);
        $ct .= $encrypted;
        $prev = $encrypted;
    }
    return $ct;
}

sub cbc_decrypt {
    my ($ciphertext, $key, $iv) = @_;
    die "cbc_decrypt: IV must be 16 bytes\n" unless length($iv) == 16;
    my $len = length($ciphertext);
    die "cbc_decrypt: ciphertext must be non-empty multiple of 16\n"
        unless $len > 0 && $len % 16 == 0;
    my $prev = $iv;
    my $pt = '';
    for (my $i = 0; $i < $len; $i += 16) {
        my $block = substr($ciphertext, $i, 16);
        my $decrypted = CodingAdventures::AES::aes_decrypt_block($block, $key);
        $pt .= _xor_bytes($decrypted, $prev);
        $prev = $block;
    }
    return pkcs7_unpad($pt);
}

# ─────────────────────────────────────────────────────────────────────────────
# CTR — Counter Mode
#
# Turns block cipher into stream cipher:
#   keystream[i] = AES_encrypt(nonce_12 || counter_4_be, key)
#   C[i] = P[i] XOR keystream[i]
#
# No padding needed. Counter starts at 1 (GCM reserves 0 for tag).
# NEVER reuse (key, nonce) pair.
# ─────────────────────────────────────────────────────────────────────────────

sub _build_counter_block {
    my ($nonce, $counter) = @_;
    return $nonce . pack('N', $counter);
}

sub ctr_encrypt {
    my ($plaintext, $key, $nonce) = @_;
    die "ctr_encrypt: nonce must be 12 bytes\n" unless length($nonce) == 12;
    my $ct = '';
    my $counter = 1;
    my $len = length($plaintext);
    for (my $i = 0; $i < $len; $i += 16) {
        my $block_len = ($i + 16 <= $len) ? 16 : $len - $i;
        my $block = substr($plaintext, $i, $block_len);
        my $counter_block = _build_counter_block($nonce, $counter);
        my $keystream = CodingAdventures::AES::aes_encrypt_block($counter_block, $key);
        # XOR only as many bytes as we have
        $ct .= _xor_bytes($block, substr($keystream, 0, $block_len));
        $counter++;
    }
    return $ct;
}

# CTR decryption is identical to encryption
sub ctr_decrypt {
    my ($ciphertext, $key, $nonce) = @_;
    return ctr_encrypt($ciphertext, $key, $nonce);
}

# ─────────────────────────────────────────────────────────────────────────────
# GCM — Galois/Counter Mode (Authenticated Encryption)
#
# GCM = CTR encryption + GHASH polynomial MAC over GF(2^128).
#
# Algorithm:
#   1. H = AES_encrypt(0^128, key)       — hash subkey
#   2. J0 = IV || 0x00000001             — initial counter
#   3. CTR encrypt starting at J0+1
#   4. GHASH over padded AAD + padded CT + lengths
#   5. Tag = GHASH_result XOR AES_encrypt(J0, key)
#
# GF(2^128) uses polynomial x^128 + x^7 + x^2 + x + 1.
# Reduction constant R = 0xE1 << 120.
#
# We represent 128-bit values as two 64-bit integers stored in a 16-byte
# string, using Perl's pack/unpack with 'Q>' (big-endian 64-bit unsigned).
# ─────────────────────────────────────────────────────────────────────────────

# GF(2^128) multiplication: bit-by-bit algorithm.
# Inputs and output are 16-byte strings (big-endian 128-bit values).
sub _gf128_mul {
    my ($x, $y) = @_;
    my ($x_hi, $x_lo) = unpack('Q>Q>', $x);
    my ($v_hi, $v_lo) = unpack('Q>Q>', $y);
    my ($z_hi, $z_lo) = (0, 0);

    # R = 0xE100000000000000 || 0x0000000000000000
    my $r_hi = 0xE100000000000000;

    for my $i (0..127) {
        # Extract bit i of X (MSB-first)
        my ($word, $bit_pos);
        if ($i < 64) {
            $word = $x_hi;
            $bit_pos = 63 - $i;
        } else {
            $word = $x_lo;
            $bit_pos = 127 - $i;
        }
        if (($word >> $bit_pos) & 1) {
            $z_hi ^= $v_hi;
            $z_lo ^= $v_lo;
        }

        # Right-shift V by 1, conditionally XOR R
        my $carry = $v_lo & 1;
        # Use no warnings for overflow — Perl integers are 64-bit
        $v_lo = ($v_lo >> 1) & 0x7FFFFFFFFFFFFFFF;
        $v_lo |= ($v_hi & 1) << 63;
        $v_hi = ($v_hi >> 1) & 0x7FFFFFFFFFFFFFFF;
        if ($carry) {
            $v_hi ^= $r_hi;
        }
    }
    return pack('Q>Q>', $z_hi, $z_lo);
}

# GHASH: polynomial hash over GF(2^128).
# X[0] = 0; X[i] = (X[i-1] XOR block[i]) * H
sub _ghash {
    my ($h, $data) = @_;
    my $x = "\0" x 16;
    my $len = length($data);
    for (my $i = 0; $i < $len; $i += 16) {
        my $block = substr($data, $i, 16);
        if (length($block) < 16) {
            $block .= "\0" x (16 - length($block));
        }
        $x = _xor_bytes($x, $block);
        $x = _gf128_mul($x, $h);
    }
    return $x;
}

# Pad to multiple of 16 bytes
sub _gcm_pad {
    my ($data) = @_;
    my $rem = length($data) % 16;
    return $rem == 0 ? $data : $data . ("\0" x (16 - $rem));
}

sub gcm_encrypt {
    my ($plaintext, $key, $iv, $aad) = @_;
    die "gcm_encrypt: IV must be 12 bytes\n" unless length($iv) == 12;
    $aad //= '';

    # Step 1: Hash subkey H = AES(0^128, key)
    my $h = CodingAdventures::AES::aes_encrypt_block("\0" x 16, $key);

    # Step 2: J0 = IV || 0x00000001
    my $j0 = $iv . pack('N', 1);

    # Step 3: CTR encrypt starting at counter = 2
    my $ct = '';
    my $counter = 2;
    my $pt_len = length($plaintext);
    for (my $i = 0; $i < $pt_len; $i += 16) {
        my $block_len = ($i + 16 <= $pt_len) ? 16 : $pt_len - $i;
        my $block = substr($plaintext, $i, $block_len);
        my $counter_block = _build_counter_block($iv, $counter);
        my $keystream = CodingAdventures::AES::aes_encrypt_block($counter_block, $key);
        $ct .= _xor_bytes($block, substr($keystream, 0, $block_len));
        $counter++;
    }

    # Step 4: GHASH over AAD||pad||CT||pad||len_AAD||len_CT
    my $ghash_input = _gcm_pad($aad) . _gcm_pad($ct)
        . pack('Q>', length($aad) * 8) . pack('Q>', length($ct) * 8);
    my $tag_hash = _ghash($h, $ghash_input);

    # Step 5: Tag = GHASH XOR AES(J0, key)
    my $j0_enc = CodingAdventures::AES::aes_encrypt_block($j0, $key);
    my $tag = _xor_bytes($tag_hash, $j0_enc);

    return ($ct, $tag);
}

sub gcm_decrypt {
    my ($ciphertext, $key, $iv, $aad, $tag) = @_;
    die "gcm_decrypt: IV must be 12 bytes\n" unless length($iv) == 12;
    die "gcm_decrypt: tag must be 16 bytes\n" unless length($tag) == 16;
    $aad //= '';

    # Compute hash subkey
    my $h = CodingAdventures::AES::aes_encrypt_block("\0" x 16, $key);

    # Compute expected tag
    my $j0 = $iv . pack('N', 1);
    my $ghash_input = _gcm_pad($aad) . _gcm_pad($ciphertext)
        . pack('Q>', length($aad) * 8) . pack('Q>', length($ciphertext) * 8);
    my $tag_hash = _ghash($h, $ghash_input);
    my $j0_enc = CodingAdventures::AES::aes_encrypt_block($j0, $key);
    my $expected_tag = _xor_bytes($tag_hash, $j0_enc);

    # Constant-time-ish comparison
    my $diff = 0;
    my @t = unpack('C*', $tag);
    my @e = unpack('C*', $expected_tag);
    for my $i (0..15) {
        $diff |= $t[$i] ^ $e[$i];
    }
    if ($diff) {
        return (undef, "gcm_decrypt: authentication tag mismatch");
    }

    # Decrypt using CTR at counter = 2
    my $pt = '';
    my $counter = 2;
    my $ct_len = length($ciphertext);
    for (my $i = 0; $i < $ct_len; $i += 16) {
        my $block_len = ($i + 16 <= $ct_len) ? 16 : $ct_len - $i;
        my $block = substr($ciphertext, $i, $block_len);
        my $counter_block = _build_counter_block($iv, $counter);
        my $keystream = CodingAdventures::AES::aes_encrypt_block($counter_block, $key);
        $pt .= _xor_bytes($block, substr($keystream, 0, $block_len));
        $counter++;
    }

    return ($pt, undef);
}

1;

=head1 NAME

CodingAdventures::AESModes - AES modes of operation (ECB, CBC, CTR, GCM)

=head1 SYNOPSIS

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

=head1 VERSION

0.01

=cut
