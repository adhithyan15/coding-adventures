package CodingAdventures::QrCode;

# =============================================================================
# CodingAdventures::QrCode — ISO/IEC 18004:2015 compliant QR Code encoder
# =============================================================================
#
# QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in 1994
# to track automotive parts moving down an assembly line. The goal: scan the
# barcode from any direction, even at oblique angles, without needing to orient
# it first. Today QR Code is the most widely-deployed 2D barcode on Earth —
# every smartphone camera can decode one without a separate app.
#
# ## What makes QR Code special?
#
# 1. **Omnidirectionality** — three "finder" patterns (big squares at three
#    corners) let a decoder locate and orient the symbol in any direction.
# 2. **Error correction** — up to 30% of the symbol can be obscured (logo
#    overlaid, torn, smudged) and the message can still be recovered. This uses
#    Reed-Solomon codes over GF(256).
# 3. **High density** — a version-40 symbol holds 7089 numeric characters (or
#    2953 bytes) in 177×177 modules.
#
# ## Encoding pipeline
#
# ```
# input string
#   → mode selection    (numeric / alphanumeric / byte)
#   → version selection (smallest version 1–40 that fits at the chosen ECC level)
#   → bit stream        (mode indicator + char count + data + padding)
#   → blocks + RS ECC   (GF(256) b=0 convention, generator poly 0x11D)
#   → interleave        (data CWs round-robin, then ECC CWs round-robin)
#   → grid init         (finder, separator, timing, alignment, format, dark module)
#   → zigzag placement  (two-column snake from bottom-right corner)
#   → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
#   → finalize          (format info written with best mask; version info for v7+)
#   → ModuleGrid        (hashref with rows/cols/modules/module_shape)
# ```
#
# ## Building blocks
#
# - P2D01 barcode-2d   — ModuleGrid representation and layout()
# - MA00 polynomial    — polynomial arithmetic over GF(256)
# - MA01 gf256         — GF(2^8) field arithmetic
# - MA02 reed-solomon  — RS encoding (re-implemented inline for b=0 convention)
#
# ## Reference
#
# ISO/IEC 18004:2015 (QR Code bar code symbology specification)
# Nayuki QR Code generator (public domain reference)
#
# =============================================================================

use strict;
use warnings;
use Carp qw(croak confess);
use Encode ();
use POSIX  ();

use CodingAdventures::Barcode2D ();
use CodingAdventures::GF256     ();

our $VERSION = '0.1.0';

# =============================================================================
# Error correction level constants
# =============================================================================
#
# QR Code supports four error correction levels. Higher levels protect more of
# the data but require more ECC codewords, leaving less space for payload.
#
# | Level | Recovery | Use case                          |
# |-------|----------|-----------------------------------|
# | L     | ~7 %     | Maximum data density              |
# | M     | ~15 %    | General-purpose (common default)  |
# | Q     | ~25 %    | Moderate noise or damage expected |
# | H     | ~30 %    | Logo overlay; high damage risk    |
#
# ECC_INDICATOR: 2-bit field embedded in format information.
# Note the deliberate non-alphabetical order! L=01, M=00, Q=11, H=10.
#
# ECC_IDX: 0-based index for table lookups (L=0, M=1, Q=2, H=3).

my %ECC_INDICATOR = ( L => 0b01, M => 0b00, Q => 0b11, H => 0b10 );
my %ECC_IDX       = ( L => 0,    M => 1,    Q => 2,    H => 3    );

# =============================================================================
# ISO 18004:2015 — Capacity tables
# =============================================================================
#
# These two tables are straight from ISO Annex I (Table 9).
# Index 0 is a placeholder (versions run 1–40).
#
# ECC_CW_PER_BLOCK: how many ECC codewords each block carries.
# NUM_BLOCKS:       how many RS blocks the codeword stream is split into.
#
# Together they tell us:
#
#   total ECC codewords = NUM_BLOCKS[ecc][version] × ECC_CW_PER_BLOCK[ecc][version]
#   total data codewords = floor(raw_modules/8) − total_ECC_codewords

my @ECC_CW_PER_BLOCK = (
    # L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    # Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
);

my @NUM_BLOCKS = (
    # L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    # M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    # Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    # H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80],
);

# =============================================================================
# Alignment pattern centre coordinates — ISO 18004:2015 Annex E
# =============================================================================
#
# Versions 2+ have "alignment patterns" — small 5×5 finder-like squares that
# help decoders correct for perspective distortion and warping. Their centres
# are listed here by version. The cross-product of any two values in a row
# gives a candidate position; positions that overlap finder patterns (checked
# by the reserved-flag) are skipped during placement.
#
# Version 1 has no alignment patterns.

my @ALIGNMENT_POSITIONS = (
    [],                              # v1  — none
    [6, 18],                         # v2
    [6, 22],                         # v3
    [6, 26],                         # v4
    [6, 30],                         # v5
    [6, 34],                         # v6
    [6, 22, 38],                     # v7
    [6, 24, 42],                     # v8
    [6, 26, 46],                     # v9
    [6, 28, 50],                     # v10
    [6, 30, 54],                     # v11
    [6, 32, 58],                     # v12
    [6, 34, 62],                     # v13
    [6, 26, 46, 66],                 # v14
    [6, 26, 48, 70],                 # v15
    [6, 26, 50, 74],                 # v16
    [6, 30, 54, 78],                 # v17
    [6, 30, 56, 82],                 # v18
    [6, 30, 58, 86],                 # v19
    [6, 34, 62, 90],                 # v20
    [6, 28, 50, 72, 94],             # v21
    [6, 26, 50, 74, 98],             # v22
    [6, 30, 54, 78, 102],            # v23
    [6, 28, 54, 80, 106],            # v24
    [6, 32, 58, 84, 110],            # v25
    [6, 30, 58, 86, 114],            # v26
    [6, 34, 62, 90, 118],            # v27
    [6, 26, 50, 74, 98, 122],        # v28
    [6, 30, 54, 78, 102, 126],       # v29
    [6, 26, 52, 78, 104, 130],       # v30
    [6, 30, 56, 82, 108, 134],       # v31
    [6, 34, 60, 86, 112, 138],       # v32
    [6, 30, 58, 86, 114, 142],       # v33
    [6, 34, 62, 90, 118, 146],       # v34
    [6, 30, 54, 78, 102, 126, 150],  # v35
    [6, 24, 50, 76, 102, 128, 154],  # v36
    [6, 28, 54, 80, 106, 132, 158],  # v37
    [6, 32, 58, 84, 110, 136, 162],  # v38
    [6, 26, 54, 82, 110, 138, 166],  # v39
    [6, 30, 58, 86, 114, 142, 170],  # v40
);

