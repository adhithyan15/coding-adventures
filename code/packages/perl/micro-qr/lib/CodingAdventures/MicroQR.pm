package CodingAdventures::MicroQR;

# =============================================================================
# CodingAdventures::MicroQR — Micro QR Code encoder
#
# ISO/IEC 18004:2015 Annex E compliant Micro QR Code encoder.
#
# ## What Is Micro QR Code?
#
# Micro QR is the compact variant of QR Code, designed for applications where
# even the smallest standard QR Code (21×21 at version 1) is too large.
# Common use cases include:
#
#   - Surface-mount component labels on circuit boards
#   - Miniature industrial tags and part markings
#   - Any application where the 4-module quiet zone of regular QR wastes space
#
# The defining characteristic is the **single finder pattern** — where regular
# QR uses three identical corner squares to establish orientation, Micro QR
# uses only one, in the top-left corner. This saves significant space at the
# cost of some scanner robustness.
#
# ## Symbol sizes
#
#   M1: 11×11 modules    M2: 13×13 modules
#   M3: 15×15 modules    M4: 17×17 modules
#
#   Formula: size = 2 × version_number + 9
#
# ## Key differences from regular QR Code
#
#   - Single finder pattern at top-left only
#   - Timing patterns at row 0 / col 0 (not row 6 / col 6)
#   - Only 4 mask patterns (not 8)
#   - Format XOR mask 0x4445 (not 0x5412)
#   - Single copy of format info (not two)
#   - 2-module quiet zone (not 4)
#   - Narrower mode indicators: 0–3 bits (not fixed 4)
#   - Single block — no interleaving
#
# ## Encoding pipeline
#
#   input string
#     → auto-select smallest symbol (M1..M4) and mode
#     → build bit stream (mode indicator + char count + data + terminator + padding)
#     → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
#     → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
#     → zigzag data placement (two-column snake from bottom-right)
#     → evaluate 4 mask patterns, pick lowest penalty
#     → write format information (15 bits, single copy, XOR 0x4445)
#     → ModuleGrid
#
# ## Dependencies
#
#   CodingAdventures::GF256    — GF(256) multiply for Reed-Solomon
#   CodingAdventures::Barcode2D — ModuleGrid type and layout()
#
# =============================================================================

use strict;
use warnings;
use Carp qw(croak);
use List::Util ();
use Exporter 'import';

use CodingAdventures::GF256 qw(multiply);
use CodingAdventures::Barcode2D ();

our $VERSION = '0.1.0';

our @EXPORT_OK = qw(
    encode
    encode_at
    layout_grid
);

# =============================================================================
# Public type constants
# =============================================================================

# MicroQRVersion constants — the four symbol designators.
#
# Each version adds two rows and columns. The formula is:
#   size = 2 × version_number + 9
#
# So M1=11×11, M2=13×13, M3=15×15, M4=17×17.
use constant {
    M1 => 'M1',
    M2 => 'M2',
    M3 => 'M3',
    M4 => 'M4',
};

# ECC level constants.
#
# | Level     | Available in | Recovery        |
# |-----------|-------------|-----------------|
# | DETECTION | M1 only     | detects errors  |
# | L         | M2, M3, M4  | ~7% of codewords|
# | M         | M2, M3, M4  | ~15%            |
# | Q         | M4 only     | ~25%            |
#
# Level H is not available in any Micro QR symbol — the symbols are too small
# to absorb 30% redundancy while retaining useful capacity.
use constant {
    DETECTION => 'Detection',
    ECC_L     => 'L',
    ECC_M     => 'M',
    ECC_Q     => 'Q',
};

# =============================================================================
# Symbol configurations
# =============================================================================
#
# There are exactly 8 valid (version, ECC) combinations. Each configuration
# captures all the compile-time constants needed for encoding.
#
# Fields:
#   version         — M1, M2, M3, or M4
#   ecc             — Detection, L, M, or Q
#   symbol_indicator— 3-bit value in format information (0..7)
#   size            — symbol side length in modules (11, 13, 15, or 17)
#   data_cw         — number of data codewords (bytes, except M1 = 2.5 bytes)
#   ecc_cw          — number of ECC codewords
#   numeric_cap     — max numeric characters (0 = not supported)
#   alpha_cap       — max alphanumeric characters (0 = not supported)
#   byte_cap        — max byte characters (0 = not supported)
#   terminator_bits — terminator zero-bit count (3/5/7/9)
#   mode_bits       — mode indicator bit width (0=M1, 1=M2, 2=M3, 3=M4)
#   cc_numeric      — character count field width for numeric mode
#   cc_alpha        — character count field width for alphanumeric mode
#   cc_byte         — character count field width for byte mode
#   m1_half_cw      — true for M1: last data "codeword" is only 4 bits
#
# The 8 configurations are listed in the order SYMBOL_CONFIGS is iterated for
# auto-selection: smallest+weakest ECC first, largest+strongest last.

