package CodingAdventures::DES;

# ============================================================================
# CodingAdventures::DES — DES and Triple DES block cipher (FIPS 46-3)
# ============================================================================
#
# DES (Data Encryption Standard) was the world's first openly standardized
# encryption algorithm, published by NIST in 1977. It is now cryptographically
# broken (56-bit key space), but it remains essential study for:
#
#   1. Feistel networks — The structural innovation that lets encryption and
#      decryption share the same hardware. The round function f is never
#      inverted; just reverse the subkey order for decryption.
#
#   2. S-boxes — The only non-linear step. Without them, DES would be a
#      linear transformation solvable by Gaussian elimination over GF(2).
#
#   3. Key schedule — How a single 56-bit key expands into 16 × 48-bit
#      round subkeys via PC-1, left rotations, and PC-2.
#
# Algorithm Overview
# ──────────────────
#   plaintext (8 bytes)
#        │
#   IP (initial permutation)
#        │
#   ┌── 16 Feistel rounds ─────────────────────────────────┐
#   │   L_i = R_{i-1}                                       │
#   │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                 │
#   │   f: E(R) → XOR K → S-boxes → P                      │
#   └───────────────────────────────────────────────────────┘
#        │
#   FP = IP⁻¹ (swap halves first)
#        │
#   ciphertext (8 bytes)

use strict;
use warnings;

our $VERSION = '0.01';

# ─────────────────────────────────────────────────────────────────────────────
# Permutation Tables (1-indexed in the DES standard)
# ─────────────────────────────────────────────────────────────────────────────

my @IP = (
    58,50,42,34,26,18,10,2, 60,52,44,36,28,20,12,4,
    62,54,46,38,30,22,14,6, 64,56,48,40,32,24,16,8,
    57,49,41,33,25,17, 9,1, 59,51,43,35,27,19,11,3,
    61,53,45,37,29,21,13,5, 63,55,47,39,31,23,15,7
);

my @FP = (
    40, 8,48,16,56,24,64,32, 39, 7,47,15,55,23,63,31,
    38, 6,46,14,54,22,62,30, 37, 5,45,13,53,21,61,29,
    36, 4,44,12,52,20,60,28, 35, 3,43,11,51,19,59,27,
    34, 2,42,10,50,18,58,26, 33, 1,41, 9,49,17,57,25
);

my @PC1 = (
    57,49,41,33,25,17, 9,  1,58,50,42,34,26,18,
    10, 2,59,51,43,35,27, 19,11, 3,60,52,44,36,
    63,55,47,39,31,23,15,  7,62,54,46,38,30,22,
    14, 6,61,53,45,37,29, 21,13, 5,28,20,12, 4
);

my @PC2 = (
    14,17,11,24, 1, 5,  3,28,15, 6,21,10,
    23,19,12, 4,26, 8, 16, 7,27,20,13, 2,
    41,52,31,37,47,55, 30,40,51,45,33,48,
    44,49,39,56,34,53, 46,42,50,36,29,32
);

my @E = (
    32, 1, 2, 3, 4, 5,  4, 5, 6, 7, 8, 9,
     8, 9,10,11,12,13, 12,13,14,15,16,17,
    16,17,18,19,20,21, 20,21,22,23,24,25,
    24,25,26,27,28,29, 28,29,30,31,32, 1
);

my @P = (
    16, 7,20,21,29,12,28,17,
     1,15,23,26, 5,18,31,10,
     2, 8,24,14,32,27, 3, 9,
    19,13,30, 6,22,11, 4,25
);

my @SHIFTS = (1,1,2,2,2,2,2,2,1,2,2,2,2,2,2,1);

# ─────────────────────────────────────────────────────────────────────────────
# S-Boxes (8 × 64 entries; flat arrays, indexed row*16 + col)
# ─────────────────────────────────────────────────────────────────────────────