# =============================================================================
# Data encoding mode constants
# =============================================================================
#
# QR Code defines three data encoding modes. Each compresses the input
# differently:
#
#   numeric      — 0-9 only. Groups of 3 digits → 10 bits (1000 combinations).
#                  Two digits → 7 bits, single digit → 4 bits.
#   alphanumeric — 45-char set (0-9, A-Z, space, $%*+-./:).
#                  Pairs encode as (idx1*45+idx2) → 11 bits. Single → 6 bits.
#   byte         — arbitrary bytes. Each byte → 8 bits (UTF-8 encoding).
#
# MODE_INDICATOR: 4-bit code prepended to the data stream.
# The mode is chosen to minimise total bit count for the given input.

my %MODE_INDICATOR = ( numeric => 0b0001, alphanumeric => 0b0010, byte => 0b0100 );

# The 45 characters in QR Code's alphanumeric set, at their canonical indices.
my $ALPHANUM_CHARS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';

# =============================================================================
# RS generator polynomial cache
# =============================================================================
#
# For each block size we need a distinct monic generator polynomial of degree n.
# We cache them here (keyed by n) to avoid rebuilding on every encode.

my %GENERATORS;

# =============================================================================
# Grid geometry helpers
# =============================================================================

# symbol_size — width/height of the QR symbol in modules.
#
# Formula: 4 × version + 17
# Version 1 → 21 modules; Version 40 → 177 modules.
sub _symbol_size { 4 * $_[0] + 17 }

# num_raw_data_modules — total bit-capable modules in the symbol.
#
# Subtracts all function pattern areas (finders, separators, timing, alignments,
# format info, version info) from the total (size × size).
# Formula from Nayuki's public-domain reference implementation.
sub _num_raw_data_modules {
    my ($version) = @_;
    my $result = (16 * $version + 128) * $version + 64;
    if ($version >= 2) {
        my $num_align = int($version / 7) + 2;
        $result -= (25 * $num_align - 10) * $num_align - 55;
        $result -= 36 if $version >= 7;
    }
    return $result;
}

# num_data_codewords — how many bytes the payload (message + padding, no ECC) occupies.
sub _num_data_codewords {
    my ($version, $ecc) = @_;
    my $e = $ECC_IDX{$ecc};
    return int(_num_raw_data_modules($version) / 8)
         - $NUM_BLOCKS[$e][$version] * $ECC_CW_PER_BLOCK[$e][$version];
}

# num_remainder_bits — leftover bits (0, 3, 4, or 7) appended after interleaved
# codewords to fill the symbol exactly.
sub _num_remainder_bits { _num_raw_data_modules($_[0]) % 8 }

# =============================================================================
# Reed-Solomon (b=0 convention, inline)
# =============================================================================
#
# QR Code uses a specific RS variant:
#   - Over GF(256) with primitive polynomial 0x11D (x^8+x^4+x^3+x^2+1)
#   - "b=0 convention": generator = ∏(x + α^i) for i=0..n-1
#
# We implement this inline rather than calling the reed-solomon package because
# the reed-solomon package uses a different convention (b=1 or Berlekamp).
# Calling the wrong convention would silently produce undecodable QR codes.

# _gf_mul — multiply two GF(256) elements using CodingAdventures::GF256.
#
# Note: GF256::multiply is a plain function (not a method), so we call it
# with the full package path, not as a class method.
sub _gf_mul {
    my ($a, $b) = @_;
    return CodingAdventures::GF256::multiply($a, $b);
}

# _build_generator — construct the monic RS generator polynomial of degree n.
#
# g(x) = ∏(x + α^i) for i=0..n-1
#
# Starting from g=[1], multiply iteratively by (x + α^i):
#
#   new[j] = old[j-1] XOR (α^i · old[j])
#
# The result is a coefficient array of length n+1, constant term first (little-endian).
# Wait — actually big-endian here: index 0 is the leading coefficient (x^n = 1).
sub _build_generator {
    my ($n) = @_;
    my @g = (1);
    for my $i (0 .. $n - 1) {
        my $ai = $CodingAdventures::GF256::ALOG[$i];
        my @next = (0) x (scalar(@g) + 1);
        for my $j (0 .. $#g) {
            $next[$j]     ^= $g[$j];
            $next[$j + 1] ^= _gf_mul($g[$j], $ai);
        }
        @g = @next;
    }
    return \@g;
}

# _get_generator — return (cached) generator for degree n.
sub _get_generator {
    my ($n) = @_;
    $GENERATORS{$n} //= _build_generator($n);
    return $GENERATORS{$n};
}

