package CodingAdventures::MicroQR;

# =============================================================================
# CodingAdventures::MicroQR — ISO/IEC 18004:2015 Annex E compliant Micro QR encoder
# =============================================================================
#
# Micro QR Code is the compact cousin of the full QR Code, designed for
# applications where even the smallest standard QR (21×21) is too large.
# Think surface-mount component labels, circuit board markings, or miniature
# industrial tags on watch parts.
#
# ## Symbol sizes
#
#   M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
#   Formula: size = 2 × version_number + 9
#
# ## Key differences from full QR Code
#
#   1. SINGLE finder pattern (top-left only) — no top-right or bottom-left.
#   2. Timing strips at ROW 0 and COL 0, not row 6/col 6 as in full QR.
#   3. Only 4 mask patterns (not 8).
#   4. Format XOR mask is 0x4445 (not 0x5412).
#   5. Single copy of format information (not two).
#   6. 2-module quiet zone (not 4).
#   7. Narrower mode indicators: 0–3 bits (symbol-dependent).
#   8. Single RS block per symbol (no interleaving needed).
#
# ## Encoding pipeline
#
#   input string
#     → auto-select smallest symbol (M1..M4) and encoding mode
#     → build bit stream (mode indicator + char count + data + terminator + pad)
#     → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
#     → init grid (finder, L-shaped separator, timing row0/col0, format reserved)
#     → zigzag data placement (two-column snake from bottom-right)
#     → evaluate 4 mask patterns, pick lowest penalty score
#     → write format information (15 bits, single copy, XOR 0x4445)
#     → return ModuleGrid hashref
#
# ## Encoding modes (subset available per symbol)
#
#   numeric      — digits 0-9 only.  3 digits → 10 bits, 2 → 7 bits, 1 → 4 bits.
#   alphanumeric — 45-char set (0-9, A-Z, space, $%*+-./:). Pairs → 11 bits, single → 6 bits.
#   byte         — raw bytes (ASCII or UTF-8). Each byte → 8 bits.
#
# ## Building blocks
#
#   P2D01 barcode-2d — ModuleGrid representation and layout() rendering
#   MA01  gf256      — GF(2^8) field arithmetic for Reed-Solomon
#
# ## Reference
#
#   ISO/IEC 18004:2015, Annex E (Micro QR Code)
#   Rust reference implementation: code/packages/rust/micro-qr/
#
# =============================================================================

use strict;
use warnings;
use Carp qw(croak);
use List::Util ();

use CodingAdventures::Barcode2D ();
use CodingAdventures::GF256     ();

our $VERSION = '0.1.0';

# =============================================================================
# Public exports
# =============================================================================

use Exporter 'import';
our @EXPORT_OK = qw(encode encode_at layout_grid);

# =============================================================================
# Symbol version constants (exported as sub constants)
# =============================================================================
#
# Each Micro QR symbol has a "version number" that governs its size:
#   M1 → 11×11 modules
#   M2 → 13×13 modules
#   M3 → 15×15 modules
#   M4 → 17×17 modules
#
# In code we represent these as small integers 1-4.
# The `use constant` pragma creates zero-argument subs that return the value;
# under `use strict` these MUST be called with parens: M1() not M1.

use constant {
    M1 => 1,
    M2 => 2,
    M3 => 3,
    M4 => 4,
};

# =============================================================================
# ECC level constants
# =============================================================================
#
# Micro QR supports only a subset of QR's four ECC levels:
#
#   DETECTION — M1 only.  No correction; detects single-codeword errors.
#   ECC_L     — M2, M3, M4.  ~7% codeword recovery.
#   ECC_M     — M2, M3, M4.  ~15% codeword recovery.
#   ECC_Q     — M4 only.     ~25% codeword recovery.
#
# Level H is not available in Micro QR.

use constant {
    DETECTION => 'D',
    ECC_L     => 'L',
    ECC_M     => 'M',
    ECC_Q     => 'Q',
};

# =============================================================================
# Encoding mode identifiers (private)
# =============================================================================
#
# These are internal string tags used throughout the module.

use constant {
    MODE_NUMERIC      => 'numeric',
    MODE_ALPHANUMERIC => 'alphanumeric',
    MODE_BYTE         => 'byte',
};

# =============================================================================
# Symbol configuration table
# =============================================================================
#
# There are exactly 8 valid (version, ECC) combinations in Micro QR.
# Each has its own set of parameters:
#
#   symbol_indicator — 3-bit field embedded in format information (0..7).
#   size             — symbol side length in modules.
#   data_cw          — number of data codewords (8-bit bytes, except M1 uses 2.5).
#   ecc_cw           — number of Reed-Solomon ECC codewords.
#   numeric_cap      — max numeric characters (0 = not supported).
#   alpha_cap        — max alphanumeric characters.
#   byte_cap         — max byte characters.
#   terminator_bits  — bits in the end-of-data terminator (3/5/7/9).
#   mode_bits        — width of the mode indicator field (0=M1, 1=M2, 2=M3, 3=M4).
#   cc_num           — char-count field width for numeric mode.
#   cc_alpha         — char-count field width for alphanumeric mode.
#   cc_byte          — char-count field width for byte mode.
#   m1_half_cw       — true only for M1: last data "codeword" is a 4-bit nibble.
#
# Source: ISO/IEC 18004:2015 Tables E.1–E.5.