my @SBOXES = (
    [14, 4,13, 1, 2,15,11, 8, 3,10, 6,12, 5, 9, 0, 7,
      0,15, 7, 4,14, 2,13, 1,10, 6,12,11, 9, 5, 3, 8,
      4, 1,14, 8,13, 6, 2,11,15,12, 9, 7, 3,10, 5, 0,
     15,12, 8, 2, 4, 9, 1, 7, 5,11, 3,14,10, 0, 6,13],
    [15, 1, 8,14, 6,11, 3, 4, 9, 7, 2,13,12, 0, 5,10,
      3,13, 4, 7,15, 2, 8,14,12, 0, 1,10, 6, 9,11, 5,
      0,14, 7,11,10, 4,13, 1, 5, 8,12, 6, 9, 3, 2,15,
     13, 8,10, 1, 3,15, 4, 2,11, 6, 7,12, 0, 5,14, 9],
    [10, 0, 9,14, 6, 3,15, 5, 1,13,12, 7,11, 4, 2, 8,
     13, 7, 0, 9, 3, 4, 6,10, 2, 8, 5,14,12,11,15, 1,
     13, 6, 4, 9, 8,15, 3, 0,11, 1, 2,12, 5,10,14, 7,
      1,10,13, 0, 6, 9, 8, 7, 4,15,14, 3,11, 5, 2,12],
    [ 7,13,14, 3, 0, 6, 9,10, 1, 2, 8, 5,11,12, 4,15,
     13, 8,11, 5, 6,15, 0, 3, 4, 7, 2,12, 1,10,14, 9,
     10, 6, 9, 0,12,11, 7,13,15, 1, 3,14, 5, 2, 8, 4,
      3,15, 0, 6,10, 1,13, 8, 9, 4, 5,11,12, 7, 2,14],
    [ 2,12, 4, 1, 7,10,11, 6, 8, 5, 3,15,13, 0,14, 9,
     14,11, 2,12, 4, 7,13, 1, 5, 0,15,10, 3, 9, 8, 6,
      4, 2, 1,11,10,13, 7, 8,15, 9,12, 5, 6, 3, 0,14,
     11, 8,12, 7, 1,14, 2,13, 6,15, 0, 9,10, 4, 5, 3],
    [12, 1,10,15, 9, 2, 6, 8, 0,13, 3, 4,14, 7, 5,11,
     10,15, 4, 2, 7,12, 9, 5, 6, 1,13,14, 0,11, 3, 8,
      9,14,15, 5, 2, 8,12, 3, 7, 0, 4,10, 1,13,11, 6,
      4, 3, 2,12, 9, 5,15,10,11,14, 1, 7, 6, 0, 8,13],
    [ 4,11, 2,14,15, 0, 8,13, 3,12, 9, 7, 5,10, 6, 1,
     13, 0,11, 7, 4, 9, 1,10,14, 3, 5,12, 2,15, 8, 6,
      1, 4,11,13,12, 3, 7,14,10,15, 6, 8, 0, 5, 9, 2,
      6,11,13, 8, 1, 4,10, 7, 9, 5, 0,15,14, 2, 3,12],
    [13, 2, 8, 4, 6,15,11, 1,10, 9, 3,14, 5, 0,12, 7,
      1,15,13, 8,10, 3, 7, 4,12, 5, 6,11, 0,14, 9, 2,
      7,11, 4, 1, 9,12,14, 2, 0, 6,10,13,15, 3, 5, 8,
      2, 1,14, 7, 4,10, 8,13,15,12, 9, 0, 3, 5, 6,11],
);

# ─────────────────────────────────────────────────────────────────────────────
# Bit Manipulation Helpers
# ─────────────────────────────────────────────────────────────────────────────

# Convert an 8-byte string to an arrayref of 64 bits (MSB first per byte)
sub _bytes_to_bits {
    my ($s) = @_;
    my @bits;
    for my $byte (unpack('C*', $s)) {
        for my $i (reverse 0..7) {
            push @bits, ($byte >> $i) & 1;
        }
    }
    return \@bits;
}

# Convert an arrayref of bits (MSB first) back to a byte string
sub _bits_to_bytes {
    my ($bits) = @_;
    my @bytes;
    for (my $i = 0; $i < @$bits; $i += 8) {
        my $byte = 0;
        for my $j (0..7) { $byte = ($byte << 1) | $bits->[$i + $j]; }
        push @bytes, $byte;
    }
    return pack('C*', @bytes);
}

# Apply a permutation table (1-indexed positions) to a bits arrayref
sub _permute {
    my ($bits, $table) = @_;
    return [map { $bits->[$_ - 1] } @$table];
}