# _rs_encode — compute ECC bytes: R(x) = D(x)·x^n mod G(x) via LFSR division.
#
# Classic shift-register implementation:
#
#   for each data byte b:
#     feedback = b XOR rem[0]
#     shift rem left (rem[i] = rem[i+1])
#     for i=0..n-1:  rem[i] ^= G[i+1] * feedback
#
# Result is n ECC bytes (the remainder polynomial, coefficient array).
sub _rs_encode {
    my ($data_ref, $gen_ref) = @_;
    my $n   = scalar(@$gen_ref) - 1;    # degree of generator
    my @rem = (0) x $n;

    for my $b (@$data_ref) {
        my $fb = $b ^ $rem[0];
        for my $i (0 .. $n - 2) { $rem[$i] = $rem[$i + 1]; }
        $rem[$n - 1] = 0;
        if ($fb != 0) {
            for my $i (0 .. $n - 1) {
                $rem[$i] ^= _gf_mul($gen_ref->[$i + 1], $fb);
            }
        }
    }
    return \@rem;
}

# =============================================================================
# Data encoding mode selection and bit encoding
# =============================================================================

# _select_mode — choose the most compact encoding mode for the input.
#
# Priority: numeric < alphanumeric < byte.
# We pick the most compact mode whose character set covers the entire input.
sub _select_mode {
    my ($input) = @_;
    return 'numeric'      if $input =~ /^\d*$/;
    return 'alphanumeric' if _all_alphanum($input);
    return 'byte';
}

# _all_alphanum — true if every character is in the 45-char QR alphanumeric set.
sub _all_alphanum {
    my ($s) = @_;
    for my $c (split //, $s) {
        return 0 if index($ALPHANUM_CHARS, $c) < 0;
    }
    return 1;
}

# _char_count_bits — width of the character-count field.
#
# This field varies with both mode and version:
#
#   Mode           V1–9   V10–26  V27–40
#   numeric          10      12      14
#   alphanumeric      9      11      13
#   byte              8      16      16   (note: same for V10+)
sub _char_count_bits {
    my ($mode, $version) = @_;
    if ($mode eq 'numeric') {
        return $version <= 9 ? 10 : $version <= 26 ? 12 : 14;
    }
    if ($mode eq 'alphanumeric') {
        return $version <= 9 ? 9 : $version <= 26 ? 11 : 13;
    }
    # byte
    return $version <= 9 ? 8 : 16;
}

# _encode_numeric — encode a decimal digit string into the bit stream.
#
# Digit triples → 10 bits (000–999 spans 0–999 < 1024 = 2^10).
# Digit pairs   →  7 bits (00–99 < 128 = 2^7).
# Single digit  →  4 bits (0–9 < 16 = 2^4).
sub _encode_numeric {
    my ($input, $bits_ref) = @_;
    my @chars = split //, $input;
    my $i = 0;
    while ($i + 2 <= $#chars) {
        my $v = substr($input, $i, 3) + 0;
        _write_bits($bits_ref, $v, 10);
        $i += 3;
    }
    if ($i + 1 <= $#chars) {
        my $v = substr($input, $i, 2) + 0;
        _write_bits($bits_ref, $v, 7);
        $i += 2;
    }
    if ($i <= $#chars) {
        _write_bits($bits_ref, substr($input, $i, 1) + 0, 4);
    }
}

# _encode_alphanumeric — encode a string using the 45-char QR alphanumeric set.
#
# Pairs encode as: (idx_a * 45 + idx_b) → 11 bits.
# Trailing single char → 6 bits.
sub _encode_alphanumeric {
    my ($input, $bits_ref) = @_;
    my @chars = split //, $input;
    my $i = 0;
    while ($i + 1 <= $#chars) {
        my $idx0 = index($ALPHANUM_CHARS, $chars[$i]);
        my $idx1 = index($ALPHANUM_CHARS, $chars[$i + 1]);
        croak "encodeAlphanumeric: char '$chars[$i]' not in QR alphanumeric set" if $idx0 < 0;
        croak "encodeAlphanumeric: char '$chars[$i+1]' not in QR alphanumeric set" if $idx1 < 0;
        _write_bits($bits_ref, $idx0 * 45 + $idx1, 11);
        $i += 2;
    }
    if ($i <= $#chars) {
        my $idx = index($ALPHANUM_CHARS, $chars[$i]);
        croak "encodeAlphanumeric: char '$chars[$i]' not in QR alphanumeric set" if $idx < 0;
        _write_bits($bits_ref, $idx, 6);
    }
}

# _encode_byte — encode a string as raw UTF-8 bytes, each 8 bits.
sub _encode_byte {
    my ($input, $bits_ref) = @_;
    my $utf8 = Encode::encode('UTF-8', $input);
    for my $b (unpack('C*', $utf8)) {
        _write_bits($bits_ref, $b, 8);
    }
}

# _write_bits — append $count bits from $value (MSB first) to @$bits_ref.
sub _write_bits {
    my ($bits_ref, $value, $count) = @_;
    for my $i (reverse 0 .. $count - 1) {
        push @$bits_ref, ($value >> $i) & 1;
    }
}

