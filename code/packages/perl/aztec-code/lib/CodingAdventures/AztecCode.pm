package CodingAdventures::AztecCode;

# =============================================================================
# CodingAdventures::AztecCode — ISO/IEC 24778:2008 Aztec Code encoder
# =============================================================================
#
# Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
# published as a patent-free format. Unlike QR Code (which uses three square
# finder patterns at three corners), Aztec Code places a single **bullseye
# finder pattern at the center** of the symbol. The scanner finds the center
# first, then reads outward in a spiral — no large quiet zone is needed.
#
# ## Where Aztec Code is used today
#
#   - IATA boarding passes — the barcode on every airline boarding pass
#   - Eurostar and Amtrak rail tickets — printed and on-screen tickets
#   - PostNL, Deutsche Post, La Poste — European postal routing
#   - US military ID cards
#
# ## Symbol variants
#
#   Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
#   Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
#
# ## Encoding pipeline (v0.1.0 — byte-mode only)
#
#   input string / bytes
#     -> Binary-Shift codewords from Upper mode
#     -> symbol size selection (smallest compact then full at 23% ECC)
#     -> pad to exact codeword count
#     -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
#     -> bit stuffing (insert complement after 4 consecutive identical bits)
#     -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
#     -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)
#
# ## v0.1.0 simplifications
#
#   1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
#      Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
#   2. 8-bit codewords -> GF(256) RS (same polynomial as Data Matrix: 0x12D).
#      GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
#   3. Default ECC = 23%.
#   4. Auto-select compact vs full (force-compact option is v0.2.0).
#
# ## Reference implementation
#
# Mirrors the TypeScript reference at
# code/packages/typescript/aztec-code/src/index.ts.
#
# =============================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(ceil);

use CodingAdventures::Barcode2D ();

our $VERSION = '0.1.0';

use Exporter 'import';
our @EXPORT_OK = qw(encode);

# =============================================================================
# GF(16) arithmetic — for mode message Reed-Solomon
# =============================================================================
#
# GF(16) is the finite field with 16 elements, built from the primitive
# polynomial p(x) = x^4 + x + 1 (binary 10011 = 0x13). Every non-zero element
# can be written as a power of the primitive element alpha. alpha is the root
# of p(x), so alpha^4 = alpha + 1.
#
# The log table maps a field element (1..15) to its discrete log (0..14).
# The antilog table maps a log value (0..15) to its element.
#
#   alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
#   alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
#   alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
#   alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)

# LOG16->[e] = i  iff  alpha^i = e.  Index 0 is "undefined" (sentinel -1).
my @LOG16 = (
    -1,   # log(0) undefined
     0,   # log(1)
     1,   # log(2)
     4,   # log(3)
     2,   # log(4)
     8,   # log(5)
     5,   # log(6)
    10,   # log(7)
     3,   # log(8)
    14,   # log(9)
     9,   # log(10)
     7,   # log(11)
     6,   # log(12)
    13,   # log(13)
    11,   # log(14)
    12,   # log(15)
);

# ALOG16->[i] = alpha^i.  Index 15 wraps back to 1.
my @ALOG16 = ( 1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1 );

# _gf16_mul — multiply two GF(16) elements via log/antilog.
#
# Returns 0 if either operand is 0; otherwise computes
# alpha^( (log a + log b) mod 15 ).
sub _gf16_mul {
    my ($a, $b) = @_;
    return 0 if $a == 0 || $b == 0;
    return $ALOG16[ ( $LOG16[$a] + $LOG16[$b] ) % 15 ];
}

# _build_gf16_generator — construct the monic GF(16) RS generator polynomial
# whose roots are alpha^1, alpha^2, ..., alpha^n.
#
# Returns coefficients [g_0, g_1, ..., g_n] such that
#   g(x) = (x + alpha^1)(x + alpha^2) ... (x + alpha^n).
# g_n is always 1 (monic).
sub _build_gf16_generator {
    my ($n) = @_;
    my @g = (1);
    for my $i ( 1 .. $n ) {
        my $ai = $ALOG16[ $i % 15 ];
        my @next = (0) x ( scalar(@g) + 1 );
        for my $j ( 0 .. $#g ) {
            $next[ $j + 1 ] ^= $g[$j];
            $next[$j]       ^= _gf16_mul( $ai, $g[$j] );
        }
        @g = @next;
    }
    return \@g;
}