# Left-rotate a 28-element arrayref by n positions
sub _left_rotate {
    my ($half, $n) = @_;
    return [@{$half}[$n..27], @{$half}[0..$n-1]];
}

# ─────────────────────────────────────────────────────────────────────────────
# Key Schedule
# ─────────────────────────────────────────────────────────────────────────────

=head2 expand_key($key)

Derive 16 DES round subkeys from an 8-byte key string.
Returns an arrayref of 16 strings, each 6 bytes (48 bits).

=cut

sub expand_key {
    my ($key) = @_;
    die "DES key must be exactly 8 bytes, got " . length($key) . "\n"
        unless length($key) == 8;

    my $key_bits = _bytes_to_bits($key);
    my $permuted  = _permute($key_bits, \@PC1);  # 64 → 56 bits

    my $c = [@{$permuted}[0..27]];
    my $d = [@{$permuted}[28..55]];

    my @subkeys;
    for my $shift (@SHIFTS) {
        $c = _left_rotate($c, $shift);
        $d = _left_rotate($d, $shift);
        my $cd = [@$c, @$d];
        my $sk_bits = _permute($cd, \@PC2);  # 56 → 48 bits
        push @subkeys, _bits_to_bytes($sk_bits);
    }
    return \@subkeys;
}

# ─────────────────────────────────────────────────────────────────────────────
# Round Function f(R, K)
# ─────────────────────────────────────────────────────────────────────────────

sub _feistel_f {
    my ($right, $subkey) = @_;

    # Step 1: Expand R from 32 → 48 bits
    my $expanded = _permute($right, \@E);

    # Step 2: XOR with 48-bit subkey
    my $sk_bits = _bytes_to_bits($subkey);
    my @xored = map { $expanded->[$_] ^ $sk_bits->[$_] } 0..47;

    # Step 3: Apply 8 S-boxes
    my @sbox_out;
    for my $box (0..7) {
        my $offset = $box * 6;
        my @chunk  = @xored[$offset..$offset+5];
        my $row = $chunk[0] * 2 + $chunk[5];
        my $col = $chunk[1] * 8 + $chunk[2] * 4 + $chunk[3] * 2 + $chunk[4];
        my $val = $SBOXES[$box][$row * 16 + $col];
        push @sbox_out, ($val >> 3) & 1, ($val >> 2) & 1,
                        ($val >> 1) & 1,  $val & 1;
    }

    # Step 4: P permutation
    return _permute(\@sbox_out, \@P);
}

# ─────────────────────────────────────────────────────────────────────────────
# Core Block Cipher
# ─────────────────────────────────────────────────────────────────────────────

sub _des_block {
    my ($block, $subkeys) = @_;
    die "DES block must be exactly 8 bytes, got " . length($block) . "\n"
        unless length($block) == 8;

    my $bits = _bytes_to_bits($block);
    $bits = _permute($bits, \@IP);

    my @left  = @{$bits}[0..31];
    my @right = @{$bits}[32..63];

    for my $sk (@$subkeys) {
        my $f_out   = _feistel_f(\@right, $sk);
        my @new_right = map { $left[$_] ^ $f_out->[$_] } 0..31;
        @left  = @right;
        @right = @new_right;
    }

    my @combined = (@right, @left);  # swap before FP
    return _bits_to_bytes(_permute(\@combined, \@FP));
}

# ─────────────────────────────────────────────────────────────────────────────
# Public API
# ─────────────────────────────────────────────────────────────────────────────

=head2 des_encrypt_block($block, $key)

Encrypt a single 8-byte block with DES.

=cut

sub des_encrypt_block {
    my ($block, $key) = @_;
    my $subkeys = expand_key($key);
    return _des_block($block, $subkeys);
}

=head2 des_decrypt_block($block, $key)

Decrypt a single 8-byte block with DES.
(Decryption = encryption with subkeys in reverse order.)

=cut

sub des_decrypt_block {
    my ($block, $key) = @_;
    my $subkeys = expand_key($key);
    return _des_block($block, [reverse @$subkeys]);
}

