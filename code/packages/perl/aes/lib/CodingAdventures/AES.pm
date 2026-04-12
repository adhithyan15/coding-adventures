package CodingAdventures::AES;

# ============================================================================
# CodingAdventures::AES — AES block cipher (FIPS 197)
# ============================================================================
#
# AES (Advanced Encryption Standard) is the most widely deployed symmetric
# encryption algorithm in the world. Published by NIST in 2001 as FIPS 197,
# it replaced DES. AES is used in TLS/HTTPS, WPA2/WPA3, BitLocker, and
# virtually every modern secure protocol.
#
# AES is a Substitution-Permutation Network (SPN) — all 16 bytes of the
# state are transformed every round. This contrasts with DES's Feistel
# network, which only transforms half the state per round.
#
# Algorithm Overview
# ──────────────────
#   plaintext (16 bytes)
#        │
#   AddRoundKey(state, round_key[0])
#        │
#   ┌── Nr-1 full rounds ─────────────────────────────────────────┐
#   │   SubBytes   — GF(2^8) inverse + affine transform            │
#   │   ShiftRows  — cyclic row shifts                             │
#   │   MixColumns — GF(2^8) matrix multiply                      │
#   │   AddRoundKey                                                │
#   └──────────────────────────────────────────────────────────────┘
#        │
#   SubBytes + ShiftRows + AddRoundKey (final round, no MixColumns)
#        │
#   ciphertext (16 bytes)
#
# Key sizes: AES-128 (10 rounds), AES-192 (12 rounds), AES-256 (14 rounds)

use strict;
use warnings;

our $VERSION = '0.01';

# ─────────────────────────────────────────────────────────────────────────────
# GF(2^8) with AES polynomial 0x11B = x^8 + x^4 + x^3 + x + 1
# ─────────────────────────────────────────────────────────────────────────────

sub _xtime {
    my ($b) = @_;
    my $shifted = ($b << 1) & 0xFF;
    return ($b & 0x80) ? $shifted ^ 0x1B : $shifted;
}

sub _gf_mul {
    my ($a, $b) = @_;
    my $result = 0;
    my $aa = $a;
    for (1..8) {
        $result ^= $aa if $b & 1;
        $aa = _xtime($aa);
        $b >>= 1;
    }
    return $result;
}

# ─────────────────────────────────────────────────────────────────────────────
# AES S-box and Inverse S-box (FIPS 197, Figures 7 and 14)
# ─────────────────────────────────────────────────────────────────────────────

my @SBOX = (
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16
);

my @INV_SBOX = (
    0x52,0x09,0x6a,0xd5,0x30,0x36,0xa5,0x38,0xbf,0x40,0xa3,0x9e,0x81,0xf3,0xd7,0xfb,
    0x7c,0xe3,0x39,0x82,0x9b,0x2f,0xff,0x87,0x34,0x8e,0x43,0x44,0xc4,0xde,0xe9,0xcb,
    0x54,0x7b,0x94,0x32,0xa6,0xc2,0x23,0x3d,0xee,0x4c,0x95,0x0b,0x42,0xfa,0xc3,0x4e,
    0x08,0x2e,0xa1,0x66,0x28,0xd9,0x24,0xb2,0x76,0x5b,0xa2,0x49,0x6d,0x8b,0xd1,0x25,
    0x72,0xf8,0xf6,0x64,0x86,0x68,0x98,0x16,0xd4,0xa4,0x5c,0xcc,0x5d,0x65,0xb6,0x92,
    0x6c,0x70,0x48,0x50,0xfd,0xed,0xb9,0xda,0x5e,0x15,0x46,0x57,0xa7,0x8d,0x9d,0x84,
    0x90,0xd8,0xab,0x00,0x8c,0xbc,0xd3,0x0a,0xf7,0xe4,0x58,0x05,0xb8,0xb3,0x45,0x06,
    0xd0,0x2c,0x1e,0x8f,0xca,0x3f,0x0f,0x02,0xc1,0xaf,0xbd,0x03,0x01,0x13,0x8a,0x6b,
    0x3a,0x91,0x11,0x41,0x4f,0x67,0xdc,0xea,0x97,0xf2,0xcf,0xce,0xf0,0xb4,0xe6,0x73,
    0x96,0xac,0x74,0x22,0xe7,0xad,0x35,0x85,0xe2,0xf9,0x37,0xe8,0x1c,0x75,0xdf,0x6e,
    0x47,0xf1,0x1a,0x71,0x1d,0x29,0xc5,0x89,0x6f,0xb7,0x62,0x0e,0xaa,0x18,0xbe,0x1b,
    0xfc,0x56,0x3e,0x4b,0xc6,0xd2,0x79,0x20,0x9a,0xdb,0xc0,0xfe,0x78,0xcd,0x5a,0xf4,
    0x1f,0xdd,0xa8,0x33,0x88,0x07,0xc7,0x31,0xb1,0x12,0x10,0x59,0x27,0x80,0xec,0x5f,
    0x60,0x51,0x7f,0xa9,0x19,0xb5,0x4a,0x0d,0x2d,0xe5,0x7a,0x9f,0x93,0xc9,0x9c,0xef,
    0xa0,0xe0,0x3b,0x4d,0xae,0x2a,0xf5,0xb0,0xc8,0xeb,0xbb,0x3c,0x83,0x53,0x99,0x61,
    0x17,0x2b,0x04,0x7e,0xba,0x77,0xd6,0x26,0xe1,0x69,0x14,0x63,0x55,0x21,0x0c,0x7d
);