# _gf16_rs_encode — compute n GF(16) RS check nibbles for the given data
# nibbles using the LFSR polynomial-division trick.
#
# This is the GF(16) analogue of QR Code's GF(256) Reed-Solomon: the same
# algorithm, just with 4-bit operands.
sub _gf16_rs_encode {
    my ( $data_ref, $n ) = @_;
    my $g = _build_gf16_generator($n);
    my @rem = (0) x $n;
    for my $byte ( @{$data_ref} ) {
        my $fb = $byte ^ $rem[0];
        for my $i ( 0 .. $n - 2 ) {
            $rem[$i] = $rem[ $i + 1 ] ^ _gf16_mul( $g->[ $i + 1 ], $fb );
        }
        $rem[ $n - 1 ] = _gf16_mul( $g->[$n], $fb );
    }
    return \@rem;
}

# =============================================================================
# GF(256)/0x12D arithmetic — for 8-bit data codewords
# =============================================================================
#
# Aztec Code uses GF(256) with primitive polynomial
#   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D.
#
# This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
# QR Code (0x11D). The repo's CodingAdventures::GF256 uses 0x11D, so we
# build a private 0x12D table here.
#
# Generator convention: b=1, roots alpha^1..alpha^n (MA02 / Data Matrix style).

use constant GF256_POLY => 0x12d;

# EXP_12D[i] = alpha^i in GF(256)/0x12D. Doubled (length 512) so a sum of two
# logs in [0..254] never needs a modulo to land back in range.
# LOG_12D[e] = discrete log of e in GF(256)/0x12D.
my @EXP_12D;
my @LOG_12D;

# Build the tables once at module load. The primitive element is alpha = 2.
# We start at x = 1 (= alpha^0), then repeatedly multiply by alpha (which is
# a left shift in GF(256)). Whenever the shift overflows past bit 8 the value
# is reduced modulo the primitive polynomial.
{
    my $x = 1;
    for my $i ( 0 .. 254 ) {
        $EXP_12D[$i]         = $x;
        $EXP_12D[ $i + 255 ] = $x;
        $LOG_12D[$x]         = $i;
        $x <<= 1;
        $x ^= GF256_POLY if $x & 0x100;
        $x &= 0xff;
    }
    $EXP_12D[255] = 1;
}

# _gf256_mul — multiply two GF(256)/0x12D elements via the doubled EXP table.
sub _gf256_mul {
    my ( $a, $b ) = @_;
    return 0 if $a == 0 || $b == 0;
    return $EXP_12D[ $LOG_12D[$a] + $LOG_12D[$b] ];
}

# _build_gf256_generator — monic GF(256)/0x12D RS generator polynomial whose
# roots are alpha^1..alpha^n.
#
# Returned in big-endian order: index 0 is the leading (highest-degree)
# coefficient, index n is the constant term.
sub _build_gf256_generator {
    my ($n) = @_;
    my @g = (1);
    for my $i ( 1 .. $n ) {
        my $ai = $EXP_12D[$i];
        my @next = (0) x ( scalar(@g) + 1 );
        for my $j ( 0 .. $#g ) {
            $next[$j]       ^= $g[$j];
            $next[ $j + 1 ] ^= _gf256_mul( $g[$j], $ai );
        }
        @g = @next;
    }
    return \@g;
}