# _bits_to_bytes — pack a flat bit array into bytes (MSB first within each byte).
sub _bits_to_bytes {
    my ($bits_ref) = @_;
    my @bytes;
    for (my $i = 0; $i < scalar(@$bits_ref); $i += 8) {
        my $byte = 0;
        for my $j (0 .. 7) {
            $byte = ($byte << 1) | ($bits_ref->[$i + $j] // 0);
        }
        push @bytes, $byte;
    }
    return \@bytes;
}

# =============================================================================
# Build data codeword sequence
# =============================================================================

# _build_data_codewords — assemble the full data codeword sequence.
#
# Format: [mode 4b] [char count] [encoded data] [terminator ≤4b] [bit pad to byte
# boundary] [0xEC/0x11 alternating pad bytes up to capacity]
#
# The output has exactly _num_data_codewords($version, $ecc) bytes.
sub _build_data_codewords {
    my ($input, $version, $ecc) = @_;
    my $mode     = _select_mode($input);
    my $capacity = _num_data_codewords($version, $ecc);

    my @bits;

    # 4-bit mode indicator
    _write_bits(\@bits, $MODE_INDICATOR{$mode}, 4);

    # Character count (using byte length for byte mode; char length otherwise)
    my $char_count;
    if ($mode eq 'byte') {
        my $utf8 = Encode::encode('UTF-8', $input);
        $char_count = length($utf8);
    } else {
        $char_count = length($input);
    }
    _write_bits(\@bits, $char_count, _char_count_bits($mode, $version));

    # Payload
    if ($mode eq 'numeric') {
        _encode_numeric($input, \@bits);
    } elsif ($mode eq 'alphanumeric') {
        _encode_alphanumeric($input, \@bits);
    } else {
        _encode_byte($input, \@bits);
    }

    # Terminator: up to 4 zero bits (fewer if within 4 bits of capacity)
    my $term_len = $capacity * 8 - scalar(@bits);
    $term_len = 4 if $term_len > 4;
    _write_bits(\@bits, 0, $term_len) if $term_len > 0;

    # Pad to byte boundary
    my $rem = scalar(@bits) % 8;
    _write_bits(\@bits, 0, 8 - $rem) if $rem != 0;

    # Pad bytes to fill capacity: alternating 0xEC, 0x11
    my $bytes_ref = _bits_to_bytes(\@bits);
    my $pad = 0xEC;
    while (scalar(@$bytes_ref) < $capacity) {
        push @$bytes_ref, $pad;
        $pad = ($pad == 0xEC) ? 0x11 : 0xEC;
    }

    return $bytes_ref;
}

# =============================================================================
# Block splitting and ECC computation
# =============================================================================

# _compute_blocks — split data bytes into groups 1 and 2 blocks, each with ECC.
#
# QR Code splits the data into multiple RS blocks to bound the maximum burst
# error that any single block must correct. The split uses two block sizes:
#
#   Group 1 (g1_count blocks): each has shortLen data bytes
#   Group 2 (numLong blocks):  each has shortLen+1 data bytes
#
# Total: g1_count × shortLen + numLong × (shortLen+1) = totalData (exact).
sub _compute_blocks {
    my ($data_ref, $version, $ecc) = @_;
    my $e            = $ECC_IDX{$ecc};
    my $total_blocks = $NUM_BLOCKS[$e][$version];
    my $ecc_len      = $ECC_CW_PER_BLOCK[$e][$version];
    my $total_data   = _num_data_codewords($version, $ecc);
    my $short_len    = int($total_data / $total_blocks);
    my $num_long     = $total_data % $total_blocks;
    my $gen          = _get_generator($ecc_len);
    my @blocks;
    my $offset = 0;

    my $g1_count = $total_blocks - $num_long;
    for my $i (0 .. $g1_count - 1) {
        my @d    = @{$data_ref}[$offset .. $offset + $short_len - 1];
        my $ecc_bytes = _rs_encode(\@d, $gen);
        push @blocks, { data => \@d, ecc => $ecc_bytes };
        $offset += $short_len;
    }
    for my $i (0 .. $num_long - 1) {
        my @d    = @{$data_ref}[$offset .. $offset + $short_len];
        my $ecc_bytes = _rs_encode(\@d, $gen);
        push @blocks, { data => \@d, ecc => $ecc_bytes };
        $offset += $short_len + 1;
    }
    return \@blocks;
}

# _interleave_blocks — interleave codewords across blocks.
#
# Round-robin data codewords from each block (index 0, then 1, …), then
# round-robin ECC codewords. This spreads burst errors across blocks so that
# no single block loses more than a few codewords.
sub _interleave_blocks {
    my ($blocks_ref) = @_;
    my @result;
    my $max_data = 0;
    my $max_ecc  = 0;
    for my $b (@$blocks_ref) {
        my $dl = scalar(@{ $b->{data} });
        my $el = scalar(@{ $b->{ecc}  });
        $max_data = $dl if $dl > $max_data;
        $max_ecc  = $el if $el > $max_ecc;
    }
    for my $i (0 .. $max_data - 1) {
        for my $b (@$blocks_ref) {
            push @result, $b->{data}[$i] if $i < scalar(@{ $b->{data} });
        }
    }
    for my $i (0 .. $max_ecc - 1) {
        for my $b (@$blocks_ref) {
            push @result, $b->{ecc}[$i] if $i < scalar(@{ $b->{ecc} });
        }
    }
    return \@result;
}

# =============================================================================
# WorkGrid — internal mutable grid with reserved-module tracking
# =============================================================================
#
# During construction we need:
#   modules[r][c]  — boolean: is this module currently dark?
#   reserved[r][c] — boolean: is this a function module (finder/separator/
#                    timing/alignment/format/version)? Reserved modules are
#                    skipped during data placement and masking.
#
# We use a flat hashref with two 2D AoA fields. Unlike Barcode2D's immutable
# grid, this is MUTABLE during construction for performance (we build one grid
# in-place, avoiding thousands of full copies).

sub _make_work_grid {
    my ($size) = @_;
    my @modules;
    my @reserved;
    for my $r (0 .. $size - 1) {
        push @modules,  [(0) x $size];
        push @reserved, [(0) x $size];
    }
    return {
        size     => $size,
        modules  => \@modules,
        reserved => \@reserved,
    };
}

# =============================================================================
# Function pattern placement
# =============================================================================

# _set_mod — write a module value (with optional reservation).
sub _set_mod {
    my ($g, $r, $c, $dark, $reserve) = @_;
    $g->{modules}[$r][$c]  = $dark ? 1 : 0;
    $g->{reserved}[$r][$c] = 1 if $reserve;
}

# _place_finder — draw a 7×7 finder pattern centred at (top_row, top_col).
#
# Layout (■=dark, □=light):
#
#   ■ ■ ■ ■ ■ ■ ■
#   ■ □ □ □ □ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ □ □ □ □ ■
#   ■ ■ ■ ■ ■ ■ ■
#
# The 1:1:3:1:1 ratio in every scan direction lets any decoder find and orient
# the symbol even under occlusion or rotation.
sub _place_finder {
    my ($g, $top_r, $top_c) = @_;
    for my $dr (0 .. 6) {
        for my $dc (0 .. 6) {
            my $on_border = ($dr == 0 || $dr == 6 || $dc == 0 || $dc == 6);
            my $in_core   = ($dr >= 2 && $dr <= 4 && $dc >= 2 && $dc <= 4);
            _set_mod($g, $top_r + $dr, $top_c + $dc, ($on_border || $in_core) ? 1 : 0, 1);
        }
    }
}

# _place_alignment — draw a 5×5 alignment pattern centred at (row, col).
#
# Layout:
#   ■ ■ ■ ■ ■
#   ■ □ □ □ ■
#   ■ □ ■ □ ■
#   ■ □ □ □ ■
#   ■ ■ ■ ■ ■
#
# These appear in versions 2+ and help decoders correct for perspective
# distortion and barrel/pincushion warping.
sub _place_alignment {
    my ($g, $row, $col) = @_;
    for my $dr (-2 .. 2) {
        for my $dc (-2 .. 2) {
            my $on_border = (abs($dr) == 2 || abs($dc) == 2);
            my $is_center = ($dr == 0 && $dc == 0);
            _set_mod($g, $row + $dr, $col + $dc, ($on_border || $is_center) ? 1 : 0, 1);
        }
    }
}

# _place_all_alignments — place alignment patterns for the version.
#
# For each cross-product pair of ALIGNMENT_POSITIONS[version-1], skip any
# centre that lands on a reserved module (finder or timing).
sub _place_all_alignments {
    my ($g, $version) = @_;
    my $pos = $ALIGNMENT_POSITIONS[$version - 1];
    for my $row (@$pos) {
        for my $col (@$pos) {
            next if $g->{reserved}[$row][$col];   # overlaps finder/timing
            _place_alignment($g, $row, $col);
        }
    }
}

# _place_timing_strips — alternating dark/light timing strips.
#
# Row 6, cols 8..size-9 (horizontal strip).
# Col 6, rows 8..size-9 (vertical strip).
# Dark when index is even (module at position 8 is always dark since 8%2==0).
sub _place_timing_strips {
    my ($g) = @_;
    my $sz = $g->{size};
    for my $c (8 .. $sz - 9) { _set_mod($g, 6, $c, ($c % 2 == 0) ? 1 : 0, 1); }
    for my $r (8 .. $sz - 9) { _set_mod($g, $r, 6, ($r % 2 == 0) ? 1 : 0, 1); }
}

# _reserve_format_info — mark format information positions as reserved.
#
# Format info occupies 15 modules in two copies:
#
# Copy 1 — around top-left finder:
#   row 8, cols 0-8 (skip col 6 = timing)  and
#   col 8, rows 0-8 (skip row 6 = timing)
#
# Copy 2:
#   col 8, rows size-7..size-1  (bottom-left)
#   row 8, cols size-8..size-1  (top-right)
sub _reserve_format_info {
    my ($g) = @_;
    my $sz = $g->{size};
    for my $c (0 .. 8) { $g->{reserved}[8][$c]  = 1 if $c != 6; }
    for my $r (0 .. 8) { $g->{reserved}[$r][8]  = 1 if $r != 6; }
    for my $r ($sz - 7 .. $sz - 1) { $g->{reserved}[$r][8] = 1; }
    for my $c ($sz - 8 .. $sz - 1) { $g->{reserved}[8][$c] = 1; }
}

# _reserve_version_info — mark version information positions (v7+).
#
# Two 6×3 blocks:
#   Near top-right:   rows 0-5, cols size-11..size-9
#   Near bottom-left: rows size-11..size-9, cols 0-5
sub _reserve_version_info {
    my ($g, $version) = @_;
    return if $version < 7;
    my $sz = $g->{size};
    for my $r (0 .. 5) {
        for my $dc (0 .. 2) { $g->{reserved}[$r][$sz - 11 + $dc] = 1; }
    }
    for my $dr (0 .. 2) {
        for my $c (0 .. 5) { $g->{reserved}[$sz - 11 + $dr][$c] = 1; }
    }
}

# _place_dark_module — the always-dark module at (4V+9, 8).
#
# This single reserved dark module is specified by the standard as a constant.
# It is never toggled by masking.
sub _place_dark_module {
    my ($g, $version) = @_;
    _set_mod($g, 4 * $version + 9, 8, 1, 1);
}

# =============================================================================
# Format information
# =============================================================================

# _compute_format_bits — 15-bit format word for (ecc_level, mask_pattern).
#
# 1. 5-bit data = [ECC indicator (2b)] [mask (3b)]
# 2. BCH(15,5) error protection: compute remainder of (data × x^10) mod G(x)
#    where G(x) = x^10+x^8+x^5+x^4+x^2+x+1 = 0x537
# 3. Concatenate: bits 14-10 = data, bits 9-0 = remainder
# 4. XOR with 0x5412 (prevents all-zero format info, which would be invisible)
#
# The returned value is a 15-bit integer with bit 14 = MSB.
sub _compute_format_bits {
    my ($ecc, $mask) = @_;
    my $data = ($ECC_INDICATOR{$ecc} << 3) | $mask;
    my $rem  = $data << 10;
    for my $i (reverse 10 .. 14) {
        $rem ^= 0x537 << ($i - 10) if ($rem >> $i) & 1;
    }
    return (($data << 10) | ($rem & 0x3ff)) ^ 0x5412;
}

# _write_format_info — write the 15-bit format word into both copy locations.
#
# IMPORTANT: bit ordering follows ISO/IEC 18004 and has been verified against
# the lessons.md record (2026-04-23). The TypeScript reference uses a different
# (incorrect for strict ISO) order; this implementation matches the Rust port
# which was validated with zbarimg.
#
# The 15 bits are labeled f14..f0 with f14 = MSB.
#
# Copy 1 (around top-left finder):
#   Row 8, cols 0-5:  f14 (col 0) → f9 (col 5)  [MSB first, left-to-right]
#   Row 8, col 7:     f8
#   Row 8, col 8:     f7
#   Col 8, row 7:     f6   (row 6 is timing — skip)
#   Col 8, rows 0-5:  f0 (row 0) → f5 (row 5)   [LSB first, top-to-bottom]
#
# Copy 2:
#   Row 8, cols n-1..n-8: f0 (col n-1) → f7 (col n-8)  [LSB at right]
#   Col 8, rows n-7..n-1: f8 (row n-7) → f14 (row n-1)
sub _write_format_info {
    my ($g, $fmt) = @_;
    my $sz = $g->{size};

    # ── Copy 1 ──────────────────────────────────────────────────────────────
    # Row 8, cols 0-5: f14 down to f9 (MSB first, left-to-right)
    for my $i (0 .. 5) {
        $g->{modules}[8][$i] = ($fmt >> (14 - $i)) & 1;
    }
    $g->{modules}[8][7] = ($fmt >> 8) & 1;   # f8
    $g->{modules}[8][8] = ($fmt >> 7) & 1;   # f7
    $g->{modules}[7][8] = ($fmt >> 6) & 1;   # f6  (row 6 is timing, skip)
    # Col 8, rows 0-5: f0 at row 0 … f5 at row 5 (LSB at top)
    for my $i (0 .. 5) {
        $g->{modules}[$i][8] = ($fmt >> $i) & 1;
    }

    # ── Copy 2 ──────────────────────────────────────────────────────────────
    # Row 8, cols n-1 down to n-8: f0 at col n-1 … f7 at col n-8
    for my $i (0 .. 7) {
        $g->{modules}[8][$sz - 1 - $i] = ($fmt >> $i) & 1;
    }
    # Col 8, rows n-7 to n-1: f8 at row n-7 … f14 at row n-1
    for my $i (8 .. 14) {
        $g->{modules}[$sz - 15 + $i][8] = ($fmt >> $i) & 1;
    }
}

# =============================================================================
# Version information (v7+)
# =============================================================================

# _compute_version_bits — 18-bit version word for v7+.
#
# 1. 6-bit version number
# 2. BCH(18,6): remainder of (version × x^12) mod G(x),
#    G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25
# 3. Result: bits 17-12 = version, bits 11-0 = remainder
sub _compute_version_bits {
    my ($version) = @_;
    my $rem = $version << 12;
    for my $i (reverse 12 .. 17) {
        $rem ^= 0x1f25 << ($i - 12) if ($rem >> $i) & 1;
    }
    return ($version << 12) | ($rem & 0xfff);
}

# _write_version_info — write 18-bit version info into both 6×3 blocks (v7+).
#
# Top-right block:   bit i → row = 5 - floor(i/3), col = size - 9 - (i%3)
# Bottom-left block: bit i → row = size - 9 - (i%3), col = 5 - floor(i/3)
# (the two blocks are transposes of each other)
sub _write_version_info {
    my ($g, $version) = @_;
    return if $version < 7;
    my $sz   = $g->{size};
    my $bits = _compute_version_bits($version);
    for my $i (0 .. 17) {
        my $dark = ($bits >> $i) & 1;
        my $a = 5 - int($i / 3);
        my $b = $sz - 9 - ($i % 3);
        $g->{modules}[$a][$b] = $dark;
        $g->{modules}[$b][$a] = $dark;
    }
}

# =============================================================================
# Data placement — zigzag scan
# =============================================================================

# _place_bits — fill non-reserved modules with codeword bits using zigzag scan.
#
# The zigzag scan visits the symbol in two-column strips from right to left,
# alternating direction (up/down) with each strip:
#
#   Col n-1/n-2 → upward strip
#   Col n-3/n-4 → downward strip
#   ... and so on leftward.
#
# Column 6 (vertical timing strip) is always skipped — when the leading column
# would be 6, we decrement to 5 instead.
#
# Reserved modules are skipped; data bits fill the non-reserved positions.
# After all codeword bits are placed, remainder bits (0-valued) fill any
# remaining non-reserved positions.
sub _place_bits {
    my ($g, $codewords_ref, $version) = @_;
    my $sz = $g->{size};

    # Flatten codewords to a bit array (MSB first within each codeword).
    my @bits;
    for my $cw (@$codewords_ref) {
        for my $b (reverse 0 .. 7) {
            push @bits, ($cw >> $b) & 1;
        }
    }
    # Append remainder bits (all zero).
    for my $i (0 .. _num_remainder_bits($version) - 1) {
        push @bits, 0;
    }

    my $bit_idx = 0;
    my $up      = 1;       # 1 = scanning upward (row decreasing), 0 = downward
    my $col     = $sz - 1; # leading column of current 2-column strip

    while ($col >= 1) {
        for my $vi (0 .. $sz - 1) {
            my $row = $up ? ($sz - 1 - $vi) : $vi;
            for my $dc (0, 1) {
                my $c = $col - $dc;
                next if $c == 6;                     # timing column
                next if $g->{reserved}[$row][$c];    # function module
                $g->{modules}[$row][$c] = ($bit_idx < scalar(@bits)) ? $bits[$bit_idx++] : 0;
            }
        }
        $up  = !$up;
        $col -= 2;
        $col  = 5 if $col == 6;   # hop over the vertical timing strip
    }
}

# =============================================================================
# Masking
# =============================================================================

# The 8 mask conditions from ISO 18004 Table 10.
# If condition($row, $col) is true, the module is XOR-flipped (dark ↔ light).
# Applied only to non-reserved (data/ECC) modules.

my @MASK_CONDS = (
    sub { ($_[0] + $_[1]) % 2 == 0 },                           # 0
    sub { $_[0] % 2 == 0 },                                      # 1
    sub { $_[1] % 3 == 0 },                                      # 2
    sub { ($_[0] + $_[1]) % 3 == 0 },                           # 3
    sub { (int($_[0] / 2) + int($_[1] / 3)) % 2 == 0 },         # 4
    sub { ($_[0] * $_[1]) % 2 + ($_[0] * $_[1]) % 3 == 0 },     # 5
    sub { (($_[0] * $_[1]) % 2 + ($_[0] * $_[1]) % 3) % 2 == 0 },  # 6
    sub { (($_[0] + $_[1]) % 2 + ($_[0] * $_[1]) % 3) % 2 == 0 },  # 7
);

# _apply_mask — return a new module grid (AoA) with mask applied.
#
# Only non-reserved modules are toggled. We return a new AoA rather than
# modifying the base grid in-place, so we can try all 8 masks cheaply.
sub _apply_mask {
    my ($modules_ref, $reserved_ref, $sz, $mask_idx) = @_;
    my $cond = $MASK_CONDS[$mask_idx];
    my @masked;
    for my $r (0 .. $sz - 1) {
        my @row;
        for my $c (0 .. $sz - 1) {
            if ($reserved_ref->[$r][$c]) {
                push @row, $modules_ref->[$r][$c];
            } else {
                push @row, $modules_ref->[$r][$c] ^ ($cond->($r, $c) ? 1 : 0);
            }
        }
        push @masked, \@row;
    }
    return \@masked;
}

# =============================================================================
# Penalty scoring — ISO 18004 Section 7.8.3
# =============================================================================

# _compute_penalty — score a masked module grid (lower is better).
#
# Four rules, each adding to a running penalty total:
#
# Rule 1: Runs of ≥5 same-colour modules in a row or column.
#   score += (run_length - 2) for each run ≥ 5.
#
# Rule 2: 2×2 same-colour blocks.
#   score += 3 for each 2×2 block.
#
# Rule 3: Finder-pattern-like sequences (1,0,1,1,1,0,1,0,0,0,0) or its
#   reverse — either direction (horizontal or vertical).
#   score += 40 per match.
#
# Rule 4: Dark module ratio deviation from 50%.
#   score += floor(|ratio - 50| / 5) * 10 per 5% step away from 50.
sub _compute_penalty {
    my ($modules_ref, $sz) = @_;
    my $penalty = 0;

    # ── Rule 1: same-colour runs ≥ 5 ────────────────────────────────────────
    for my $r (0 .. $sz - 1) {
        for my $horiz (1, 0) {   # 1 = horizontal, 0 = vertical
            my $run  = 1;
            my $prev = $horiz ? $modules_ref->[$r][0] : $modules_ref->[0][$r];
            for my $i (1 .. $sz - 1) {
                my $cur = $horiz ? $modules_ref->[$r][$i] : $modules_ref->[$i][$r];
                if ($cur == $prev) {
                    $run++;
                } else {
                    $penalty += $run - 2 if $run >= 5;
                    $run  = 1;
                    $prev = $cur;
                }
            }
            $penalty += $run - 2 if $run >= 5;
        }
    }

    # ── Rule 2: 2×2 same-colour blocks ──────────────────────────────────────
    for my $r (0 .. $sz - 2) {
        for my $c (0 .. $sz - 2) {
            my $d = $modules_ref->[$r][$c];
            if ($d == $modules_ref->[$r][$c + 1]
             && $d == $modules_ref->[$r + 1][$c]
             && $d == $modules_ref->[$r + 1][$c + 1]) {
                $penalty += 3;
            }
        }
    }

    # ── Rule 3: finder-pattern-like sequences ────────────────────────────────
    my @p1 = (1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0);
    my @p2 = (0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1);
    for my $a (0 .. $sz - 1) {
        for my $b (0 .. $sz - 12) {
            my ($mh1, $mh2, $mv1, $mv2) = (1, 1, 1, 1);
            for my $k (0 .. 10) {
                my $bh = $modules_ref->[$a][$b + $k];
                my $bv = $modules_ref->[$b + $k][$a];
                $mh1 = 0 if $bh != $p1[$k];
                $mh2 = 0 if $bh != $p2[$k];
                $mv1 = 0 if $bv != $p1[$k];
                $mv2 = 0 if $bv != $p2[$k];
            }
            $penalty += 40 if $mh1;
            $penalty += 40 if $mh2;
            $penalty += 40 if $mv1;
            $penalty += 40 if $mv2;
        }
    }

    # ── Rule 4: dark module ratio ─────────────────────────────────────────────
    my $dark = 0;
    for my $r (0 .. $sz - 1) {
        for my $c (0 .. $sz - 1) {
            $dark++ if $modules_ref->[$r][$c];
        }
    }
    my $ratio  = ($dark / ($sz * $sz)) * 100;
    my $prev5  = int($ratio / 5) * 5;
    my $a = abs($prev5 - 50);
    my $b = abs($prev5 + 5 - 50);
    $penalty += (($a < $b ? $a : $b) / 5) * 10;

    return $penalty;
}

# =============================================================================
# Version selection
# =============================================================================

# _select_version — find the smallest version (1–40) that fits the input.
#
# Computes the exact bit count for the selected mode and compares to capacity.
# Version and mode both affect charCountBits, so we must recalculate per version.
sub _select_version {
    my ($input, $ecc) = @_;
    my $mode = _select_mode($input);
    my $utf8 = Encode::encode('UTF-8', $input);
    my $byte_len = length($utf8);

    for my $v (1 .. 40) {
        my $capacity = _num_data_codewords($v, $ecc);
        my $data_bits;
        if ($mode eq 'byte') {
            $data_bits = $byte_len * 8;
        } elsif ($mode eq 'numeric') {
            $data_bits = int(length($input) * 10 / 3 + 0.9999999);   # ceil
        } else {
            # alphanumeric
            $data_bits = int(length($input) * 11 / 2 + 0.9999999);   # ceil
        }
        my $bits_needed = 4 + _char_count_bits($mode, $v) + $data_bits;
        return $v if int(($bits_needed + 7) / 8) <= $capacity;
    }
    croak "InputTooLong: input (${\ length($input)} chars, ECC=$ecc) exceeds version-40 capacity";
}

# =============================================================================
# Grid initialisation
# =============================================================================

# _build_grid — construct the function-pattern skeleton (no data yet).
#
# Places:
#   1. Three 7×7 finder patterns (top-left, top-right, bottom-left corners)
#   2. Separator (1-module light border) around each finder
#   3. Timing strips (row 6 and col 6)
#   4. Alignment patterns (version 2+)
#   5. Format info reserved positions
#   6. Version info reserved positions (v7+)
#   7. Dark module at (4V+9, 8)
sub _build_grid {
    my ($version) = @_;
    my $sz = _symbol_size($version);
    my $g  = _make_work_grid($sz);

    # Three finder patterns
    _place_finder($g, 0,      0     );   # top-left
    _place_finder($g, 0,      $sz-7 );   # top-right
    _place_finder($g, $sz-7,  0     );   # bottom-left

    # Separators (1-module light strip just outside each finder).
    # Top-left: row 7 (cols 0-7) and col 7 (rows 0-7)
    for my $i (0 .. 7) {
        _set_mod($g, 7, $i, 0, 1);
        _set_mod($g, $i, 7, 0, 1);
    }
    # Top-right: row 7 (cols sz-1..sz-8) and col sz-8 (rows 0-7)
    for my $i (0 .. 7) {
        _set_mod($g, 7, $sz - 1 - $i, 0, 1);
        _set_mod($g, $i, $sz - 8, 0, 1);
    }
    # Bottom-left: row sz-8 (cols 0-7) and col 7 (rows sz-1..sz-8)
    for my $i (0 .. 7) {
        _set_mod($g, $sz - 8, $i, 0, 1);
        _set_mod($g, $sz - 1 - $i, 7, 0, 1);
    }

    _place_timing_strips($g);              # row 6, col 6
    _place_all_alignments($g, $version);   # version 2+

    _reserve_format_info($g);             # 15 positions × 2 copies
    _reserve_version_info($g, $version);  # 6×3 × 2 copies (v7+)
    _place_dark_module($g, $version);     # (4V+9, 8) always dark

    return $g;
}

# =============================================================================
# Public API
# =============================================================================

# encode — QR Code main entry point.
#
# Encodes a text string (UTF-8) into a ModuleGrid at the requested ECC level.
#
# Arguments:
#   $class  — class name (called as CodingAdventures::QrCode->encode(...))
#   $data   — input string (UTF-8)
#   %opts   — options hash; recognised keys:
#               level => 'L' | 'M' | 'Q' | 'H'  (default 'M')
#
# Returns a ModuleGrid hashref (from CodingAdventures::Barcode2D):
#   {
#     rows         => $sz,
#     cols         => $sz,
#     modules      => \@modules,   # 2D boolean array (1=dark, 0=light)
#     module_shape => 'square',
#   }
#
# Throws a string exception prefixed with "InputTooLong:" if the input exceeds
# the version-40 capacity at the given ECC level.
#
# Example:
#   my $grid = CodingAdventures::QrCode->encode('Hello, World!', level => 'M');
#   # $grid->{rows} == $grid->{cols} == some odd number (21 for v1, 25 for v2, …)
sub encode {
    my ($class, $data, %opts) = @_;

    # Validate / default ECC level.
    my $level = $opts{level} // 'M';
    croak "encode: invalid ECC level '$level' (must be L, M, Q, or H)"
        unless exists $ECC_INDICATOR{$level};

    # Quick sanity guard: v40 numeric mode holds at most 7089 chars.
    if (length($data) > 7089) {
        croak "InputTooLong: input length " . length($data)
            . " exceeds 7089 (QR v40 numeric-mode maximum)";
    }

    # 1. Select version (smallest that fits).
    my $version = _select_version($data, $level);
    my $sz      = _symbol_size($version);

    # 2. Build data codeword sequence (mode + char count + payload + padding).
    my $data_cw   = _build_data_codewords($data, $version, $level);

    # 3. Split into RS blocks; compute ECC for each block.
    my $blocks    = _compute_blocks($data_cw, $version, $level);

    # 4. Interleave data codewords, then ECC codewords, across all blocks.
    my $interleaved = _interleave_blocks($blocks);

    # 5. Build the function-pattern skeleton.
    my $grid = _build_grid($version);

    # 6. Zigzag placement of interleaved codewords.
    _place_bits($grid, $interleaved, $version);

    # 7. Evaluate all 8 masks; pick the one with lowest penalty score.
    my $best_mask    = 0;
    my $best_penalty = ~0;   # infinity (max int)
    for my $m (0 .. 7) {
        my $masked   = _apply_mask($grid->{modules}, $grid->{reserved}, $sz, $m);
        my $fmt_bits = _compute_format_bits($level, $m);
        # Write format info into a temporary view of the grid
        my $test = {
            size     => $sz,
            modules  => $masked,
            reserved => $grid->{reserved},
        };
        _write_format_info($test, $fmt_bits);
        my $p = _compute_penalty($masked, $sz);
        if ($p < $best_penalty) {
            $best_penalty = $p;
            $best_mask    = $m;
        }
    }

    # 8. Finalize: apply best mask, write format info and version info.
    my $final_mods = _apply_mask($grid->{modules}, $grid->{reserved}, $sz, $best_mask);
    my $final_g = {
        size     => $sz,
        modules  => $final_mods,
        reserved => $grid->{reserved},
    };
    _write_format_info($final_g, _compute_format_bits($level, $best_mask));
    _write_version_info($final_g, $version);

    # 9. Return as a ModuleGrid hashref.
    return {
        rows         => $sz,
        cols         => $sz,
        modules      => $final_mods,
        module_shape => 'square',
    };
}

1;