# Round constants (1-indexed; Rcon[i] = 2^(i-1) in GF(2^8))
my @RCON = (0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36,0x6c,0xd8,0xab,0x4d);

# Public accessors
sub sbox     { return \@SBOX }
sub inv_sbox { return \@INV_SBOX }

# ─────────────────────────────────────────────────────────────────────────────
# State Helpers
#
# AES state: 4×4 column-major byte matrix.
# state[row][col] = block[row + 4*col]   (0-indexed)
# We represent state as arrayref of 4 rows, each an arrayref of 4 bytes.
# ─────────────────────────────────────────────────────────────────────────────

sub _bytes_to_state {
    my ($block) = @_;
    my @bytes = unpack('C*', $block);
    return [
        map { my $r = $_; [map { $bytes[$r + 4 * $_] } 0..3] } 0..3
    ];
}

sub _state_to_bytes {
    my ($state) = @_;
    return pack('C*', map { my $c = $_; map { $state->[$_][$c] } 0..3 } 0..3);
}

# ─────────────────────────────────────────────────────────────────────────────
# Key Schedule: expand_key
# ─────────────────────────────────────────────────────────────────────────────

=head2 expand_key($key)

Expand a 16, 24, or 32-byte key into (Nr+1) round keys.
Returns an arrayref of (Nr+1) state matrices.

=cut

sub expand_key {
    my ($key) = @_;
    my $key_len = length($key);
    die "AES key must be 16, 24, or 32 bytes; got $key_len\n"
        unless $key_len == 16 || $key_len == 24 || $key_len == 32;

    my $nk = $key_len / 4;
    my %nr_map = (4 => 10, 6 => 12, 8 => 14);
    my $nr = $nr_map{$nk};
    my $total_words = 4 * ($nr + 1);

    my @bytes = unpack('C*', $key);

    # Initialize W[0..nk-1] from the key
    my @W;
    for my $i (0 .. $nk - 1) {
        $W[$i] = [@bytes[4*$i .. 4*$i+3]];
    }

    for my $i ($nk .. $total_words - 1) {
        my @temp = @{$W[$i - 1]};
        if ($i % $nk == 0) {
            # RotWord + SubWord + XOR Rcon
            @temp = @temp[1, 2, 3, 0];
            @temp = map { $SBOX[$_] } @temp;
            $temp[0] ^= $RCON[$i / $nk - 1];
        } elsif ($nk == 8 && $i % $nk == 4) {
            @temp = map { $SBOX[$_] } @temp;
        }
        my @prev = @{$W[$i - $nk]};
        $W[$i] = [map { $prev[$_] ^ $temp[$_] } 0..3];
    }

    # Pack into (Nr+1) round key states
    my @round_keys;
    for my $rk (0 .. $nr) {
        my @rk_words = @W[4*$rk .. 4*$rk+3];
        push @round_keys, [
            map { my $r = $_; [map { $rk_words[$_][$r] } 0..3] } 0..3
        ];
    }
    return \@round_keys;
}

# ─────────────────────────────────────────────────────────────────────────────
# AES Operations
# ─────────────────────────────────────────────────────────────────────────────

sub _add_round_key {
    my ($state, $rk) = @_;
    return [map { my $r = $_; [map { $state->[$r][$_] ^ $rk->[$r][$_] } 0..3] } 0..3];
}

sub _sub_bytes {
    my ($state) = @_;
    return [map { [map { $SBOX[$_] } @$_] } @$state];
}

sub _inv_sub_bytes {
    my ($state) = @_;
    return [map { [map { $INV_SBOX[$_] } @$_] } @$state];
}

# ShiftRows: shift row r left by r positions (0-indexed: row 0 unchanged, row 1 left 1, etc.)
sub _shift_rows {
    my ($state) = @_;
    return [map { my $r = $_; [map { $state->[$r][($_ + $r) % 4] } 0..3] } 0..3];
}