# _gf256_rs_encode — compute $n_check GF(256)/0x12D RS check bytes for the
# given data bytes using the LFSR polynomial-division algorithm.
#
# This mirrors the Data Matrix ECC200 generator convention exactly.
sub _gf256_rs_encode {
    my ( $data_ref, $n_check ) = @_;
    my $g = _build_gf256_generator($n_check);
    my $n = scalar(@$g) - 1;
    my @rem = (0) x $n;
    for my $b ( @{$data_ref} ) {
        my $fb = $b ^ $rem[0];
        for my $i ( 0 .. $n - 2 ) {
            $rem[$i] = $rem[ $i + 1 ] ^ _gf256_mul( $g->[ $i + 1 ], $fb );
        }
        $rem[ $n - 1 ] = _gf256_mul( $g->[$n], $fb );
    }
    return \@rem;
}

# =============================================================================
# Aztec Code capacity tables (ISO/IEC 24778:2008 Table 1)
# =============================================================================
#
# Each entry maps (compact?, layer count) to:
#   total_bits  — total data+ECC bit positions in the symbol.
#   max_bytes_8 — number of 8-bit codeword slots available.
#
# Only the 8-bit codeword path is exercised in v0.1.0. The 6-bit / 8-bit /
# 10-bit / 12-bit boundaries used by GF(16)/GF(32) codewords are deferred.

# Index 0 is unused; index k = layer count (1..4 compact, 1..32 full).
my @COMPACT_CAPACITY = (
    { total_bits => 0,   max_bytes_8 => 0  },   # unused
    { total_bits =>  72, max_bytes_8 =>  9 },   # 1 layer, 15x15
    { total_bits => 200, max_bytes_8 => 25 },   # 2 layers, 19x19
    { total_bits => 392, max_bytes_8 => 49 },   # 3 layers, 23x23
    { total_bits => 648, max_bytes_8 => 81 },   # 4 layers, 27x27
);

my @FULL_CAPACITY = (
    { total_bits => 0,     max_bytes_8 => 0    },   # unused
    { total_bits =>    88, max_bytes_8 =>   11 },   #  1 layer
    { total_bits =>   216, max_bytes_8 =>   27 },
    { total_bits =>   360, max_bytes_8 =>   45 },
    { total_bits =>   520, max_bytes_8 =>   65 },
    { total_bits =>   696, max_bytes_8 =>   87 },
    { total_bits =>   888, max_bytes_8 =>  111 },
    { total_bits =>  1096, max_bytes_8 =>  137 },
    { total_bits =>  1320, max_bytes_8 =>  165 },
    { total_bits =>  1560, max_bytes_8 =>  195 },
    { total_bits =>  1816, max_bytes_8 =>  227 },
    { total_bits =>  2088, max_bytes_8 =>  261 },
    { total_bits =>  2376, max_bytes_8 =>  297 },
    { total_bits =>  2680, max_bytes_8 =>  335 },
    { total_bits =>  3000, max_bytes_8 =>  375 },
    { total_bits =>  3336, max_bytes_8 =>  417 },
    { total_bits =>  3688, max_bytes_8 =>  461 },
    { total_bits =>  4056, max_bytes_8 =>  507 },
    { total_bits =>  4440, max_bytes_8 =>  555 },
    { total_bits =>  4840, max_bytes_8 =>  605 },
    { total_bits =>  5256, max_bytes_8 =>  657 },
    { total_bits =>  5688, max_bytes_8 =>  711 },
    { total_bits =>  6136, max_bytes_8 =>  767 },
    { total_bits =>  6600, max_bytes_8 =>  825 },
    { total_bits =>  7080, max_bytes_8 =>  885 },
    { total_bits =>  7576, max_bytes_8 =>  947 },
    { total_bits =>  8088, max_bytes_8 => 1011 },
    { total_bits =>  8616, max_bytes_8 => 1077 },
    { total_bits =>  9160, max_bytes_8 => 1145 },
    { total_bits =>  9720, max_bytes_8 => 1215 },
    { total_bits => 10296, max_bytes_8 => 1287 },
    { total_bits => 10888, max_bytes_8 => 1361 },
    { total_bits => 11496, max_bytes_8 => 1437 },
);