# PKCS#7 padding: append N bytes each with value N (1 ≤ N ≤ 8)
sub _pkcs7_pad {
    my ($data) = @_;
    my $pad_len = 8 - (length($data) % 8);
    return $data . chr($pad_len) x $pad_len;
}

# Remove PKCS#7 padding; dies if invalid
sub _pkcs7_unpad {
    my ($data) = @_;
    die "Cannot unpad empty data\n" unless length($data) > 0;
    my $pad_len = ord(substr($data, -1));
    die "Invalid PKCS#7 padding byte: $pad_len\n"
        unless $pad_len >= 1 && $pad_len <= 8;
    die "Padding length exceeds data length\n"
        if length($data) < $pad_len;
    my $expected = chr($pad_len) x $pad_len;
    my $actual   = substr($data, -$pad_len);
    die "Invalid PKCS#7 padding bytes\n" unless $actual eq $expected;
    return substr($data, 0, length($data) - $pad_len);
}

=head2 des_ecb_encrypt($plaintext, $key)

Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).
WARNING: ECB mode is insecure for most purposes.

=cut

sub des_ecb_encrypt {
    my ($plaintext, $key) = @_;
    my $subkeys = expand_key($key);
    my $padded  = _pkcs7_pad($plaintext);
    my $result  = '';
    for (my $i = 0; $i < length($padded); $i += 8) {
        $result .= _des_block(substr($padded, $i, 8), $subkeys);
    }
    return $result;
}

=head2 des_ecb_decrypt($ciphertext, $key)

Decrypt variable-length ciphertext with DES in ECB mode.

=cut

sub des_ecb_decrypt {
    my ($ciphertext, $key) = @_;
    die "Cannot decrypt empty ciphertext\n" unless length($ciphertext) > 0;
    die "DES ECB ciphertext must be a multiple of 8 bytes, got "
        . length($ciphertext) . "\n"
        if length($ciphertext) % 8 != 0;
    my $subkeys  = expand_key($key);
    my $rev_subs = [reverse @$subkeys];
    my $result   = '';
    for (my $i = 0; $i < length($ciphertext); $i += 8) {
        $result .= _des_block(substr($ciphertext, $i, 8), $rev_subs);
    }
    return _pkcs7_unpad($result);
}

# ─────────────────────────────────────────────────────────────────────────────
# Triple DES (TDEA) — EDE ordering: E_K1(D_K2(E_K3(P)))
# ─────────────────────────────────────────────────────────────────────────────

=head2 tdea_encrypt_block($block, $k1, $k2, $k3)

Triple DES EDE encrypt: E_K1(D_K2(E_K3(block))).
When K1=K2=K3, reduces to single DES (backward compatibility).

=cut

sub tdea_encrypt_block {
    my ($block, $k1, $k2, $k3) = @_;
    my $sk1 = expand_key($k1);
    my $sk2 = expand_key($k2);
    my $sk3 = expand_key($k3);
    my $t = _des_block($block, $sk3);              # E_K3
    $t = _des_block($t, [reverse @$sk2]);          # D_K2
    return _des_block($t, $sk1);                   # E_K1
}

=head2 tdea_decrypt_block($block, $k1, $k2, $k3)

Triple DES EDE decrypt: D_K3(E_K2(D_K1(block))).

=cut

sub tdea_decrypt_block {
    my ($block, $k1, $k2, $k3) = @_;
    my $sk1 = expand_key($k1);
    my $sk2 = expand_key($k2);
    my $sk3 = expand_key($k3);
    my $t = _des_block($block, [reverse @$sk1]);   # D_K1
    $t = _des_block($t, $sk2);                     # E_K2
    return _des_block($t, [reverse @$sk3]);        # D_K3
}

1;

=head1 NAME

CodingAdventures::DES - DES and Triple DES block cipher (FIPS 46-3)

=head1 SYNOPSIS

    use CodingAdventures::DES;

    my $key   = pack('H*', '133457799BBCDFF1');
    my $plain = pack('H*', '0123456789ABCDEF');
    my $ct    = CodingAdventures::DES::des_encrypt_block($plain, $key);
    my $pt    = CodingAdventures::DES::des_decrypt_block($ct, $key);

=head1 DESCRIPTION

Educational implementation of DES and 3DES. Not for production use.

=head1 VERSION

0.01

=cut