my @CONFIGS = (
    # M1 / Detection
    {
        version => M1, ecc => DETECTION,
        symbol_indicator => 0, size => 11,
        data_cw => 3, ecc_cw => 2,
        numeric_cap => 5, alpha_cap => 0, byte_cap => 0,
        terminator_bits => 3, mode_bits => 0,
        cc_num => 3, cc_alpha => 0, cc_byte => 0,
        m1_half_cw => 1,
    },
    # M2 / L
    {
        version => M2, ecc => ECC_L,
        symbol_indicator => 1, size => 13,
        data_cw => 5, ecc_cw => 5,
        numeric_cap => 10, alpha_cap => 6, byte_cap => 4,
        terminator_bits => 5, mode_bits => 1,
        cc_num => 4, cc_alpha => 3, cc_byte => 4,
        m1_half_cw => 0,
    },
    # M2 / M
    {
        version => M2, ecc => ECC_M,
        symbol_indicator => 2, size => 13,
        data_cw => 4, ecc_cw => 6,
        numeric_cap => 8, alpha_cap => 5, byte_cap => 3,
        terminator_bits => 5, mode_bits => 1,
        cc_num => 4, cc_alpha => 3, cc_byte => 4,
        m1_half_cw => 0,
    },
    # M3 / L
    {
        version => M3, ecc => ECC_L,
        symbol_indicator => 3, size => 15,
        data_cw => 11, ecc_cw => 6,
        numeric_cap => 23, alpha_cap => 14, byte_cap => 9,
        terminator_bits => 7, mode_bits => 2,
        cc_num => 5, cc_alpha => 4, cc_byte => 4,
        m1_half_cw => 0,
    },
    # M3 / M
    {
        version => M3, ecc => ECC_M,
        symbol_indicator => 4, size => 15,
        data_cw => 9, ecc_cw => 8,
        numeric_cap => 18, alpha_cap => 11, byte_cap => 7,
        terminator_bits => 7, mode_bits => 2,
        cc_num => 5, cc_alpha => 4, cc_byte => 4,
        m1_half_cw => 0,
    },
    # M4 / L
    {
        version => M4, ecc => ECC_L,
        symbol_indicator => 5, size => 17,
        data_cw => 16, ecc_cw => 8,
        numeric_cap => 35, alpha_cap => 21, byte_cap => 15,
        terminator_bits => 9, mode_bits => 3,
        cc_num => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0,
    },
    # M4 / M
    {
        version => M4, ecc => ECC_M,
        symbol_indicator => 6, size => 17,
        data_cw => 14, ecc_cw => 10,
        numeric_cap => 30, alpha_cap => 18, byte_cap => 13,
        terminator_bits => 9, mode_bits => 3,
        cc_num => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0,
    },
    # M4 / Q
    {
        version => M4, ecc => ECC_Q,
        symbol_indicator => 7, size => 17,
        data_cw => 10, ecc_cw => 14,
        numeric_cap => 21, alpha_cap => 13, byte_cap => 9,
        terminator_bits => 9, mode_bits => 3,
        cc_num => 6, cc_alpha => 5, cc_byte => 5,
        m1_half_cw => 0,
    },
);

# =============================================================================
# Pre-computed format information table
# =============================================================================
#
# Each of the 32 combinations of (symbol_indicator × mask_pattern) has a
# unique pre-computed 15-bit format word. Pre-computing avoids BCH polynomial
# division at encode time.
#
# FORMAT_TABLE[$si][$mp] gives the word already XOR-masked with 0x4445.
#
# The 15-bit format word structure:
#   bits 14-12: symbol_indicator (3 bits)
#   bits 11-10: mask_pattern (2 bits)
#   bits  9- 0: BCH-10 error-protection bits (computed from the 5-bit data above)
# XOR-masked with 0x4445 (Micro QR specific, NOT 0x5412 like regular QR).
#
# Values verified against Rust reference implementation.