# =============================================================================
# Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
# =============================================================================
#
# All input is wrapped in a single Binary-Shift block from Upper mode:
#   1. Emit 5 bits = 0b11111  (Binary-Shift escape in Upper mode)
#   2. If len <= 31: 5 bits for length
#      If len > 31:  5 bits = 0b00000, then 11 bits for length
#   3. Each byte as 8 bits, MSB first.

# _encode_bytes_as_bits — produce the Binary-Shift bit stream for the input
# bytes. Returns an arrayref of 0/1 values, MSB first.
sub _encode_bytes_as_bits {
    my ($input_bytes_ref) = @_;
    my @bits;

    # Inner closure: append the $count least-significant bits of $value MSB-first.
    my $write_bits = sub {
        my ( $value, $count ) = @_;
        for my $i ( reverse 0 .. $count - 1 ) {
            push @bits, ( $value >> $i ) & 1;
        }
    };

    my $len = scalar @$input_bytes_ref;

    # Step 1: Binary-Shift escape (5 bits all 1s)
    $write_bits->( 31, 5 );

    # Step 2: length field (short or long form)
    if ( $len <= 31 ) {
        $write_bits->( $len, 5 );
    }
    else {
        $write_bits->( 0,    5 );
        $write_bits->( $len, 11 );
    }

    # Step 3: each byte 8 bits MSB-first.
    for my $byte (@$input_bytes_ref) {
        $write_bits->( $byte, 8 );
    }

    return \@bits;
}

# =============================================================================
# Symbol size selection
# =============================================================================
#
# Try compact 1..4 layers first (smaller, more efficient finder), then full
# 1..32 layers. For each candidate compute:
#
#   ecc_cw  = ceil( min_ecc_pct/100 * total_bytes )
#   data_cw = total_bytes - ecc_cw
#
# Apply a conservative 20% bit-stuffing inflation factor to the input bit
# count before checking fit. Pick the first candidate whose data_cw is
# large enough.

# _select_symbol — returns a hashref describing the chosen symbol:
#   { compact => 1|0, layers => N, data_cw_count => D, ecc_cw_count => E,
#     total_bits => T }
#
# Croaks with "InputTooLong: ..." if no symbol fits.
sub _select_symbol {
    my ( $data_bit_count, $min_ecc_pct ) = @_;
    my $stuffed_bit_count = ceil( $data_bit_count * 1.2 );

    for my $layers ( 1 .. 4 ) {
        my $cap = $COMPACT_CAPACITY[$layers];
        next unless $cap;
        my $total_bytes = $cap->{max_bytes_8};
        my $ecc_cw      = ceil( ( $min_ecc_pct / 100 ) * $total_bytes );
        my $data_cw     = $total_bytes - $ecc_cw;
        next if $data_cw <= 0;
        if ( ceil( $stuffed_bit_count / 8 ) <= $data_cw ) {
            return {
                compact       => 1,
                layers        => $layers,
                data_cw_count => $data_cw,
                ecc_cw_count  => $ecc_cw,
                total_bits    => $cap->{total_bits},
            };
        }
    }

    for my $layers ( 1 .. 32 ) {
        my $cap = $FULL_CAPACITY[$layers];
        next unless $cap;
        my $total_bytes = $cap->{max_bytes_8};
        my $ecc_cw      = ceil( ( $min_ecc_pct / 100 ) * $total_bytes );
        my $data_cw     = $total_bytes - $ecc_cw;
        next if $data_cw <= 0;
        if ( ceil( $stuffed_bit_count / 8 ) <= $data_cw ) {
            return {
                compact       => 0,
                layers        => $layers,
                data_cw_count => $data_cw,
                ecc_cw_count  => $ecc_cw,
                total_bits    => $cap->{total_bits},
            };
        }
    }

    croak
        "InputTooLong: input is too long to fit in any Aztec Code symbol "
      . "($data_bit_count bits needed)";
}

# =============================================================================
# Padding
# =============================================================================
#
# Pad the bit stream with zeros up to the next byte boundary, then up to the
# target byte count. The stream is then sliced to exactly target_bytes * 8 bits.