my @SYMBOL_CONFIGS = (
    # M1 / Detection
    {   version => M1, ecc => DETECTION, symbol_indicator => 0, size => 11,
        data_cw => 3,  ecc_cw => 2,
        numeric_cap => 5, alpha_cap => 0, byte_cap => 0,
        terminator_bits => 3, mode_bits => 0,
        cc_numeric => 3, cc_alpha => 0, cc_byte => 0,
        m1_half_cw => 1 },
    # M2 / L
    {   version => M2, ecc => ECC_L, symbol_indicator => 1, size => 13,
        data_cw => 5,  ecc_cw => 5,
        numeric_cap => 10, alpha_cap => 6, byte_cap => 4,
        terminator_bits => 5, mode_bits => 1,
        cc_numeric => 4, cc_alpha => 3, cc_byte => 4,
        m1_half_cw => 0 },
    # M2 / M
    {   version => M2, ecc => ECC_M, symbol_indicator => 2, size => 13,
        data_cw => 4,  ecc_cw => 6,
        numeric_cap => 8, alpha_cap => 5, byte_cap => 3,
        terminator_bits => 5, mode_bits => 1,
        cc_numeric => 4, cc_alpha => 3, cc_byte => 4,
        m1_half_cw => 0 },
    # M3 / L
    {   version => M3, ecc => ECC_L, symbol_indicator => 3, size => 15,
        data_cw => 11, ecc_cw => 6,
        numeric_cap => 23, alpha_cap => 14, byte_cap => 9,
        terminator_bits => 7, mode_bits => 2,
        cc_numeric => 5, cc_alpha => 4, cc_byte => 4,
        m1_half_cw => 0 },
    # M3 / M
    {   version => M3, ecc => ECC_M, symbol_indicator => 4, size => 15,
        data_cw => 9,  ecc_cw => 8,
        numeric_cap => 18, alpha_cap => 11, byte_cap => 7,
        terminator_bits => 7, mode_bits => 2,
        cc_numeric => 5, cc_alpha => 4, cc_byte => 4,
        m1_half_cw => 0 },
    # M4 / L
    {   version => M4, ecc => ECC_L, symbol_indicator => 5, size => 17,
        data_cw => 16, ecc_cw => 8,
        numeric_cap => 35, alpha_cap => 21, byte_cap => 15,
        terminator_bits => 9, mode_bits => 3,
        cc_numeric => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0 },
    # M4 / M
    {   version => M4, ecc => ECC_M, symbol_indicator => 6, size => 17,
        data_cw => 14, ecc_cw => 10,
        numeric_cap => 30, alpha_cap => 18, byte_cap => 13,
        terminator_bits => 9, mode_bits => 3,
        cc_numeric => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0 },
    # M4 / Q
    {   version => M4, ecc => ECC_Q, symbol_indicator => 7, size => 17,
        data_cw => 10, ecc_cw => 14,
        numeric_cap => 21, alpha_cap => 13, byte_cap => 9,
        terminator_bits => 9, mode_bits => 3,
        cc_numeric => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0 },
);

# =============================================================================
# RS generator polynomials (compile-time constants)
# =============================================================================
#
# Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
#
# The generator of degree n for n ECC codewords is:
#
#   g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1})
#
# where α = 2 (the primitive element of the field).
#
# Each entry is an arrayref of n+1 coefficients, leading monic term included,
# highest degree first.
#
# Only the six counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
# These are the same polynomials used by regular QR Code for blocks with
# matching ECC counts — the field and convention are identical.

my %RS_GENERATORS = (
    2  => [0x01, 0x03, 0x02],
    5  => [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68],
    6  => [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37],
    8  => [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3],
    10 => [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45],
    14 => [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1,
           0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac],
);

# =============================================================================
# Format information table (pre-computed)
# =============================================================================
#
# All 32 format words after XOR with the Micro QR mask 0x4445.
#
# The format information bit structure is:
#   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
#
# XOR-masked with 0x4445 to prevent Micro QR symbols from being misread
# as regular QR symbols (which use 0x5412).
#
# Indexed as $FORMAT_TABLE[$symbol_indicator][$mask_pattern].
# symbol_indicator runs 0..7, mask_pattern runs 0..3.
#
# The values are 15-bit integers (bit 14 = MSB, bit 0 = LSB).
# They are placed MSB-first into the format information strip.