my @FORMAT_TABLE = (
    [0x4445, 0x4172, 0x4E2B, 0x4B1C],  # si=0 (M1/Detection)
    [0x5528, 0x501F, 0x5F46, 0x5A71],  # si=1 (M2/L)
    [0x6649, 0x637E, 0x6C27, 0x6910],  # si=2 (M2/M)
    [0x7764, 0x7253, 0x7D0A, 0x783D],  # si=3 (M3/L)
    [0x06DE, 0x03E9, 0x0CB0, 0x0987],  # si=4 (M3/M)
    [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  # si=5 (M4/L)
    [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  # si=6 (M4/M)
    [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  # si=7 (M4/Q)
);

# =============================================================================
# Alphanumeric character set
# =============================================================================
#
# 45 characters in QR/Micro QR's alphanumeric encoding mode, indexed 0-44.
# Character pair (a, b) encodes to: a_idx * 45 + b_idx → 11 bits.

my $ALPHANUM_CHARS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';

# =============================================================================
# Symbol selection
# =============================================================================

# _select_config — find the smallest (version, ECC) config that fits the input.
#
# If $version and/or $ecc are provided, only configs matching those constraints
# are considered. Among the matching candidates (ordered smallest-first), the
# first one where both the mode is supported and the input fits is returned.
#
# The candidates array is already ordered smallest-first (M1..M4, L before M,
# M before Q within the same version) so the first fit is the optimal choice.
#
# Returns a hashref from @CONFIGS, or dies with a descriptive message.
sub _select_config {
    my ($input, $version, $ecc) = @_;

    my @candidates = grep {
        (!defined $version || $_->{version} == $version) &&
        (!defined $ecc     || $_->{ecc}     eq $ecc    )
    } @CONFIGS;

    croak "ECCNotAvailable: no Micro QR symbol matches version=${\(defined $version ? $version : 'any')}"
        . " ecc=${\(defined $ecc ? $ecc : 'any')}"
        unless @candidates;

    for my $cfg (@candidates) {
        my $mode = _select_mode($input, $cfg);
        next unless defined $mode;

        my $len = ($mode eq MODE_BYTE)
            ? length(pack 'A*', $input)
            : length($input);
        my $cap = $mode eq MODE_NUMERIC      ? $cfg->{numeric_cap}
                : $mode eq MODE_ALPHANUMERIC ? $cfg->{alpha_cap}
                :                              $cfg->{byte_cap};
        next unless $cap > 0 && $len <= $cap;
        return $cfg;
    }

    croak "InputTooLong: input (length " . length($input) . ") does not fit in any"
        . " Micro QR symbol. Maximum is 35 numeric chars in M4-L.";
}

# _select_mode — choose the most compact encoding mode supported by $cfg.
#
# Priority: numeric (most compact) → alphanumeric → byte (least compact).
# Returns undef if the input cannot be encoded in any mode $cfg supports.
sub _select_mode {
    my ($input, $cfg) = @_;

    # Numeric: all characters are ASCII digits 0-9.
    if ($input =~ /^\d*$/ && $cfg->{cc_num} > 0) {
        return MODE_NUMERIC;
    }

    # Alphanumeric: all characters are in the 45-char QR set.
    if ($cfg->{alpha_cap} > 0 && _all_alphanum($input)) {
        return MODE_ALPHANUMERIC;
    }

    # Byte: raw bytes.
    if ($cfg->{byte_cap} > 0) {
        return MODE_BYTE;
    }

    return undef;  # no supported mode
}

# _all_alphanum — return 1 if every character in $s is in the QR alphanumeric set.
sub _all_alphanum {
    my ($s) = @_;
    for my $c (split //, $s) {
        return 0 if index($ALPHANUM_CHARS, $c) < 0;
    }
    return 1;
}

# =============================================================================
# Inner package: BitWriter
# =============================================================================
#
# A simple MSB-first bit accumulator. Callers write (value, count) pairs;
# the writer appends the `count` least-significant bits of `value` in
# big-endian (MSB-first) order. This matches the QR Code bit convention
# throughout.
#
# Example:
#   my $w = CodingAdventures::MicroQR::BitWriter->new;
#   $w->write(0b101, 3);   # appends bits: 1, 0, 1
#   $w->to_bytes();         # returns [0b10100000]  (packed into first byte)

package CodingAdventures::MicroQR::BitWriter;

sub new  { bless { bits => [] }, shift }

# write — append the $count LSBs of $value, MSB first.
sub write {
    my ($self, $value, $count) = @_;
    for my $i (reverse 0 .. $count - 1) {
        push @{ $self->{bits} }, ($value >> $i) & 1;
    }
}

# bit_len — total number of bits accumulated so far.
sub bit_len { scalar @{ $_[0]->{bits} } }

# to_bytes — pack accumulated bits into a byte array (MSB first within each byte).
#
# Bits are grouped into 8-bit chunks. If the total bit count is not a
# multiple of 8, the last byte is zero-padded on the right (least significant side).
sub to_bytes {
    my ($self) = @_;
    my @bytes;
    my @bits = @{ $self->{bits} };
    my $n = scalar @bits;
    for (my $i = 0; $i < $n; $i += 8) {
        my $byte = 0;
        for my $j (0 .. 7) {
            $byte = ($byte << 1) | ($bits[$i + $j] // 0);
        }
        push @bytes, $byte;
    }
    return \@bytes;
}

# to_bit_vec — return a reference to the raw bit array (0s and 1s).
sub to_bit_vec { [ @{ $_[0]->{bits} } ] }

package CodingAdventures::MicroQR;

# =============================================================================
# Data encoding helpers
# =============================================================================

# _mode_indicator_value — numeric value for the mode indicator field.
#
# The mode indicator bit width varies by symbol (0 bits for M1, 1 for M2,
# 2 for M3, 3 for M4). Within a given width, each mode has a distinct code.
#
#   M1 (0 bits): no indicator — M1 only supports numeric anyway.
#   M2 (1 bit):  numeric=0, alphanumeric/byte=1
#   M3 (2 bits): numeric=00, alphanumeric=01, byte=10
#   M4 (3 bits): numeric=000, alphanumeric=001, byte=010
sub _mode_indicator_value {
    my ($mode, $cfg) = @_;
    my $bits = $cfg->{mode_bits};
    return 0 if $bits == 0;   # M1: no indicator
    if ($bits == 1) {
        return ($mode eq MODE_NUMERIC) ? 0 : 1;
    }
    if ($bits == 2) {
        return 0b00 if $mode eq MODE_NUMERIC;
        return 0b01 if $mode eq MODE_ALPHANUMERIC;
        return 0b10;   # byte
    }
    # $bits == 3 (M4)
    return 0b000 if $mode eq MODE_NUMERIC;
    return 0b001 if $mode eq MODE_ALPHANUMERIC;
    return 0b010;  # byte
}

# _cc_bits — width of the character-count field for the given mode and config.
sub _cc_bits {
    my ($mode, $cfg) = @_;
    return $cfg->{cc_num}   if $mode eq MODE_NUMERIC;
    return $cfg->{cc_alpha} if $mode eq MODE_ALPHANUMERIC;
    return $cfg->{cc_byte};
}

# _encode_numeric — push numeric data bits into $w.
#
# Groups of 3 digits → 10 bits (value range 0-999, needs 10 bits).
# Pairs of 2 digits  →  7 bits (value range 0-99).
# Single digit       →  4 bits (value range 0-9).
sub _encode_numeric {
    my ($input, $w) = @_;
    my @chars = split //, $input;
    my $n = scalar @chars;
    my $i = 0;
    while ($i + 2 < $n) {
        my $v = substr($input, $i, 3) + 0;
        $w->write($v, 10);
        $i += 3;
    }
    if ($i + 1 < $n) {
        my $v = substr($input, $i, 2) + 0;
        $w->write($v, 7);
        $i += 2;
    }
    if ($i < $n) {
        $w->write(substr($input, $i, 1) + 0, 4);
    }
}

# _encode_alphanumeric — push alphanumeric data bits into $w.
#
# Character pairs encode as: (idx_a * 45 + idx_b) → 11 bits.
# Trailing single character encodes as: idx → 6 bits.
sub _encode_alphanumeric {
    my ($input, $w) = @_;
    my @chars = split //, $input;
    my $n = scalar @chars;
    my $i = 0;
    while ($i + 1 < $n) {
        my $ia = index($ALPHANUM_CHARS, $chars[$i]);
        my $ib = index($ALPHANUM_CHARS, $chars[$i + 1]);
        croak "InvalidCharacter: '$chars[$i]' not in QR alphanumeric set" if $ia < 0;
        croak "InvalidCharacter: '$chars[$i+1]' not in QR alphanumeric set" if $ib < 0;
        $w->write($ia * 45 + $ib, 11);
        $i += 2;
    }
    if ($i < $n) {
        my $idx = index($ALPHANUM_CHARS, $chars[$i]);
        croak "InvalidCharacter: '$chars[$i]' not in QR alphanumeric set" if $idx < 0;
        $w->write($idx, 6);
    }
}

# _encode_byte — push raw byte data bits into $w.
#
# Each byte is written as-is, 8 bits, MSB first.
# For pure ASCII input this is equivalent to the character code.
sub _encode_byte {
    my ($input, $w) = @_;
    for my $b (unpack 'C*', $input) {
        $w->write($b, 8);
    }
}

# =============================================================================
# Data codeword assembly
# =============================================================================

# _build_data_codewords — assemble the full data codeword byte sequence.
#
# For all symbols except M1:
#   [mode indicator (0/1/2/3 bits)] [char count] [data bits]
#   [terminator (≤N zero bits)] [byte-boundary pad] [0xEC/0x11 fill bytes]
#   → exactly cfg->{data_cw} bytes
#
# For M1 (m1_half_cw = true):
#   Total data capacity = 20 bits (2 full bytes + 4-bit nibble).
#   The RS encoder sees 3 bytes where byte[2] carries data in its upper nibble
#   and its lower nibble is always 0.
#
# The 0xEC/0x11 pad bytes are the "11101100" / "00010001" alternation from
# ISO 18004. They ensure that the padding doesn't accidentally create a
# valid-looking terminator pattern.
sub _build_data_codewords {
    my ($input, $cfg, $mode) = @_;

    # Total usable data bit capacity.
    # M1 has 2.5 codewords = 20 bits; the last byte uses only upper 4 bits.
    my $total_bits = $cfg->{m1_half_cw}
        ? ($cfg->{data_cw} * 8 - 4)   # 3×8 − 4 = 20 bits
        : ($cfg->{data_cw} * 8);

    my $w = CodingAdventures::MicroQR::BitWriter->new;

    # 1. Mode indicator (variable width: 0-3 bits)
    if ($cfg->{mode_bits} > 0) {
        $w->write(_mode_indicator_value($mode, $cfg), $cfg->{mode_bits});
    }

    # 2. Character count
    my $char_count;
    if ($mode eq MODE_BYTE) {
        $char_count = length(pack 'A*', $input);
    } else {
        $char_count = length($input);
    }
    $w->write($char_count, _cc_bits($mode, $cfg));

    # 3. Data payload
    if ($mode eq MODE_NUMERIC) {
        _encode_numeric($input, $w);
    } elsif ($mode eq MODE_ALPHANUMERIC) {
        _encode_alphanumeric($input, $w);
    } else {
        _encode_byte($input, $w);
    }

    # 4. Terminator: up to cfg->{terminator_bits} zero bits, truncated if near capacity.
    my $remaining = $total_bits - $w->bit_len;
    if ($remaining > 0) {
        my $term = $cfg->{terminator_bits} < $remaining
            ? $cfg->{terminator_bits}
            : $remaining;
        $w->write(0, $term);
    }

    # 5. M1 special case: pack 20 bits into 3 bytes (last byte = upper nibble only).
    if ($cfg->{m1_half_cw}) {
        my $bits = $w->to_bit_vec;
        # Extend to exactly 20 bits (zero-pad on the right if needed).
        push @$bits, (0) x (20 - scalar(@$bits)) if scalar(@$bits) < 20;
        my $b0 = ($bits->[0]  << 7) | ($bits->[1]  << 6) | ($bits->[2]  << 5) | ($bits->[3]  << 4)
               | ($bits->[4]  << 3) | ($bits->[5]  << 2) | ($bits->[6]  << 1) | $bits->[7];
        my $b1 = ($bits->[8]  << 7) | ($bits->[9]  << 6) | ($bits->[10] << 5) | ($bits->[11] << 4)
               | ($bits->[12] << 3) | ($bits->[13] << 2) | ($bits->[14] << 1) | $bits->[15];
        my $b2 = ($bits->[16] << 7) | ($bits->[17] << 6) | ($bits->[18] << 5) | ($bits->[19] << 4);
        return [$b0, $b1, $b2];
    }

    # 6. Pad to byte boundary.
    my $rem = $w->bit_len % 8;
    $w->write(0, 8 - $rem) if $rem != 0;

    # 7. Fill remaining codewords with alternating 0xEC / 0x11 pad bytes.
    my $bytes = $w->to_bytes;
    my $pad = 0xEC;
    while (scalar(@$bytes) < $cfg->{data_cw}) {
        push @$bytes, $pad;
        $pad = ($pad == 0xEC) ? 0x11 : 0xEC;
    }

    return $bytes;
}

# =============================================================================
# Reed-Solomon encoder (GF(256)/0x11D, b=0 convention)
# =============================================================================
#
# Micro QR uses a specific RS variant:
#   - Field: GF(2^8) with primitive polynomial 0x11D (x^8+x^4+x^3+x^2+1)
#   - b=0 convention: generator polynomial g(x) = ∏(x + α^i) for i=0..n-1
#     where α^0 = 1 (first root is 1, not α).
#
# Generator polynomial degree equals the number of ECC codewords.
# The 6 degrees needed are {2, 5, 6, 8, 10, 14}.
#
# We pre-compute (cache) each generator on first use.

my %GENERATORS;

# Generator polynomials pre-computed from the Rust reference implementation.
# Each array contains coefficients [leading=1, ..., constant term], degree = n.
my %GENERATOR_TABLE = (
    2  => [0x01, 0x03, 0x02],
    5  => [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68],
    6  => [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37],
    8  => [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3],
    10 => [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45],
    14 => [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac],
);

# _get_generator — return (cached) generator polynomial for degree $n.
sub _get_generator {
    my ($n) = @_;
    unless (exists $GENERATORS{$n}) {
        croak "micro-qr: no generator for ecc_count=$n"
            unless exists $GENERATOR_TABLE{$n};
        $GENERATORS{$n} = $GENERATOR_TABLE{$n};
    }
    return $GENERATORS{$n};
}

# _gf_mul — multiply two GF(256) elements.
#
# Delegates to CodingAdventures::GF256::multiply which uses log/antilog
# tables for O(1) multiplication.
sub _gf_mul {
    my ($a, $b) = @_;
    return CodingAdventures::GF256::multiply($a, $b);
}

# _rs_encode — compute $n ECC bytes for @$data using the LFSR shift-register method.
#
# This implements polynomial long division: R(x) = D(x)·x^n mod G(x).
#
# Algorithm (classic LFSR):
#   Initialize remainder register rem[0..n-1] = 0.
#   For each data byte b:
#     feedback = b XOR rem[0]
#     Shift register left: rem[i] = rem[i+1] for i=0..n-2; rem[n-1] = 0
#     For each position i = 0..n-1:
#       rem[i] ^= G[i+1] * feedback
#
# Result: rem[0..n-1] are the n ECC bytes.
sub _rs_encode {
    my ($data_ref, $gen_ref) = @_;
    my $n = scalar(@$gen_ref) - 1;   # degree of generator polynomial
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
# WorkGrid — mutable grid with reservation tracking
# =============================================================================
#
# During construction we maintain two parallel 2D arrays:
#   modules[r][c]  — boolean (1=dark, 0=light)
#   reserved[r][c] — boolean (1=function module; skip during data placement/masking)
#
# "Function modules" include: finder pattern, separator, timing strips, and
# format information positions. They are never overwritten by data or masking.

sub _make_work_grid {
    my ($size) = @_;
    my (@modules, @reserved);
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

# _set_mod — write a module value and optionally mark it as reserved.
sub _set_mod {
    my ($g, $r, $c, $dark, $reserve) = @_;
    $g->{modules}[$r][$c]  = $dark ? 1 : 0;
    $g->{reserved}[$r][$c] = 1 if $reserve;
}

# =============================================================================
# Structural module placement
# =============================================================================

# _place_finder — draw the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
#
# The finder pattern is the distinctive 3-ring square that lets any scanner
# locate and orient the symbol:
#
#   ■ ■ ■ ■ ■ ■ ■   (outer ring: all dark)
#   ■ □ □ □ □ □ ■
#   ■ □ ■ ■ ■ □ ■   (middle ring: inner-border dark, rest light)
#   ■ □ ■ ■ ■ □ ■
#   ■ □ ■ ■ ■ □ ■
#   ■ □ □ □ □ □ ■
#   ■ ■ ■ ■ ■ ■ ■
#
# The 1:1:3:1:1 ratio appears in every horizontal and vertical scan direction,
# making the pattern uniquely identifiable at any orientation or skew angle.
sub _place_finder {
    my ($g) = @_;
    for my $dr (0 .. 6) {
        for my $dc (0 .. 6) {
            my $on_border = ($dr == 0 || $dr == 6 || $dc == 0 || $dc == 6);
            my $in_core   = ($dr >= 2 && $dr <= 4 && $dc >= 2 && $dc <= 4);
            _set_mod($g, $dr, $dc, ($on_border || $in_core) ? 1 : 0, 1);
        }
    }
}

# _place_separator — draw the L-shaped light separator around the finder.
#
# Unlike full QR (which surrounds all three finders), Micro QR's single finder
# only needs separation on its bottom (row 7) and right (col 7) sides.
# The top and left are the symbol boundary itself.
#
#   Row 7, cols 0-7: light (bottom separator)
#   Col 7, rows 0-7: light (right separator)
sub _place_separator {
    my ($g) = @_;
    for my $i (0 .. 7) {
        _set_mod($g, 7, $i, 0, 1);   # bottom row
        _set_mod($g, $i, 7, 0, 1);   # right column
    }
}

# _place_timing — draw timing strips along row 0 and col 0.
#
# In Micro QR, timing is at row 0 and col 0 (the symbol edges), not row 6/col 6
# as in regular QR. This saves space and eliminates the timing column skip.
#
# Positions 0-6 are already covered by the finder pattern.
# Position 7 is the separator (light).
# Positions 8+ alternate dark/light starting with dark at even indices.
#
# The alternating pattern (dark, light, dark, light...) gives decoders a
# reference for module pitch calibration.
sub _place_timing {
    my ($g) = @_;
    my $sz = $g->{size};
    for my $c (8 .. $sz - 1) {
        _set_mod($g, 0, $c, ($c % 2 == 0) ? 1 : 0, 1);
    }
    for my $r (8 .. $sz - 1) {
        _set_mod($g, $r, 0, ($r % 2 == 0) ? 1 : 0, 1);
    }
}

# _reserve_format_info — mark the 15 format information module positions.
#
# Format info occupies a single L-shaped region (unlike regular QR which has two copies):
#
#   Row 8, cols 1-8: f14 (col 1, MSB) ... f7 (col 8)
#   Col 8, rows 1-7: f6 (row 7) ... f0 (row 1, LSB)
#
# These 15 modules are reserved now (light) and filled with the actual format
# word at the end of encoding, after the best mask is determined.
sub _reserve_format_info {
    my ($g) = @_;
    for my $c (1 .. 8) { $g->{reserved}[8][$c] = 1; }
    for my $r (1 .. 7) { $g->{reserved}[$r][8] = 1; }
}

# _write_format_info — write the 15-bit format word into the reserved positions.
#
# Bit ordering (verified against Rust reference and ISO 18004:2015 Annex E):
#
#   Row 8, col 1 → bit f14 (MSB)
#   Row 8, col 2 → bit f13
#   ...
#   Row 8, col 8 → bit f7
#   Col 8, row 7 → bit f6
#   Col 8, row 6 → bit f5
#   ...
#   Col 8, row 1 → bit f0 (LSB)
sub _write_format_info {
    my ($g, $fmt) = @_;
    # Row 8, cols 1-8: bits f14 down to f7 (MSB first, left to right)
    for my $i (0 .. 7) {
        $g->{modules}[8][1 + $i] = ($fmt >> (14 - $i)) & 1;
    }
    # Col 8, rows 7 down to 1: bits f6 down to f0
    for my $i (0 .. 6) {
        $g->{modules}[7 - $i][8] = ($fmt >> (6 - $i)) & 1;
    }
}

# _build_grid — initialize the symbol grid with all structural modules.
#
# Builds the "skeleton" grid before data placement. Steps:
#   1. Place 7×7 finder pattern (top-left corner).
#   2. Place L-shaped separator (bottom and right of finder).
#   3. Place timing strips (row 0 and col 0, positions 8+).
#   4. Reserve format information positions.
sub _build_grid {
    my ($cfg) = @_;
    my $g = _make_work_grid($cfg->{size});
    _place_finder($g);
    _place_separator($g);
    _place_timing($g);
    _reserve_format_info($g);
    return $g;
}

# =============================================================================
# Data placement — two-column zigzag scan
# =============================================================================

# _place_bits — fill non-reserved modules with codeword bits via zigzag scan.
#
# The scan visits the symbol in two-column strips from right to left,
# alternating direction (up/down) with each strip:
#
#   Strip at cols (sz-1, sz-2): upward   (first strip; row decreases)
#   Strip at cols (sz-3, sz-4): downward
#   ... and so on leftward.
#
# Note: unlike regular QR there is NO timing column skip at col 6.
# Micro QR's timing is at col 0, which is reserved and auto-skipped.
#
# Reserved modules are skipped; data bits fill the unreserved positions.
# Any codeword bits beyond the available space are discarded (should not happen).
sub _place_bits {
    my ($g, $bits_ref) = @_;
    my $sz      = $g->{size};
    my $bit_idx = 0;
    my $up      = 1;      # 1=upward (row decreasing), 0=downward
    my $col     = $sz - 1;

    while ($col >= 1) {
        for my $vi (0 .. $sz - 1) {
            my $row = $up ? ($sz - 1 - $vi) : $vi;
            for my $dc (0, 1) {
                my $c = $col - $dc;
                next if $g->{reserved}[$row][$c];
                $g->{modules}[$row][$c] = ($bit_idx < scalar(@$bits_ref))
                    ? $bits_ref->[$bit_idx++]
                    : 0;
            }
        }
        $up  = !$up;
        $col -= 2;
    }
}

# =============================================================================
# Masking
# =============================================================================
#
# Micro QR defines 4 mask patterns (regular QR has 8). A mask is applied by
# XOR-ing every non-reserved module with the condition for that mask.
# The goal is to break up large homogeneous regions that could confuse decoders.
#
# Mask conditions (row r, col c):
#   0: (r + c) mod 2 == 0
#   1: r mod 2 == 0
#   2: c mod 3 == 0
#   3: (r + c) mod 3 == 0

my @MASK_CONDS = (
    sub { ($_[0] + $_[1]) % 2 == 0 },   # mask 0
    sub { $_[0] % 2 == 0 },             # mask 1
    sub { $_[1] % 3 == 0 },             # mask 2
    sub { ($_[0] + $_[1]) % 3 == 0 },   # mask 3
);

# _apply_mask — return a new module AoA with the chosen mask applied.
#
# Only non-reserved modules are toggled. We return a fresh AoA (not in-place)
# so we can try all 4 masks cheaply on the same base grid.
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
# Penalty scoring — ISO 18004 Section 7.8.3 (same rules as regular QR)
# =============================================================================

# _compute_penalty — score a masked module grid; lower score is better.
#
# Four rules:
#
# Rule 1 — adjacent same-color runs of ≥5 modules:
#   For each horizontal or vertical run of length L ≥ 5: score += L − 2.
#   A run of 5 scores 3; a run of 6 scores 4; etc.
#
# Rule 2 — 2×2 same-color blocks:
#   For each 2×2 square where all 4 modules are the same color: score += 3.
#
# Rule 3 — finder-pattern-like sequences:
#   Scans for 11-module sequences matching [1,0,1,1,1,0,1,0,0,0,0] or its
#   reverse [0,0,0,0,1,0,1,1,1,0,1] in both rows and columns.
#   Each match: score += 40.
#
# Rule 4 — dark module ratio deviation from 50%:
#   Compute dark% = (dark_modules / total_modules) × 100.
#   Round down to nearest multiple of 5: prev5.
#   Score += min(|prev5 − 50|, |prev5+5 − 50|) / 5 × 10.
sub _compute_penalty {
    my ($modules_ref, $sz) = @_;
    my $penalty = 0;

    # ── Rule 1: same-color runs ≥ 5 ─────────────────────────────────────────
    for my $a (0 .. $sz - 1) {
        for my $horiz (1, 0) {
            my $run  = 1;
            my $prev = $horiz ? $modules_ref->[$a][0] : $modules_ref->[0][$a];
            for my $i (1 .. $sz - 1) {
                my $cur = $horiz ? $modules_ref->[$a][$i] : $modules_ref->[$i][$a];
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

    # ── Rule 2: 2×2 same-color blocks ───────────────────────────────────────
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
    if ($sz >= 11) {
        for my $a (0 .. $sz - 1) {
            for my $b (0 .. $sz - 11) {
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
    }

    # ── Rule 4: dark module ratio ─────────────────────────────────────────────
    my $dark = 0;
    for my $r (0 .. $sz - 1) {
        for my $c (0 .. $sz - 1) {
            $dark++ if $modules_ref->[$r][$c];
        }
    }
    my $total  = $sz * $sz;
    my $dark_pct = int($dark * 100 / $total);
    my $prev5    = int($dark_pct / 5) * 5;
    my $a_dist   = abs($prev5      - 50);
    my $b_dist   = abs($prev5 + 5  - 50);
    my $r4_raw   = $a_dist < $b_dist ? $a_dist : $b_dist;
    $penalty    += int($r4_raw / 5) * 10;

    return $penalty;
}

# =============================================================================
# Public API
# =============================================================================

# encode — encode a string to a Micro QR Code ModuleGrid hashref.
#
# Automatically selects the smallest (version, ECC) combination that fits the
# input. Pass $version and/or $ecc to restrict selection to a specific symbol.
#
# Arguments:
#   $input   — input string (ASCII or UTF-8)
#   $version — undef, or one of: M1(), M2(), M3(), M4()   (integer 1-4)
#   $ecc     — undef, or one of: DETECTION(), ECC_L(), ECC_M(), ECC_Q()
#
# Returns a hashref compatible with CodingAdventures::Barcode2D::ModuleGrid:
#   {
#     rows         => $size,      # e.g. 11 for M1
#     cols         => $size,
#     modules      => \@aoa,      # 2D array of 0/1 (0=light, 1=dark)
#     module_shape => 'square',
#   }
#
# Dies with a descriptive string on error:
#   "InputTooLong: ..."     — input exceeds maximum capacity
#   "ECCNotAvailable: ..."  — no config matches the requested version/ECC
#   "InvalidCharacter: ..."  — character not encodeable in selected mode
#
# Example:
#   use CodingAdventures::MicroQR qw(encode);
#   my $grid = encode("HELLO", undef, undef);  # auto-select: M2 13×13
#   my $m4   = encode("12345", M4(), ECC_Q()); # forced: M4 17×17
sub encode {
    my ($input, $version, $ecc) = @_;

    my $cfg  = _select_config($input, $version, $ecc);
    my $mode = _select_mode($input, $cfg);
    croak "UnsupportedMode: cannot encode '$input' in any mode for this symbol"
        unless defined $mode;

    # 1. Build data codewords (mode + count + data + terminator + padding).
    my $data_cw = _build_data_codewords($input, $cfg, $mode);

    # 2. Compute RS ECC (GF(256)/0x11D, b=0 convention).
    my $gen    = _get_generator($cfg->{ecc_cw});
    my $ecc_cw = _rs_encode($data_cw, $gen);

    # 3. Flatten codewords to a bit stream.
    #
    # For M1: data[2] carries data only in its upper nibble → contribute 4 bits.
    # All other codewords contribute all 8 bits.
    my @all_cw = (@$data_cw, @$ecc_cw);
    my @bits;
    for my $cw_idx (0 .. $#all_cw) {
        my $cw = $all_cw[$cw_idx];
        my $bits_in_cw = ($cfg->{m1_half_cw} && $cw_idx == $cfg->{data_cw} - 1) ? 4 : 8;
        for my $b (reverse 0 .. $bits_in_cw - 1) {
            push @bits, ($cw >> ($b + (8 - $bits_in_cw))) & 1;
        }
    }

    # 4. Build the structural grid (finder, separator, timing, reserved format).
    my $grid = _build_grid($cfg);

    # 5. Zigzag placement of data/ECC bits.
    _place_bits($grid, \@bits);

    # 6. Evaluate all 4 masks; choose the one with the lowest penalty score.
    my $best_mask    = 0;
    my $best_penalty = ~0;   # start at "infinity" (max int)
    my $sz = $cfg->{size};
    for my $m (0 .. 3) {
        my $masked = _apply_mask($grid->{modules}, $grid->{reserved}, $sz, $m);
        my $fmt    = $FORMAT_TABLE[ $cfg->{symbol_indicator} ][$m];
        # Write format info into a temporary grid for scoring.
        my $tmp = {
            size     => $sz,
            modules  => $masked,
            reserved => $grid->{reserved},
        };
        _write_format_info($tmp, $fmt);
        my $p = _compute_penalty($masked, $sz);
        if ($p < $best_penalty) {
            $best_penalty = $p;
            $best_mask    = $m;
        }
    }

    # 7. Apply best mask and write final format information.
    my $final_mods = _apply_mask($grid->{modules}, $grid->{reserved}, $sz, $best_mask);
    my $final_fmt  = $FORMAT_TABLE[ $cfg->{symbol_indicator} ][$best_mask];
    my $final_g = {
        size     => $sz,
        modules  => $final_mods,
        reserved => $grid->{reserved},
    };
    _write_format_info($final_g, $final_fmt);

    # 8. Return as a ModuleGrid hashref.
    return {
        rows         => $sz,
        cols         => $sz,
        modules      => $final_mods,
        module_shape => CodingAdventures::Barcode2D::SHAPE_SQUARE,
    };
}

# encode_at — encode with explicit version and ECC level (convenience wrapper).
#
# Equivalent to encode($input, $version, $ecc) but with positional $version/$ecc
# required rather than optional — throws immediately if the combination is invalid.
#
# Example:
#   use CodingAdventures::MicroQR qw(encode_at);
#   my $grid = encode_at("HELLO", M2(), ECC_L());
sub encode_at {
    my ($input, $version, $ecc) = @_;
    croak "encode_at: version is required" unless defined $version;
    croak "encode_at: ecc is required"     unless defined $ecc;
    return encode($input, $version, $ecc);
}

# layout_grid — convert a ModuleGrid to a PaintScene via barcode-2d layout.
#
# Uses a default quiet zone of 2 modules (Micro QR minimum; regular QR uses 4).
# Pass a CodingAdventures::Barcode2D::LayoutConfig hashref to override.
#
# Arguments:
#   $grid   — ModuleGrid hashref from encode() or encode_at()
#   $config — optional LayoutConfig hashref (see barcode-2d docs)
#
# Returns a PaintScene hashref.
sub layout_grid {
    my ($grid, $config) = @_;
    $config //= { quiet_zone_modules => 2 };
    return CodingAdventures::Barcode2D::layout($grid, $config);
}

1;