# _pad_to_bytes — return a NEW arrayref padded/truncated to target_bytes * 8.
sub _pad_to_bytes {
    my ( $bits_ref, $target_bytes ) = @_;
    my @out = @$bits_ref;
    push @out, 0 while ( @out % 8 ) != 0;
    push @out, 0 while @out < $target_bytes * 8;
    @out = @out[ 0 .. $target_bytes * 8 - 1 ];
    return \@out;
}

# =============================================================================
# Bit stuffing
# =============================================================================
#
# After every 4 consecutive identical bits (all 0 or all 1), insert one
# complement bit. Applies only to the data+ECC bit stream so that scanners
# never see runs of >=5 identical bits in the encoded body.
#
# Example:
#   Input:  1 1 1 1 0 0 0 0
#   After 4 ones: insert 0  -> [1,1,1,1,0]
#   After 4 zeros: insert 1 -> [1,1,1,1,0, 0,0,0,1,0]

# _stuff_bits — apply Aztec bit stuffing.
sub _stuff_bits {
    my ($bits_ref) = @_;
    my @stuffed;
    my $run_val = -1;
    my $run_len = 0;

    for my $bit (@$bits_ref) {
        if ( $bit == $run_val ) {
            $run_len++;
        }
        else {
            $run_val = $bit;
            $run_len = 1;
        }

        push @stuffed, $bit;

        if ( $run_len == 4 ) {
            my $stuff_bit = 1 - $bit;
            push @stuffed, $stuff_bit;
            $run_val = $stuff_bit;
            $run_len = 1;
        }
    }

    return \@stuffed;
}

# =============================================================================
# Mode message encoding
# =============================================================================
#
# The mode message encodes the layer count and data-codeword count, protected
# by GF(16) RS:
#
#   Compact (28 bits = 7 nibbles):
#     m = ((layers-1) << 6) | (dataCwCount-1)
#     2 data nibbles + 5 ECC nibbles
#
#   Full (40 bits = 10 nibbles):
#     m = ((layers-1) << 11) | (dataCwCount-1)
#     4 data nibbles + 6 ECC nibbles

# _encode_mode_message — return arrayref of 28 or 40 bits.
sub _encode_mode_message {
    my ( $compact, $layers, $data_cw_count ) = @_;

    my @data_nibbles;
    my $num_ecc;
    if ($compact) {
        my $m = ( ( $layers - 1 ) << 6 ) | ( $data_cw_count - 1 );
        @data_nibbles = ( $m & 0xf, ( $m >> 4 ) & 0xf );
        $num_ecc      = 5;
    }
    else {
        my $m = ( ( $layers - 1 ) << 11 ) | ( $data_cw_count - 1 );
        @data_nibbles = (
              $m         & 0xf,
            ( $m >> 4 )  & 0xf,
            ( $m >> 8 )  & 0xf,
            ( $m >> 12 ) & 0xf,
        );
        $num_ecc = 6;
    }

    my $ecc_nibbles  = _gf16_rs_encode( \@data_nibbles, $num_ecc );
    my @all_nibbles = ( @data_nibbles, @$ecc_nibbles );

    my @bits;
    for my $nibble (@all_nibbles) {
        for my $i ( reverse 0 .. 3 ) {
            push @bits, ( $nibble >> $i ) & 1;
        }
    }
    return \@bits;
}

# =============================================================================
# Grid construction helpers
# =============================================================================

# _symbol_size — total side length in modules.
#   compact: 11 + 4*layers   (15..27)
#   full:    15 + 4*layers   (19..143)
sub _symbol_size {
    my ( $compact, $layers ) = @_;
    return $compact ? 11 + 4 * $layers : 15 + 4 * $layers;
}

# _bullseye_radius — Chebyshev radius of the bullseye finder pattern.
#   compact: 5  (11x11 finder)
#   full:    7  (15x15 finder)
sub _bullseye_radius {
    my ($compact) = @_;
    return $compact ? 5 : 7;
}