# InvShiftRows: shift row r right by r positions
sub _inv_shift_rows {
    my ($state) = @_;
    return [map { my $r = $_; [map { $state->[$r][($_ - $r + 4) % 4] } 0..3] } 0..3];
}

# MixColumns: multiply each column by AES matrix
sub _mix_col {
    my ($s0, $s1, $s2, $s3) = @_;
    return (
        _xtime($s0) ^ _xtime($s1) ^ $s1 ^ $s2 ^ $s3,
        $s0 ^ _xtime($s1) ^ _xtime($s2) ^ $s2 ^ $s3,
        $s0 ^ $s1 ^ _xtime($s2) ^ _xtime($s3) ^ $s3,
        _xtime($s0) ^ $s0 ^ $s1 ^ $s2 ^ _xtime($s3),
    );
}

sub _mix_columns {
    my ($state) = @_;
    my $out = [map { [(0) x 4] } 0..3];
    for my $c (0..3) {
        my @col = map { $state->[$_][$c] } 0..3;
        my @mixed = _mix_col(@col);
        $out->[$_][$c] = $mixed[$_] for 0..3;
    }
    return $out;
}

# InvMixColumns: multiply by inverse AES matrix
sub _inv_mix_col {
    my ($s0, $s1, $s2, $s3) = @_;
    return (
        _gf_mul(0x0e,$s0)^_gf_mul(0x0b,$s1)^_gf_mul(0x0d,$s2)^_gf_mul(0x09,$s3),
        _gf_mul(0x09,$s0)^_gf_mul(0x0e,$s1)^_gf_mul(0x0b,$s2)^_gf_mul(0x0d,$s3),
        _gf_mul(0x0d,$s0)^_gf_mul(0x09,$s1)^_gf_mul(0x0e,$s2)^_gf_mul(0x0b,$s3),
        _gf_mul(0x0b,$s0)^_gf_mul(0x0d,$s1)^_gf_mul(0x09,$s2)^_gf_mul(0x0e,$s3),
    );
}

sub _inv_mix_columns {
    my ($state) = @_;
    my $out = [map { [(0) x 4] } 0..3];
    for my $c (0..3) {
        my @col = map { $state->[$_][$c] } 0..3;
        my @mixed = _inv_mix_col(@col);
        $out->[$_][$c] = $mixed[$_] for 0..3;
    }
    return $out;
}

# ─────────────────────────────────────────────────────────────────────────────
# Public API
# ─────────────────────────────────────────────────────────────────────────────

=head2 aes_encrypt_block($block, $key)

Encrypt a single 16-byte block with AES.

=cut

sub aes_encrypt_block {
    my ($block, $key) = @_;
    die "AES block must be 16 bytes, got " . length($block) . "\n"
        unless length($block) == 16;

    my $round_keys = expand_key($key);
    my $nr = $#$round_keys;

    my $state = _bytes_to_state($block);
    $state = _add_round_key($state, $round_keys->[0]);

    for my $rnd (1 .. $nr - 1) {
        $state = _sub_bytes($state);
        $state = _shift_rows($state);
        $state = _mix_columns($state);
        $state = _add_round_key($state, $round_keys->[$rnd]);
    }

    $state = _sub_bytes($state);
    $state = _shift_rows($state);
    $state = _add_round_key($state, $round_keys->[$nr]);

    return _state_to_bytes($state);
}

=head2 aes_decrypt_block($block, $key)

Decrypt a single 16-byte block with AES.

=cut

sub aes_decrypt_block {
    my ($block, $key) = @_;
    die "AES block must be 16 bytes, got " . length($block) . "\n"
        unless length($block) == 16;

    my $round_keys = expand_key($key);
    my $nr = $#$round_keys;

    my $state = _bytes_to_state($block);
    $state = _add_round_key($state, $round_keys->[$nr]);

    for my $rnd (reverse 1 .. $nr - 1) {
        $state = _inv_shift_rows($state);
        $state = _inv_sub_bytes($state);
        $state = _add_round_key($state, $round_keys->[$rnd]);
        $state = _inv_mix_columns($state);
    }

    $state = _inv_shift_rows($state);
    $state = _inv_sub_bytes($state);
    $state = _add_round_key($state, $round_keys->[0]);

    return _state_to_bytes($state);
}

1;

=head1 NAME

CodingAdventures::AES - AES block cipher (FIPS 197)

=head1 SYNOPSIS

    use CodingAdventures::AES;

    my $key   = pack('H*', '2b7e151628aed2a6abf7158809cf4f3c');
    my $plain = pack('H*', '3243f6a8885a308d313198a2e0370734');
    my $ct    = CodingAdventures::AES::aes_encrypt_block($plain, $key);
    my $pt    = CodingAdventures::AES::aes_decrypt_block($ct, $key);

=head1 VERSION

0.01

=cut