my @FORMAT_TABLE = (
    [0x4445, 0x4172, 0x4E2B, 0x4B1C],  # M1  (sym_ind=0)
    [0x5528, 0x501F, 0x5F46, 0x5A71],  # M2-L (sym_ind=1)
    [0x6649, 0x637E, 0x6C27, 0x6910],  # M2-M (sym_ind=2)
    [0x7764, 0x7253, 0x7D0A, 0x783D],  # M3-L (sym_ind=3)
    [0x06DE, 0x03E9, 0x0CB0, 0x0987],  # M3-M (sym_ind=4)
    [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  # M4-L (sym_ind=5)
    [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  # M4-M (sym_ind=6)
    [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  # M4-Q (sym_ind=7)
);

# =============================================================================
# Alphanumeric character set
# =============================================================================
#
# The 45-character set shared with regular QR Code. Each character maps to its
# index in this string (0-based). Index is used in the 45×first+second encoding.
#
# | Range       | Indices |
# |-------------|---------|
# | '0'–'9'     | 0–9     |
# | 'A'–'Z'     | 10–35   |
# | ' '         | 36      |
# | '$'         | 37      |
# | '%'         | 38      |
# | '*'         | 39      |
# | '+'         | 40      |
# | '-'         | 41      |
# | '.'         | 42      |
# | '/'         | 43      |
# | ':'         | 44      |

my $ALPHANUM_CHARS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';

# Pre-compute a lookup hash for O(1) index retrieval.
my %ALPHANUM_INDEX;
{
    my @chars = split //, $ALPHANUM_CHARS;
    for my $i (0 .. $#chars) {
        $ALPHANUM_INDEX{$chars[$i]} = $i;
    }
}

# =============================================================================
# Encoding mode constants
# =============================================================================

use constant {
    MODE_NUMERIC      => 'numeric',
    MODE_ALPHANUMERIC => 'alphanumeric',
    MODE_BYTE         => 'byte',
};

# =============================================================================
# _select_mode($input, $cfg) → mode string
# =============================================================================
#
# Choose the most compact encoding mode supported by $cfg that can represent
# every character in $input.
#
# Priority (most to least compact):
#   1. Numeric      — all characters are ASCII digits 0–9
#   2. Alphanumeric — all characters are in the 45-char set
#   3. Byte         — raw byte encoding (UTF-8 bytes for non-ASCII)
#
# Returns MODE_NUMERIC, MODE_ALPHANUMERIC, or MODE_BYTE.
# Dies with an informative message if no mode is available.

sub _select_mode {
    my ($input, $cfg) = @_;

    # --- Numeric check ---
    # All characters must be ASCII digits 0-9.
    # An empty string qualifies as numeric (zero digits to encode).
    my $is_numeric = ($input =~ /\A[0-9]*\z/);
    if ($is_numeric && $cfg->{cc_numeric} > 0) {
        return MODE_NUMERIC;
    }

    # --- Alphanumeric check ---
    # Every character must appear in the 45-char alphanumeric set.
    my $is_alpha = 1;
    for my $ch (split //, $input) {
        unless (exists $ALPHANUM_INDEX{$ch}) {
            $is_alpha = 0;
            last;
        }
    }
    if ($is_alpha && $cfg->{alpha_cap} > 0) {
        return MODE_ALPHANUMERIC;
    }

    # --- Byte mode ---
    # Raw bytes. All inputs are representable; the question is whether
    # this symbol version supports byte mode.
    if ($cfg->{byte_cap} > 0) {
        return MODE_BYTE;
    }

    die sprintf(
        "MicroQR::UnsupportedMode: input cannot be encoded in any mode "
        . "supported by %s/%s\n",
        $cfg->{version}, $cfg->{ecc}
    );
}

# =============================================================================
# _mode_indicator($mode, $cfg) → integer
# =============================================================================
#
# Return the mode indicator value for the given mode and symbol version.
#
# The mode indicator width (in bits) depends on the symbol:
#
#   M1 (mode_bits=0) — no indicator; only numeric is supported
#   M2 (mode_bits=1) — 1 bit: 0=numeric, 1=alphanumeric
#   M3 (mode_bits=2) — 2 bits: 00=num, 01=alpha, 10=byte
#   M4 (mode_bits=3) — 3 bits: 000=num, 001=alpha, 010=byte, 011=kanji
#
# Kanji is not implemented in this encoder (returns 0x011 if ever requested).

sub _mode_indicator {
    my ($mode, $cfg) = @_;
    my $bits = $cfg->{mode_bits};

    return 0 if $bits == 0;   # M1: no indicator

    if ($bits == 1) {
        return $mode eq MODE_NUMERIC ? 0 : 1;
    }
    if ($bits == 2) {
        return 0 if $mode eq MODE_NUMERIC;
        return 1 if $mode eq MODE_ALPHANUMERIC;
        return 2;   # byte
    }
    if ($bits == 3) {
        return 0 if $mode eq MODE_NUMERIC;
        return 1 if $mode eq MODE_ALPHANUMERIC;
        return 2;   # byte (kanji would be 3, not implemented)
    }

    return 0;
}

# =============================================================================
# _char_count_bits($mode, $cfg) → integer
# =============================================================================
#
# Return the number of bits used for the character count field.

sub _char_count_bits {
    my ($mode, $cfg) = @_;
    return $cfg->{cc_numeric} if $mode eq MODE_NUMERIC;
    return $cfg->{cc_alpha}   if $mode eq MODE_ALPHANUMERIC;
    return $cfg->{cc_byte};
}

# =============================================================================
# BitWriter — accumulate bits MSB-first, flush to bytes
# =============================================================================
#
# The QR/Micro-QR specification uses big-endian bit ordering within each
# codeword. A "bit writer" accumulates bits as individual 0/1 values, then
# packs them into bytes 8 at a time (MSB at the highest index of each byte).
#
# Example:
#   write(5, 4)  → 0 1 0 1   (binary 0101 in 4 bits, MSB first)
#   write(3, 3)  → 0 1 1     (binary 011 in 3 bits, MSB first)
#   to_bytes()   → [0b01010011]  = [0x53]

{
    package CodingAdventures::MicroQR::BitWriter;

    sub new {
        my $class = shift;
        return bless { bits => [] }, $class;
    }

    # Write the `count` least-significant bits of `$value`, MSB first.
    sub write {
        my ($self, $value, $count) = @_;
        for my $i (reverse 0 .. $count - 1) {
            push @{$self->{bits}}, ($value >> $i) & 1;
        }
    }

    # Return the current bit count.
    sub bit_len { return scalar @{$_[0]->{bits}} }

    # Return the raw bit array (arrayref of 0/1 values).
    sub bit_vec { return [@{$_[0]->{bits}}] }

    # Pack the accumulated bits into a byte array (arrayref of integers 0..255).
    # If the total bit count is not a multiple of 8, the final byte is
    # zero-padded on the right (LSB side).
    sub to_bytes {
        my $self = shift;
        my @bits = @{$self->{bits}};
        my @result;
        my $i = 0;
        while ($i < @bits) {
            my $byte = 0;
            for my $j (0 .. 7) {
                $byte = ($byte << 1) | (($i + $j < @bits) ? $bits[$i + $j] : 0);
            }
            push @result, $byte;
            $i += 8;
        }
        return \@result;
    }
}

# =============================================================================
# _encode_numeric($input, $bw)
# =============================================================================
#
# Encode a string of digits into the bit writer.
#
# The encoding groups digits greedily from left to right:
#
#   3 digits (000–999) → 10 bits   (decimal value)
#   2 digits (00–99)   →  7 bits
#   1 digit  (0–9)     →  4 bits
#
# Example: "12345" → groups "123" (123→10 bits) + "45" (45→7 bits) = 17 bits
# Example: "1"     → 1 (4 bits)
# Example: "12"    → 12 (7 bits)

sub _encode_numeric {
    my ($input, $bw) = @_;
    my @digits = map { ord($_) - ord('0') } split //, $input;
    my $i = 0;
    while ($i + 2 <= $#digits) {
        $bw->write($digits[$i] * 100 + $digits[$i+1] * 10 + $digits[$i+2], 10);
        $i += 3;
    }
    if ($i + 1 <= $#digits) {
        $bw->write($digits[$i] * 10 + $digits[$i+1], 7);
        $i += 2;
    }
    if ($i <= $#digits) {
        $bw->write($digits[$i], 4);
    }
}

# =============================================================================
# _encode_alphanumeric($input, $bw)
# =============================================================================
#
# Encode a string using the 45-character alphanumeric set.
#
# Pairs of characters are packed into 11 bits:
#   value = first_index × 45 + second_index
#
# A trailing single character uses 6 bits.
#
# Example: "AC-3"
#   "AC" → index(A)=10, index(C)=12  → 10×45+12 = 462  → 11 bits
#   "-3" → index(-)=41, index(3)=3   → 41×45+3  = 1848 → 11 bits

sub _encode_alphanumeric {
    my ($input, $bw) = @_;
    my @indices = map { $ALPHANUM_INDEX{$_} } split //, $input;
    my $i = 0;
    while ($i + 1 <= $#indices) {
        $bw->write($indices[$i] * 45 + $indices[$i+1], 11);
        $i += 2;
    }
    if ($i <= $#indices) {
        $bw->write($indices[$i], 6);
    }
}

# =============================================================================
# _encode_byte($input, $bw)
# =============================================================================
#
# Encode raw bytes. Each byte of the UTF-8 representation of $input is
# encoded as 8 bits. For ASCII input this is identical to ISO-8859-1 encoding.
#
# Example: "Hi!" → 0x48 0x69 0x21 → 01001000 01101001 00100001

sub _encode_byte {
    my ($input, $bw) = @_;
    for my $byte (unpack 'C*', $input) {
        $bw->write($byte, 8);
    }
}

# =============================================================================
# _build_data_codewords($input, $cfg, $mode) → arrayref of bytes
# =============================================================================
#
# Construct the complete data codeword byte sequence for the given input,
# configuration, and encoding mode.
#
# The bit stream structure is:
#
#   [mode indicator  (0/1/2/3 bits, version-dependent)]
#   [character count (width from char-count table)]
#   [encoded data bits (mode-specific)]
#   [terminator (3/5/7/9 zero bits, truncated if capacity full)]
#   [zero bits to next byte boundary]
#   [pad bytes: alternate 0xEC and 0x11 to fill remaining data codewords]
#
# M1 special case: total data capacity is 20 bits (not 24). The RS encoder
# still receives 3 bytes, but the last byte carries data in its upper nibble
# only; the lower nibble is always zero. There is no 0xEC/0x11 padding for M1.

sub _build_data_codewords {
    my ($input, $cfg, $mode) = @_;

    # Total usable data bit capacity.
    # M1: 3 codewords × 8 bits − 4 = 20 bits (last "codeword" is 4 bits)
    # Others: data_cw × 8 bits
    my $total_bits = $cfg->{m1_half_cw}
        ? ($cfg->{data_cw} * 8 - 4)
        : ($cfg->{data_cw} * 8);

    my $bw = CodingAdventures::MicroQR::BitWriter->new();

    # Mode indicator (0, 1, 2, or 3 bits)
    if ($cfg->{mode_bits} > 0) {
        $bw->write(_mode_indicator($mode, $cfg), $cfg->{mode_bits});
    }

    # Character count (width depends on mode and version)
    # For byte mode: count raw bytes (UTF-8 encoded).
    # For numeric/alphanumeric: count characters (code points).
    my $char_count;
    if ($mode eq MODE_BYTE) {
        $char_count = length(pack 'A*', $input);   # byte length
    } else {
        $char_count = length($input);              # character count
    }
    $bw->write($char_count, _char_count_bits($mode, $cfg));

    # Encoded data
    if    ($mode eq MODE_NUMERIC)      { _encode_numeric($input, $bw) }
    elsif ($mode eq MODE_ALPHANUMERIC) { _encode_alphanumeric($input, $bw) }
    else                               { _encode_byte($input, $bw) }

    # Terminator: up to terminator_bits zero bits, truncated if capacity is full
    my $remaining = $total_bits - $bw->bit_len();
    if ($remaining > 0) {
        my $term = ($cfg->{terminator_bits} < $remaining)
            ? $cfg->{terminator_bits}
            : $remaining;
        $bw->write(0, $term);
    }

    # --- M1 special packing ---
    # Pack exactly 20 bits into 3 bytes.
    # The first two bytes are fully packed (bits 0..15).
    # The third byte carries bits 16..19 in the upper nibble; lower nibble = 0.
    if ($cfg->{m1_half_cw}) {
        my $bits = $bw->bit_vec();
        while (@$bits < 20) { push @$bits, 0 }
        my @b = @{$bits}[0..19];
        my $b0 = ($b[0]<<7)|($b[1]<<6)|($b[2]<<5)|($b[3]<<4)
               | ($b[4]<<3)|($b[5]<<2)|($b[6]<<1)|$b[7];
        my $b1 = ($b[8]<<7)|($b[9]<<6)|($b[10]<<5)|($b[11]<<4)
               | ($b[12]<<3)|($b[13]<<2)|($b[14]<<1)|$b[15];
        my $b2 = ($b[16]<<7)|($b[17]<<6)|($b[18]<<5)|($b[19]<<4);
        return [$b0, $b1, $b2];
    }

    # Pad to byte boundary
    my $rem = $bw->bit_len() % 8;
    if ($rem != 0) {
        $bw->write(0, 8 - $rem);
    }

    # Fill remaining codewords with alternating 0xEC / 0x11
    my $bytes = $bw->to_bytes();
    my $pad = 0xEC;
    while (@$bytes < $cfg->{data_cw}) {
        push @$bytes, $pad;
        $pad = ($pad == 0xEC) ? 0x11 : 0xEC;
    }

    return $bytes;
}

# =============================================================================
# _rs_encode($data, $n_ecc) → arrayref of ECC bytes
# =============================================================================
#
# Compute the Reed-Solomon ECC bytes using polynomial remainder division.
#
# Algorithm — LFSR polynomial division over GF(256)/0x11D:
#
#   ecc = [0] × n
#   for each byte b in data:
#       feedback = b XOR ecc[0]
#       shift ecc left by one (drop ecc[0], append 0)
#       for i in 0..n-1:
#           ecc[i] ^= gf_multiply(generator[i+1], feedback)
#
# This computes the remainder of D(x)·x^n mod G(x), which is the standard
# RS ECC computation for the b=0 convention (first root is α^0 = 1).
#
# The same algorithm is used by regular QR Code — Micro QR shares the same
# GF(256) field, primitive polynomial 0x11D, and generator convention.

sub _rs_encode {
    my ($data, $n_ecc) = @_;
    my $gen = $RS_GENERATORS{$n_ecc}
        or die "MicroQR: no RS generator for n_ecc=$n_ecc\n";

    my @rem = (0) x $n_ecc;

    for my $b (@$data) {
        my $fb = $b ^ $rem[0];
        # Shift register left: drop rem[0], shift rest left, append 0
        @rem = (@rem[1..$#rem], 0);
        if ($fb != 0) {
            for my $i (0 .. $n_ecc - 1) {
                $rem[$i] ^= multiply($gen->[$i + 1], $fb);
            }
        }
    }

    return \@rem;
}

# =============================================================================
# _select_config($input, $version, $ecc) → $cfg hashref
# =============================================================================
#
# Find the smallest symbol configuration that can hold the given input.
#
# If $version and/or $ecc are provided, only configurations matching those
# constraints are considered.
#
# The configurations in @SYMBOL_CONFIGS are ordered smallest-first, so the
# first fit is automatically the smallest valid symbol.
#
# Dies with an informative error if no configuration fits the input.

sub _select_config {
    my ($input, $version, $ecc) = @_;

    # Filter to matching configurations
    my @candidates = grep {
        (!defined $version || $_->{version} eq $version) &&
        (!defined $ecc     || $_->{ecc}     eq $ecc)
    } @SYMBOL_CONFIGS;

    unless (@candidates) {
        die sprintf(
            "MicroQR::ECCNotAvailable: no symbol configuration matches "
            . "version=%s ecc=%s\n",
            $version // 'any', $ecc // 'any'
        );
    }

    for my $cfg (@candidates) {
        # Determine what mode would be used
        my $mode = eval { _select_mode($input, $cfg) };
        next unless defined $mode;   # mode not supported for this config

        # Determine character count (byte count for byte mode)
        my $len;
        if ($mode eq MODE_BYTE) {
            $len = length(pack 'A*', $input);
        } else {
            my @chars = split //, $input;
            $len = scalar @chars;
        }

        # Check capacity
        my $cap = ($mode eq MODE_NUMERIC)      ? $cfg->{numeric_cap}
                : ($mode eq MODE_ALPHANUMERIC)  ? $cfg->{alpha_cap}
                :                                 $cfg->{byte_cap};

        return $cfg if $cap > 0 && $len <= $cap;
    }

    die sprintf(
        "MicroQR::InputTooLong: input (length %d) does not fit in any "
        . "Micro QR symbol (version=%s, ecc=%s). "
        . "Maximum is 35 numeric chars in M4-L.\n",
        length($input), $version // 'any', $ecc // 'any'
    );
}

# =============================================================================
# Grid helpers
# =============================================================================
#
# The working grid holds two parallel 2D arrays:
#
#   $modules[row][col]  — 0 (light) or 1 (dark)
#   $reserved[row][col] — boolean: true means the module is structural
#                         (finder, separator, timing, or format info)
#
# Structural modules are never overwritten by data or modified by masking.
# The reserved flag is the authoritative gate for both data placement and
# masking.

sub _make_working_grid {
    my $size = shift;
    my @modules  = map { [(0) x $size] } 1..$size;
    my @reserved = map { [(0) x $size] } 1..$size;
    return (\@modules, \@reserved);
}

# =============================================================================
# _place_finder($modules, $reserved, $size)
# =============================================================================
#
# Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
#
# The finder pattern is identical to the one used in regular QR Code:
#
#   ■ ■ ■ ■ ■ ■ ■
#   ■ □ □ □ □ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ □ □ □ □ ■
#   ■ ■ ■ ■ ■ ■ ■
#
# The outer ring (border) is always dark. The inner 5×5 ring is always light.
# The innermost 3×3 core is always dark. This 1:1:3:1:1 ratio is what scanners
# look for to detect a finder pattern.

sub _place_finder {
    my ($modules, $reserved) = @_;
    for my $dr (0..6) {
        for my $dc (0..6) {
            my $on_border = ($dr == 0 || $dr == 6 || $dc == 0 || $dc == 6);
            my $in_core   = ($dr >= 2 && $dr <= 4 && $dc >= 2 && $dc <= 4);
            $modules->[$dr][$dc]  = ($on_border || $in_core) ? 1 : 0;
            $reserved->[$dr][$dc] = 1;
        }
    }
}

# =============================================================================
# _place_separator($modules, $reserved)
# =============================================================================
#
# Place the L-shaped separator (all-light modules).
#
# Unlike regular QR Code which surrounds all three finder patterns with a
# full ring of separator modules, Micro QR has only one finder in the
# top-left corner. Its top edge and left edge are the symbol boundary itself,
# so only the bottom and right edges need separators.
#
# Separator positions:
#   Row 7, cols 0–7  — bottom of finder
#   Col 7, rows 0–7  — right of finder
#
# The corner module at (row=7, col=7) is shared by both; it is always light.

sub _place_separator {
    my ($modules, $reserved) = @_;
    for my $i (0..7) {
        $modules->[7][$i] = 0;  $reserved->[7][$i] = 1;  # bottom row
        $modules->[$i][7] = 0;  $reserved->[$i][7] = 1;  # right column
    }
}

# =============================================================================
# _place_timing($modules, $reserved, $size)
# =============================================================================
#
# Place timing pattern extensions along row 0 and col 0.
#
# In regular QR Code, timing patterns run along row 6 and column 6. In Micro
# QR, they run along the OUTER edge: row 0 and column 0.
#
# Positions 0–6 are already occupied by the finder pattern. Position 7 is the
# separator (always light). The timing pattern extends from position 8 to the
# end of the symbol:
#
#   Position k (from 0): dark if k is even, light if k is odd
#   Col 8 (even) is dark; col 9 (odd) is light; col 10 (even) is dark; …
#
# This produces the alternating dark/light "teeth" that timing patterns are
# named for. Scanners use them to precisely locate module boundaries.

sub _place_timing {
    my ($modules, $reserved, $size) = @_;
    for my $c (8..$size-1) {
        $modules->[0][$c]  = ($c % 2 == 0) ? 1 : 0;
        $reserved->[0][$c] = 1;
    }
    for my $r (8..$size-1) {
        $modules->[$r][0]  = ($r % 2 == 0) ? 1 : 0;
        $reserved->[$r][0] = 1;
    }
}

# =============================================================================
# _reserve_format_info($modules, $reserved)
# =============================================================================
#
# Mark the 15 format information module positions as reserved.
#
# The format information strip forms an L-shape adjacent to the finder's
# separator:
#
#   Row 8, cols 1–8  →  8 modules  (bits f14 down to f7, MSB first)
#   Col 8, rows 1–7  →  7 modules  (bits f6 down to f0, with f6 at row 7)
#
# These 15 positions match exactly the 15 bits of the format information word.
# They are initialized to light (0) here; the actual values are written after
# mask selection is complete.

sub _reserve_format_info {
    my ($modules, $reserved) = @_;
    for my $c (1..8) {
        $modules->[8][$c]  = 0;
        $reserved->[8][$c] = 1;
    }
    for my $r (1..7) {
        $modules->[$r][8]  = 0;
        $reserved->[$r][8] = 1;
    }
}

# =============================================================================
# _write_format_info($modules, $fmt)
# =============================================================================
#
# Write a 15-bit format word into the reserved format information positions.
#
# Bit placement (f14 = MSB, f0 = LSB):
#
#   Row 8, col 1  ← f14   Row 8, col 5  ← f10   Col 8, row 6  ← f5
#   Row 8, col 2  ← f13   Row 8, col 6  ← f9    Col 8, row 5  ← f4
#   Row 8, col 3  ← f12   Row 8, col 7  ← f8    Col 8, row 4  ← f3
#   Row 8, col 4  ← f11   Row 8, col 8  ← f7    Col 8, row 3  ← f2
#                          Col 8, row 7  ← f6    Col 8, row 2  ← f1
#                                                 Col 8, row 1  ← f0
#
# There is only ONE copy of the format information in Micro QR (unlike regular
# QR which places it in two locations for redundancy).

sub _write_format_info {
    my ($modules, $fmt) = @_;
    # Row 8, cols 1–8: bits f14 down to f7
    for my $i (0..7) {
        $modules->[8][1 + $i] = ($fmt >> (14 - $i)) & 1;
    }
    # Col 8, rows 7 down to 1: bits f6 down to f0
    for my $i (0..6) {
        $modules->[7 - $i][8] = ($fmt >> (6 - $i)) & 1;
    }
}

# =============================================================================
# _build_grid($cfg) → ($modules, $reserved)
# =============================================================================
#
# Initialize the symbol grid with all structural modules in place.
#
# Call order matters:
#   1. Finder (rows 0–6, cols 0–6)
#   2. Separator (row 7 cols 0–7, col 7 rows 0–7)
#   3. Timing extensions (row 0 and col 0 from position 8 onward)
#   4. Format info reservation (row 8 cols 1–8, col 8 rows 1–7)
#
# Later steps will add data bits and mask, then write the format info.

sub _build_grid {
    my $cfg = shift;
    my $size = $cfg->{size};
    my ($modules, $reserved) = _make_working_grid($size);
    _place_finder($modules, $reserved);
    _place_separator($modules, $reserved);
    _place_timing($modules, $reserved, $size);
    _reserve_format_info($modules, $reserved);
    return ($modules, $reserved);
}

# =============================================================================
# _place_bits($modules, $reserved, $bits, $size)
# =============================================================================
#
# Place data/ECC bits into the grid via a two-column zigzag scan.
#
# The scan starts at the bottom-right corner and moves upward, scanning two
# columns at a time. After each vertical scan, direction reverses and the
# scan moves two columns to the left.
#
# Pictorially for a small grid (columns numbered right-to-left):
#
#   col:  ... 5  4  3  2  1  0
#              ↑  ↑  ↑  ↑  ...
#   pass1: cols 5,4  — upward   (then flip)
#   pass2: cols 3,2  — downward (then flip)
#   pass3: cols 1,0  — upward
#
# Reserved modules are skipped silently. Any bits left after all non-reserved
# modules are filled are discarded (this handles the 4 remainder bits in M1,
# which are left as 0 since the array only has data+ecc bits).
#
# Note: Unlike regular QR, there is NO timing column at col 6 to hop over.
# Micro QR's timing is at col 0 (reserved), so it is auto-skipped.

sub _place_bits {
    my ($modules, $reserved, $bits, $size) = @_;
    my $bit_idx = 0;
    my $up = 1;   # 1 = scanning upward, 0 = downward

    my $col = $size - 1;
    while ($col >= 1) {
        # Scan this two-column strip in the current direction
        for my $vi (0..$size-1) {
            my $row = $up ? ($size - 1 - $vi) : $vi;
            for my $dc (0, 1) {
                my $c = $col - $dc;
                next if $reserved->[$row][$c];
                $modules->[$row][$c] = ($bit_idx < @$bits) ? $bits->[$bit_idx++] : 0;
            }
        }
        $up = !$up;
        $col -= 2;
    }
}

# =============================================================================
# _mask_condition($mask_idx, $row, $col) → boolean
# =============================================================================
#
# Return true if mask pattern $mask_idx applies to module at ($row, $col).
#
# Micro QR uses only 4 mask patterns — the first four of regular QR's eight:
#
#   Pattern 0: (row + col) mod 2 == 0   — checkerboard
#   Pattern 1:  row mod 2 == 0          — alternate rows
#   Pattern 2:  col mod 3 == 0          — every third column
#   Pattern 3: (row + col) mod 3 == 0   — diagonal thirds
#
# When the condition is true for a data/ECC module, that module's value is
# flipped (dark↔light). Structural modules are never masked.

sub _mask_condition {
    my ($mask_idx, $row, $col) = @_;
    return 0 if $mask_idx == 0 && ($row + $col) % 2 != 0;
    return 1 if $mask_idx == 0;

    return 0 if $mask_idx == 1 && $row % 2 != 0;
    return 1 if $mask_idx == 1;

    return 0 if $mask_idx == 2 && $col % 3 != 0;
    return 1 if $mask_idx == 2;

    return 0 if $mask_idx == 3 && ($row + $col) % 3 != 0;
    return 1 if $mask_idx == 3;

    return 0;
}

# =============================================================================
# _apply_mask($modules, $reserved, $size, $mask_idx) → new modules arrayref
# =============================================================================
#
# Apply a mask pattern to all non-reserved modules. Returns a new 2D array
# (does not modify the input).

sub _apply_mask {
    my ($modules, $reserved, $size, $mask_idx) = @_;
    my @result;
    for my $r (0..$size-1) {
        push @result, [@{$modules->[$r]}];
    }
    for my $r (0..$size-1) {
        for my $c (0..$size-1) {
            unless ($reserved->[$r][$c]) {
                if (_mask_condition($mask_idx, $r, $c)) {
                    $result[$r][$c] = $result[$r][$c] ? 0 : 1;
                }
            }
        }
    }
    return \@result;
}

# =============================================================================
# _compute_penalty($modules, $size) → integer
# =============================================================================
#
# Compute the 4-rule penalty score for a candidate masked grid.
# The mask with the lowest score is selected.
#
# The four penalty rules (identical to regular QR Code):
#
#   Rule 1 — Adjacent same-color run penalty:
#     For each row and column, find runs of ≥5 consecutive same-color modules.
#     Each qualifying run contributes (run_length − 2) to the penalty.
#
#     A run of exactly 5 → +3
#     A run of 6 → +4
#     A run of 7 → +5   … and so on.
#
#   Rule 2 — 2×2 block penalty:
#     For each 2×2 square where all four modules are the same color, add 3.
#
#   Rule 3 — Finder-pattern-like sequences:
#     For each row and column, check for the 11-module patterns:
#       P1: 1 0 1 1 1 0 1 0 0 0 0
#       P2: 0 0 0 0 1 0 1 1 1 0 1  (reverse of P1)
#     Each occurrence (horizontal or vertical) adds 40.
#     These sequences look like finder patterns and confuse scanners.
#
#   Rule 4 — Dark module proportion penalty:
#     Compute the percentage of dark modules in the symbol.
#     Penalty = min(|prev5 − 50|, |next5 − 50|) / 5 × 10
#     where prev5 and next5 are the multiples of 5 surrounding dark_pct.
#     Penalty is 0 at exactly 50% dark, escalates as the balance shifts.

sub _compute_penalty {
    my ($modules, $size) = @_;
    my $penalty = 0;

    # --- Rule 1: same-color runs of ≥ 5 ---
    for my $a (0..$size-1) {
        for my $horiz (0, 1) {
            my $run = 1;
            my $prev = $horiz ? $modules->[$a][0] : $modules->[0][$a];
            for my $i (1..$size-1) {
                my $cur = $horiz ? $modules->[$a][$i] : $modules->[$i][$a];
                if ($cur == $prev) {
                    $run++;
                } else {
                    $penalty += ($run - 2) if $run >= 5;
                    $run = 1;
                    $prev = $cur;
                }
            }
            $penalty += ($run - 2) if $run >= 5;
        }
    }

    # --- Rule 2: 2×2 same-color blocks ---
    for my $r (0..$size-2) {
        for my $c (0..$size-2) {
            my $d = $modules->[$r][$c];
            if ($d == $modules->[$r][$c+1] &&
                $d == $modules->[$r+1][$c] &&
                $d == $modules->[$r+1][$c+1]) {
                $penalty += 3;
            }
        }
    }

    # --- Rule 3: finder-pattern-like sequences ---
    my @P1 = (1,0,1,1,1,0,1,0,0,0,0);
    my @P2 = (0,0,0,0,1,0,1,1,1,0,1);

    for my $a (0..$size-1) {
        my $limit = ($size >= 11) ? $size - 11 : 0;
        for my $b (0..$limit) {
            my ($mh1, $mh2, $mv1, $mv2) = (1,1,1,1);
            for my $k (0..10) {
                my $bh = $modules->[$a][$b+$k];
                my $bv = $modules->[$b+$k][$a];
                $mh1 = 0 unless $bh == $P1[$k];
                $mh2 = 0 unless $bh == $P2[$k];
                $mv1 = 0 unless $bv == $P1[$k];
                $mv2 = 0 unless $bv == $P2[$k];
            }
            $penalty += 40 if $mh1;
            $penalty += 40 if $mh2;
            $penalty += 40 if $mv1;
            $penalty += 40 if $mv2;
        }
    }

    # --- Rule 4: dark-module proportion ---
    my $dark = 0;
    for my $r (0..$size-1) {
        for my $c (0..$size-1) {
            $dark++ if $modules->[$r][$c];
        }
    }
    my $total    = $size * $size;
    my $dark_pct = int($dark * 100 / $total);
    my $prev5    = int($dark_pct / 5) * 5;
    my $next5    = $prev5 + 5;
    my $a_dist   = abs($prev5 - 50);
    my $b_dist   = abs($next5 - 50);
    my $r4_raw   = $a_dist < $b_dist ? $a_dist : $b_dist;
    $penalty += int($r4_raw / 5) * 10;

    return $penalty;
}

# =============================================================================
# _flatten_to_bits($final_cw, $cfg) → arrayref of 0/1 values
# =============================================================================
#
# Flatten the final codeword sequence (data + ECC) to a bit array.
#
# Each codeword is expanded MSB-first (big-endian within codeword). The only
# special case is M1: the third data codeword (index 2) contributes only 4 bits
# (the upper nibble), not 8. All ECC codewords are always full 8 bits.

sub _flatten_to_bits {
    my ($final_cw, $cfg) = @_;
    my @bits;
    for my $cw_idx (0..$#$final_cw) {
        my $cw = $final_cw->[$cw_idx];
        # M1: the last data codeword (index data_cw - 1) is a 4-bit nibble
        my $bits_in_cw = ($cfg->{m1_half_cw} && $cw_idx == $cfg->{data_cw} - 1)
            ? 4 : 8;
        # Extract bits MSB first, right-aligning within the byte
        for my $b (reverse 0 .. $bits_in_cw - 1) {
            push @bits, ($cw >> ($b + (8 - $bits_in_cw))) & 1;
        }
    }
    return \@bits;
}

# =============================================================================
# encode($input, $version, $ecc) → ModuleGrid hashref
# =============================================================================
#
# Encode a string to a Micro QR Code ModuleGrid.
#
# Automatically selects the smallest symbol (M1..M4) and mode that can hold
# the input. Pass $version (M1/M2/M3/M4) and/or $ecc (L/M/Q/Detection) to
# override auto-selection.
#
# Returns a ModuleGrid hashref:
#   {
#     rows   => $size,    # e.g. 11 for M1
#     cols   => $size,
#     modules => \@grid,  # 2D array of 0/1 values
#     module_shape => 'square',
#   }
#
# Dies (with a descriptive message) on any encoding error.
#
# Examples:
#
#   my $grid = encode("1");
#   # $grid->{rows} == 11 (M1, smallest symbol)
#
#   my $grid = encode("HELLO");
#   # $grid->{rows} == 13 (M2-L, alphanumeric mode)
#
#   my $grid = encode("https://a.b", M4, ECC_L);
#   # $grid->{rows} == 17 (M4-L, byte mode)

sub encode {
    my ($input, $version, $ecc) = @_;

    # 1. Select symbol configuration
    my $cfg = _select_config($input, $version, $ecc);
    my $mode = _select_mode($input, $cfg);

    # 2. Build data codewords
    my $data_cw = _build_data_codewords($input, $cfg, $mode);

    # 3. Compute RS ECC
    my $ecc_cw = _rs_encode($data_cw, $cfg->{ecc_cw});

    # 4. Flatten to bit stream
    my @final_cw = (@$data_cw, @$ecc_cw);
    my $bits = _flatten_to_bits(\@final_cw, $cfg);

    # 5. Initialize grid
    my ($modules, $reserved) = _build_grid($cfg);

    # 6. Place data bits
    _place_bits($modules, $reserved, $bits, $cfg->{size});

    # 7. Evaluate all 4 masks, pick the one with the lowest penalty score
    #    Ties are broken by preferring the lower-numbered mask.
    my $best_mask    = 0;
    my $best_penalty = 2**31;   # large initial value

    for my $m (0..3) {
        my $masked = _apply_mask($modules, $reserved, $cfg->{size}, $m);
        my $fmt    = $FORMAT_TABLE[$cfg->{symbol_indicator}][$m];
        # Write format info into a temporary copy of the masked modules
        my @tmp;
        for my $r (0..$cfg->{size}-1) {
            push @tmp, [@{$masked->[$r]}];
        }
        _write_format_info(\@tmp, $fmt);
        my $p = _compute_penalty(\@tmp, $cfg->{size});
        if ($p < $best_penalty) {
            $best_penalty = $p;
            $best_mask    = $m;
        }
    }

    # 8. Apply best mask and write final format information
    my $final_modules = _apply_mask($modules, $reserved, $cfg->{size}, $best_mask);
    my $final_fmt     = $FORMAT_TABLE[$cfg->{symbol_indicator}][$best_mask];
    _write_format_info($final_modules, $final_fmt);

    # 9. Return as a ModuleGrid hashref (CodingAdventures::Barcode2D format)
    return {
        rows         => $cfg->{size},
        cols         => $cfg->{size},
        modules      => $final_modules,
        module_shape => CodingAdventures::Barcode2D::SHAPE_SQUARE,
    };
}

# =============================================================================
# encode_at($input, $version, $ecc) → ModuleGrid hashref
# =============================================================================
#
# Encode to a specific symbol version. Both $version and $ecc are required.
# Dies if the input does not fit in the requested version/ECC combination.

sub encode_at {
    my ($input, $version, $ecc) = @_;
    croak "encode_at: version is required" unless defined $version;
    croak "encode_at: ecc is required"     unless defined $ecc;
    return encode($input, $version, $ecc);
}

# =============================================================================
# layout_grid($grid, $config) → PaintScene hashref
# =============================================================================
#
# Convert a ModuleGrid to a PaintScene via CodingAdventures::Barcode2D::layout().
#
# The Micro QR quiet zone is 2 modules (not QR's default of 4), so this
# function overrides the quiet_zone_modules default to 2.
#
# $config is an optional hashref with layout parameters. If omitted, sensible
# defaults are used (quiet_zone=2, module_size=10, black on white).

sub layout_grid {
    my ($grid, $config) = @_;
    $config //= {};
    $config->{quiet_zone_modules} //= 2;   # Micro QR uses half the QR quiet zone
    return CodingAdventures::Barcode2D::layout($grid, $config);
}

1;

__END__

=head1 NAME

CodingAdventures::MicroQR - Micro QR Code encoder (ISO/IEC 18004:2015 Annex E)

=head1 VERSION

0.1.0

=head1 SYNOPSIS

  use CodingAdventures::MicroQR qw(encode encode_at layout_grid M1 M2 M3 M4
                                    ECC_L ECC_M ECC_Q DETECTION);

  # Auto-select smallest symbol
  my $grid = encode("HELLO");   # returns 13x13 M2 grid

  # Single digit → M1 (11x11, detection-only ECC)
  my $grid = encode("1");

  # Byte mode URL → M4
  my $grid = encode("https://a.b", M4, ECC_L);

  # Forced version + ECC
  my $grid = encode_at("12345", M1, DETECTION);

  # Convert to PaintScene for rendering
  my $scene = layout_grid($grid);

=head1 DESCRIPTION

Encodes arbitrary strings to ISO/IEC 18004:2015 Annex E compliant Micro QR
Code symbols. Supports all four symbol sizes (M1–M4), all available ECC levels
(Detection, L, M, Q), and three encoding modes (numeric, alphanumeric, byte).

=head1 FUNCTIONS

=head2 encode($input, $version, $ecc)

Encode $input to a ModuleGrid. $version (M1..M4) and $ecc (ECC_L/ECC_M/ECC_Q/
DETECTION) are optional; if omitted, the smallest fitting symbol is chosen.

=head2 encode_at($input, $version, $ecc)

Like encode() but requires both $version and $ecc explicitly.

=head2 layout_grid($grid, $config)

Convert a ModuleGrid to a PaintScene. Uses quiet_zone=2 by default.

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