# _draw_bullseye — paint the central bullseye finder pattern.
#
# Color at Chebyshev distance d from the center (cx, cy):
#   d <= 1:                DARK   (the solid 3x3 inner core)
#   d > 1, d odd:          DARK
#   d > 1, d even:         LIGHT
#
# Both the modules and reserved arrays are mutated in place.
sub _draw_bullseye {
    my ( $modules, $reserved, $cx, $cy, $compact ) = @_;
    my $br = _bullseye_radius($compact);
    for my $row ( $cy - $br .. $cy + $br ) {
        for my $col ( $cx - $br .. $cx + $br ) {
            my $dx = abs( $col - $cx );
            my $dy = abs( $row - $cy );
            my $d  = $dx > $dy ? $dx : $dy;
            my $dark = ( $d <= 1 ) ? 1 : ( $d % 2 == 1 ? 1 : 0 );
            $modules->[$row][$col]  = $dark;
            $reserved->[$row][$col] = 1;
        }
    }
}

# _draw_reference_grid — paint the reference grid lines used by full Aztec
# symbols only.
#
# Grid lines lie on every row and column whose offset from the center is a
# multiple of 16. At intersections both lines meet (always dark); on a single
# line a module is dark iff its offset along the OTHER axis is even.
sub _draw_reference_grid {
    my ( $modules, $reserved, $cx, $cy, $size ) = @_;
    for my $row ( 0 .. $size - 1 ) {
        for my $col ( 0 .. $size - 1 ) {
            my $on_h = ( ( $cy - $row ) % 16 == 0 );
            my $on_v = ( ( $cx - $col ) % 16 == 0 );
            next unless $on_h || $on_v;

            my $dark;
            if ( $on_h && $on_v ) {
                $dark = 1;
            }
            elsif ($on_h) {
                $dark = ( ( $cx - $col ) % 2 == 0 ) ? 1 : 0;
            }
            else {
                $dark = ( ( $cy - $row ) % 2 == 0 ) ? 1 : 0;
            }

            $modules->[$row][$col]  = $dark;
            $reserved->[$row][$col] = 1;
        }
    }
}

# _draw_orientation_and_mode_message — populate the perimeter ring just
# outside the bullseye:
#
#   - The 4 corners of the ring are orientation marks (always DARK).
#   - The remaining non-corner positions, walked clockwise from "TL+1",
#     carry the mode-message bits one by one.
#
# Returns an arrayref of [col, row] positions left over after the mode bits;
# these positions still need to be filled with data bits during placement.
sub _draw_orientation_and_mode_message {
    my ( $modules, $reserved, $cx, $cy, $compact, $mode_bits_ref ) = @_;
    my $r = _bullseye_radius($compact) + 1;

    my @non_corner;

    # Top edge, left to right (skip both corners).
    for my $col ( $cx - $r + 1 .. $cx + $r - 1 ) {
        push @non_corner, [ $col, $cy - $r ];
    }
    # Right edge, top to bottom.
    for my $row ( $cy - $r + 1 .. $cy + $r - 1 ) {
        push @non_corner, [ $cx + $r, $row ];
    }
    # Bottom edge, right to left.
    for ( my $col = $cx + $r - 1; $col >= $cx - $r + 1; $col-- ) {
        push @non_corner, [ $col, $cy + $r ];
    }
    # Left edge, bottom to top.
    for ( my $row = $cy + $r - 1; $row >= $cy - $r + 1; $row-- ) {
        push @non_corner, [ $cx - $r, $row ];
    }

    # 4 corners of the orientation ring are always DARK.
    my @corners = (
        [ $cx - $r, $cy - $r ],
        [ $cx + $r, $cy - $r ],
        [ $cx + $r, $cy + $r ],
        [ $cx - $r, $cy + $r ],
    );
    for my $cr (@corners) {
        my ( $c, $r2 ) = @$cr;
        $modules->[$r2][$c]  = 1;
        $reserved->[$r2][$c] = 1;
    }

    # Walk clockwise placing mode-message bits. If the message is shorter
    # than the perimeter, leftover positions are returned for the data step.
    my $n_bits = scalar @$mode_bits_ref;
    my $n_pos  = scalar @non_corner;
    my $n      = $n_bits < $n_pos ? $n_bits : $n_pos;
    for my $i ( 0 .. $n - 1 ) {
        my ( $col, $row ) = @{ $non_corner[$i] };
        $modules->[$row][$col]  = $mode_bits_ref->[$i] == 1 ? 1 : 0;
        $reserved->[$row][$col] = 1;
    }

    if ( $n_bits >= $n_pos ) {
        return [];
    }
    my @rest = @non_corner[ $n_bits .. $n_pos - 1 ];
    return \@rest;
}

# _place_data_bits — fill the symbol with data bits in clockwise spiral order.
#
# Data bits are placed in two-module-wide bands, one band per data layer,
# starting at the inner radius adjacent to the orientation ring and spiralling
# outward. Each band traverses Top, Right, Bottom, Left edges in sequence,
# touching outer (dO) then inner (dI) modules at each step.
#
# Reserved positions (bullseye, reference grid, orientation ring, mode message)
# are skipped silently.
#
# The mode-ring leftover positions returned by _draw_orientation_and_mode_message
# are filled FIRST so the spiral picks up exactly where the mode message ended.
sub _place_data_bits {
    my ( $modules, $reserved, $bits_ref, $cx, $cy, $compact, $layers,
        $mode_ring_remaining ) = @_;
    my $size      = scalar @$modules;
    my $bit_index = 0;

    # Inner closure: place the next bit at (col, row) if in-bounds and not
    # already reserved by a structural element.
    my $place_bit = sub {
        my ( $col, $row ) = @_;
        return if $row < 0 || $row >= $size || $col < 0 || $col >= $size;
        return if $reserved->[$row][$col];
        my $bit = $bits_ref->[$bit_index] // 0;
        $modules->[$row][$col] = ( $bit == 1 ) ? 1 : 0;
        $bit_index++;
    };

    # 1. Mode-ring leftover positions first.
    for my $cr (@$mode_ring_remaining) {
        my ( $col, $row ) = @$cr;
        my $bit = $bits_ref->[$bit_index] // 0;
        $modules->[$row][$col] = ( $bit == 1 ) ? 1 : 0;
        $bit_index++;
    }

    # 2. Spiral through data layers.
    my $br      = _bullseye_radius($compact);
    my $d_start = $br + 2;    # mode-msg ring sits at br+1, first data at br+2

    for my $L ( 0 .. $layers - 1 ) {
        my $dI = $d_start + 2 * $L;    # inner radius
        my $dO = $dI + 1;              # outer radius

        # Top edge: left to right.
        for my $col ( $cx - $dI + 1 .. $cx + $dI ) {
            $place_bit->( $col, $cy - $dO );
            $place_bit->( $col, $cy - $dI );
        }
        # Right edge: top to bottom.
        for my $row ( $cy - $dI + 1 .. $cy + $dI ) {
            $place_bit->( $cx + $dO, $row );
            $place_bit->( $cx + $dI, $row );
        }
        # Bottom edge: right to left.
        for ( my $col = $cx + $dI; $col >= $cx - $dI + 1; $col-- ) {
            $place_bit->( $col, $cy + $dO );
            $place_bit->( $col, $cy + $dI );
        }
        # Left edge: bottom to top.
        for ( my $row = $cy + $dI; $row >= $cy - $dI + 1; $row-- ) {
            $place_bit->( $cx - $dO, $row );
            $place_bit->( $cx - $dI, $row );
        }
    }
}

# =============================================================================
# Public API
# =============================================================================

# encode — encode data as an Aztec Code symbol.
#
# Arguments:
#   $data    — input scalar (string or octets). Wide chars are NOT encoded;
#              callers should pass already-utf8-encoded bytes (use Encode).
#   $options — optional hashref:
#                { min_ecc_percent => 10..90 }   default 23
#
# Returns a ModuleGrid hashref compatible with CodingAdventures::Barcode2D:
#   {
#     rows         => $size,
#     cols         => $size,
#     modules      => \@aoa,        # 2D AoA of 0/1 (1 = dark)
#     module_shape => 'square',
#   }
#
# Croaks with "InputTooLong: ..." if the data exceeds maximum capacity.
sub encode {
    my ( $data, $options ) = @_;
    my $min_ecc_pct = ( $options && defined $options->{min_ecc_percent} )
        ? $options->{min_ecc_percent}
        : 23;

    # Treat the input as a sequence of bytes: unpack 'C*' returns one integer
    # per byte. For Perl strings holding bytes (length == bytecount) this is
    # exactly what we want; for utf8-flagged strings the caller should
    # already have Encode::encode_utf8'd them.
    my @bytes = unpack( 'C*', defined $data ? $data : '' );

    # Step 1: encode input bits (Binary-Shift escape + length + bytes).
    my $data_bits = _encode_bytes_as_bits( \@bytes );

    # Step 2: pick the smallest symbol that fits.
    my $spec = _select_symbol( scalar @$data_bits, $min_ecc_pct );
    my ( $compact, $layers, $data_cw_count, $ecc_cw_count ) =
      ( $spec->{compact}, $spec->{layers}, $spec->{data_cw_count},
        $spec->{ecc_cw_count} );

    # Step 3: pad to data_cw_count whole bytes.
    my $padded_bits = _pad_to_bytes( $data_bits, $data_cw_count );

    my @data_bytes;
    for my $i ( 0 .. $data_cw_count - 1 ) {
        my $byte = 0;
        for my $b ( 0 .. 7 ) {
            $byte = ( $byte << 1 ) | ( $padded_bits->[ $i * 8 + $b ] // 0 );
        }
        # All-zero codeword avoidance: if the LAST padded codeword would be
        # 0x00, replace it with 0xFF. Aztec scanners treat 0x00 as a sentinel.
        if ( $byte == 0 && $i == $data_cw_count - 1 ) {
            $byte = 0xff;
        }
        push @data_bytes, $byte;
    }

    # Step 4: GF(256)/0x12D RS ECC.
    my $ecc_bytes = _gf256_rs_encode( \@data_bytes, $ecc_cw_count );

    # Step 5: assemble combined byte stream and apply bit stuffing.
    my @all_bytes = ( @data_bytes, @$ecc_bytes );
    my @raw_bits;
    for my $byte (@all_bytes) {
        for my $i ( reverse 0 .. 7 ) {
            push @raw_bits, ( $byte >> $i ) & 1;
        }
    }
    my $stuffed_bits = _stuff_bits( \@raw_bits );

    # Step 6: GF(16) mode message.
    my $mode_msg = _encode_mode_message( $compact, $layers, $data_cw_count );

    # Step 7: initialize empty grid.
    my $size = _symbol_size( $compact, $layers );
    my $cx   = int( $size / 2 );
    my $cy   = int( $size / 2 );

    my @modules;
    my @reserved;
    for ( 0 .. $size - 1 ) {
        push @modules,  [ (0) x $size ];
        push @reserved, [ (0) x $size ];
    }

    # Reference grid first (full only), then bullseye overwrites.
    if ( !$compact ) {
        _draw_reference_grid( \@modules, \@reserved, $cx, $cy, $size );
    }
    _draw_bullseye( \@modules, \@reserved, $cx, $cy, $compact );

    my $mode_ring_remaining = _draw_orientation_and_mode_message(
        \@modules, \@reserved, $cx, $cy, $compact, $mode_msg
    );

    # Step 8: place data spiral.
    _place_data_bits(
        \@modules, \@reserved, $stuffed_bits,
        $cx, $cy, $compact, $layers,
        $mode_ring_remaining
    );

    # Return a fresh ModuleGrid hashref. Each row is shallow-copied so callers
    # mutating the result won't accidentally share state with internal arrays.
    my @final_modules = map { [ @$_ ] } @modules;

    return {
        rows         => $size,
        cols         => $size,
        modules      => \@final_modules,
        module_shape => CodingAdventures::Barcode2D::SHAPE_SQUARE,
    };
}

1;
