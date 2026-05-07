package CodingAdventures::DataMatrix;

# =============================================================================
# CodingAdventures::DataMatrix — ISO/IEC 16022:2006 Data Matrix ECC200 encoder
# =============================================================================
#
# Data Matrix ECC200 was standardised in 2006 by RVSI Acuity CiMatrix.
# It is ubiquitous on objects that need permanent, damage-tolerant marking:
#
#   - Printed circuit boards: every PCB carries a Data Matrix.
#   - US FDA DSCSA mandate: unit-dose pharmaceutical packaging.
#   - Aerospace parts: rivets, shims, brackets — etched into metal.
#   - Surgical instruments: GS1 DataMatrix on implantables.
#
# ## What makes Data Matrix different from QR Code
#
#   1. No masking — the diagonal Utah placement distributes bits well enough.
#   2. L-shaped finder + timing clock (not three separate finder squares).
#   3. Utah diagonal codeword placement (not QR's two-column zigzag).
#   4. GF(256)/0x12D (not QR's 0x11D) with b=1 RS roots (α^1..α^n).
#
# ## Encoding pipeline (this module)
#
#   input bytes/string
#     → ASCII encoding       (digit pairs packed into single codewords)
#     → symbol size selection (smallest that fits)
#     → pad to capacity      (scrambled-pad fills unused slots)
#     → RS ECC per block     (GF(256)/0x12D, b=1 convention)
#     → interleave blocks    (data round-robin then ECC round-robin)
#     → grid initialization  (L-finder + timing border + alignment borders)
#     → Utah placement       (diagonal codeword placement, no masking!)
#     → ModuleGrid           (2D array of 0/1 values, 1 = dark)
#
# =============================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);

use CodingAdventures::Barcode2D ();

our $VERSION = '0.1.0';

use Exporter 'import';
our @EXPORT_OK = qw(encode_data_matrix encode);

# =============================================================================
# GF(256) arithmetic over the 0x12D field
# =============================================================================
#
# Data Matrix uses GF(256) with primitive polynomial:
#
#   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  = 0x12D = 301
#
# This is DIFFERENT from QR Code's 0x11D polynomial. Both are degree-8
# irreducible polynomials over GF(2), but the resulting fields are
# non-isomorphic — the primitive element α behaves differently in each.
#
# The primitive element is α = 2 (the polynomial "x"). We generate all
# 255 non-zero elements by repeatedly multiplying by α and reducing modulo
# p(x) when the degree-8 term appears.
#
# We pre-build two lookup tables:
#   @GF_EXP[$i] = α^i  for i in 0..254, with @GF_EXP[255] = @GF_EXP[0] = 1
#   @GF_LOG[$v] = k such that α^k = v  (index 0 is undefined/unused)

use constant GF256_POLY => 0x12d;

our @GF_EXP = (0) x 256;
our @GF_LOG = (0) x 256;

# Build tables at module load time.
{
    my $val = 1;
    for my $i ( 0 .. 254 ) {
        $GF_EXP[$i]   = $val;
        $GF_LOG[$val] = $i;
        $val <<= 1;           # multiply by α (left-shift)
        $val ^= GF256_POLY if $val & 0x100;  # reduce by 0x12D
        $val &= 0xff;
    }
    $GF_EXP[255] = $GF_EXP[0];  # wrap-around: α^255 = α^0 = 1
}

# _gf_mul — multiply two GF(256)/0x12D elements using the log/antilog trick.
#
# For nonzero a, b:
#   a × b = α^( (log_a + log_b) mod 255 )
#
# The mod-255 is handled implicitly because @GF_EXP is indexed in [0..254]
# and we use (sum % 255).
sub _gf_mul {
    my ( $a, $b ) = @_;
    return 0 if $a == 0 || $b == 0;
    return $GF_EXP[ ( $GF_LOG[$a] + $GF_LOG[$b] ) % 255 ];
}

# =============================================================================
# Symbol size tables (ISO/IEC 16022:2006, Table 7)
# =============================================================================
#
# Each symbol size entry records all the parameters the encoder needs:
#
#   symbolRows / symbolCols  — physical grid dimensions (incl. outer border)
#   regionRows / regionCols  — how many data sub-regions (rr × rc)
#   dataRegionHeight / dataRegionWidth — interior size of each sub-region
#   dataCW  — total data codeword capacity
#   eccCW   — total ECC codeword count
#   numBlocks    — number of independent RS blocks
#   eccPerBlock  — ECC codewords per block (constant across all blocks)
#
# For small symbols (≤ 26×26) there is exactly one data region (1×1), so
# the logical data matrix is the full interior (symbolRows-2) × (symbolCols-2).
# Larger symbols subdivide into a grid of regions separated by 2-module-wide
# alignment borders. The Utah algorithm works on the logical data matrix — all
# regions concatenated — and maps back to physical coordinates afterward.

my @SQUARE_SIZES = (
    # [ sR,   sC,  rR, rC,  drH,  drW,  dataCW, eccCW, numBlks, eccPerBlk ]
    [ 10,   10,   1,  1,   8,   8,    3,    5,  1,   5 ],
    [ 12,   12,   1,  1,  10,  10,    5,    7,  1,   7 ],
    [ 14,   14,   1,  1,  12,  12,    8,   10,  1,  10 ],
    [ 16,   16,   1,  1,  14,  14,   12,   12,  1,  12 ],
    [ 18,   18,   1,  1,  16,  16,   18,   14,  1,  14 ],
    [ 20,   20,   1,  1,  18,  18,   22,   18,  1,  18 ],
    [ 22,   22,   1,  1,  20,  20,   30,   20,  1,  20 ],
    [ 24,   24,   1,  1,  22,  22,   36,   24,  1,  24 ],
    [ 26,   26,   1,  1,  24,  24,   44,   28,  1,  28 ],
    [ 32,   32,   2,  2,  14,  14,   62,   36,  2,  18 ],
    [ 36,   36,   2,  2,  16,  16,   86,   42,  2,  21 ],
    [ 40,   40,   2,  2,  18,  18,  114,   48,  2,  24 ],
    [ 44,   44,   2,  2,  20,  20,  144,   56,  4,  14 ],
    [ 48,   48,   2,  2,  22,  22,  174,   68,  4,  17 ],
    [ 52,   52,   2,  2,  24,  24,  204,   84,  4,  21 ],
    [ 64,   64,   4,  4,  14,  14,  280,  112,  4,  28 ],
    [ 72,   72,   4,  4,  16,  16,  368,  144,  4,  36 ],
    [ 80,   80,   4,  4,  18,  18,  456,  192,  4,  48 ],
    [ 88,   88,   4,  4,  20,  20,  576,  224,  4,  56 ],
    [ 96,   96,   4,  4,  22,  22,  696,  272,  4,  68 ],
    [ 104, 104,   4,  4,  24,  24,  816,  336,  6,  56 ],
    [ 120, 120,   6,  6,  18,  18, 1050,  408,  6,  68 ],
    [ 132, 132,   6,  6,  20,  20, 1304,  496,  8,  62 ],
    [ 144, 144,   6,  6,  22,  22, 1558,  620, 10,  62 ],
);

my @RECT_SIZES = (
    # [ sR,  sC,  rR, rC,  drH,  drW,  dataCW, eccCW, numBlks, eccPerBlk ]
    [  8,  18,   1,  1,   6,  16,    5,    7,  1,   7 ],
    [  8,  32,   1,  2,   6,  14,   10,   11,  1,  11 ],
    [ 12,  26,   1,  1,  10,  24,   16,   14,  1,  14 ],
    [ 12,  36,   1,  2,  10,  16,   22,   18,  1,  18 ],
    [ 16,  36,   1,  2,  14,  16,   32,   24,  1,  24 ],
    [ 16,  48,   1,  2,  14,  22,   49,   28,  1,  28 ],
);

# _size_entry_struct — given a row from the size table, return a named hashref.
# Using a helper keeps the rest of the code readable.
sub _make_entry {
    my ($row) = @_;
    return {
        symbolRows       => $row->[0],
        symbolCols       => $row->[1],
        regionRows       => $row->[2],
        regionCols       => $row->[3],
        dataRegionHeight => $row->[4],
        dataRegionWidth  => $row->[5],
        dataCW           => $row->[6],
        eccCW            => $row->[7],
        numBlocks        => $row->[8],
        eccPerBlock      => $row->[9],
    };
}

# All square entries as hashrefs (built once at module load time).
my @SQUARE_ENTRIES = map { _make_entry($_) } @SQUARE_SIZES;

# All rectangular entries.
my @RECT_ENTRIES = map { _make_entry($_) } @RECT_SIZES;

# =============================================================================
# RS generator polynomial (GF(256)/0x12D, b=1 convention)
# =============================================================================
#
# The generator polynomial for n ECC codewords is:
#
#   g(x) = ∏_{i=1..n} (x + α^i)
#
# The b=1 convention means the roots are α^1, α^2, ..., α^n. This is exactly
# the same as the MA02 reed-solomon package's convention. Data Matrix uses
# b=1 throughout; QR Code uses b=0 (roots α^0..α^{n-1}) which is different.
#
# We cache generator polynomials keyed by ECC length so the same polynomial
# is not recomputed for every block.

my %GEN_CACHE;   # n_ecc → arrayref of coefficients

# _build_generator — compute the monic generator polynomial of degree n.
# Returns an arrayref of length n+1: [1, a1, a2, ..., a_n] (leading coeff = 1).
sub _build_generator {
    my ($n) = @_;
    my @g = (1);
    for my $i ( 1 .. $n ) {
        # Multiply g(x) by (x + α^i) = [1, α^i].
        my $ai    = $GF_EXP[$i];
        my @next  = (0) x ( scalar(@g) + 1 );
        for my $j ( 0 .. $#g ) {
            $next[$j]       ^= $g[$j];
            $next[ $j + 1 ] ^= _gf_mul( $g[$j], $ai );
        }
        @g = @next;
    }
    return \@g;
}

# _get_generator — return (and cache) the generator for n ECC bytes.
sub _get_generator {
    my ($n) = @_;
    unless ( exists $GEN_CACHE{$n} ) {
        $GEN_CACHE{$n} = _build_generator($n);
    }
    return $GEN_CACHE{$n};
}

# Pre-build all generators needed for the symbol size table at module load.
{
    my %seen;
    for my $e ( @SQUARE_ENTRIES, @RECT_ENTRIES ) {
        my $n = $e->{eccPerBlock};
        $seen{$n} = 1;
    }
    _get_generator($_) for keys %seen;
}

# =============================================================================
# Reed-Solomon encoding (LFSR / polynomial-division approach)
# =============================================================================
#
# Given:
#   D(x) = data polynomial (data bytes as coefficients, highest degree first)
#   G(x) = generator polynomial of degree n_ecc
#
# Compute:
#   R(x) = D(x) × x^n_ecc  mod  G(x)
#
# The n_ecc coefficients of R(x) are the ECC codewords.
#
# The LFSR implementation:
#   for each data byte d:
#     feedback = d XOR rem[0]
#     shift rem left: rem[i] ← rem[i+1]
#     rem[last] ← 0
#     for each generator coefficient:
#       rem[i] ^= gen[i+1] * feedback
#
# This avoids the full polynomial multiplication and is O(n*ecc) time.

sub _rs_encode_block {
    my ( $data_ref, $n_ecc ) = @_;
    my $gen  = _get_generator($n_ecc);
    my @rem  = (0) x $n_ecc;

    for my $byte ( @{$data_ref} ) {
        my $fb = $byte ^ $rem[0];
        # Shift the register left.
        for my $i ( 0 .. $n_ecc - 2 ) {
            $rem[$i] = $rem[ $i + 1 ];
        }
        $rem[ $n_ecc - 1 ] = 0;
        if ( $fb != 0 ) {
            for my $i ( 0 .. $n_ecc - 1 ) {
                $rem[$i] ^= _gf_mul( $gen->[ $i + 1 ], $fb );
            }
        }
    }
    return \@rem;
}

# =============================================================================
# ASCII data encoding
# =============================================================================
#
# Data Matrix ECC200 uses ASCII mode by default. The codeword vocabulary:
#
#   Single ASCII char (0–127):  codeword = ASCII_value + 1    (range 1–128)
#   Two consecutive ASCII digits: codeword = 130 + (d1×10+d2) (range 130–229)
#   Extended ASCII (128–255):   two codewords: 235, (char - 127)
#
# The digit-pair optimization is critical: two-digit sequences like "12" fit
# in a SINGLE codeword (codeword 142 = 130+12), halving codeword count for
# numeric strings. This matters for lot codes, serial numbers, and barcodes
# that are predominantly digit strings.
#
# Example:
#   "A"    → [66]      (65+1)
#   " "    → [33]      (32+1)
#   "12"   → [142]     (130+12, digit pair)
#   "1A"   → [50, 66]  (no pair — 'A' is not a digit)
#   "99"   → [229]     (130+99)

sub _encode_ascii {
    my ($bytes_ref) = @_;
    my @cw;
    my $i   = 0;
    my $len = scalar @$bytes_ref;

    while ( $i < $len ) {
        my $c = $bytes_ref->[$i];

        # Digit-pair check: both current and next byte are ASCII digits 0–9.
        if (   $c >= 0x30 && $c <= 0x39
            && $i + 1 < $len
            && $bytes_ref->[ $i + 1 ] >= 0x30
            && $bytes_ref->[ $i + 1 ] <= 0x39 )
        {
            my $d1 = $c                 - 0x30;
            my $d2 = $bytes_ref->[$i+1] - 0x30;
            push @cw, 130 + $d1 * 10 + $d2;
            $i += 2;
        }
        elsif ( $c <= 127 ) {
            push @cw, $c + 1;
            $i++;
        }
        else {
            # Extended ASCII: UPPER_SHIFT (235) + (char - 127).
            push @cw, 235;
            push @cw, $c - 127;
            $i++;
        }
    }

    return \@cw;
}

# =============================================================================
# Pad codewords (ISO/IEC 16022:2006 §5.2.3)
# =============================================================================
#
# After encoding, the codeword sequence must be padded to exactly dataCW
# bytes (the symbol's capacity). Padding rules:
#
#   1. First pad codeword is always 129.
#   2. Subsequent pads use a scrambled value:
#
#        scrambled = 129 + (149 × k mod 253) + 1
#        if scrambled > 254: scrambled -= 254
#
#      where k is the 1-indexed position of the pad byte within the FULL
#      codeword stream (including data codewords before the pad region).
#
# The scrambling prevents a run of "129 129 129..." from creating a
# degenerate Utah placement pattern. Each pad codeword is unique, so the
# resulting module pattern looks pseudo-random.
#
# Example for "A" (codeword [66]) in a 10×10 symbol (dataCW=3):
#   Position k=2: first pad → 129
#   Position k=3: scrambled = 129 + (149*3 mod 253) + 1
#                           = 129 + (447 mod 253) + 1
#                           = 129 + 194 + 1 = 324; 324 > 254 → 70
#   Final: [66, 129, 70]

sub _pad_codewords {
    my ( $cw_ref, $data_cw ) = @_;
    my @padded = @$cw_ref;
    my $first  = scalar @padded;          # index of first pad byte
    my $k      = $first + 1;             # k is 1-indexed

    while ( scalar @padded < $data_cw ) {
        if ( scalar @padded == $first ) {
            # First pad codeword is always exactly 129.
            push @padded, 129;
        }
        else {
            my $scrambled = 129 + ( ( 149 * $k ) % 253 ) + 1;
            $scrambled -= 254 if $scrambled > 254;
            push @padded, $scrambled;
        }
        $k++;
    }
    return \@padded;
}

# =============================================================================
# Symbol selection
# =============================================================================
#
# Select the smallest symbol whose data capacity fits the encoded codeword
# count. Iterate square symbols first (smallest to largest), then rectangular
# ones if requested. The "shape" option controls which pools are considered.

sub _select_symbol {
    my ( $cw_count, $shape ) = @_;
    $shape //= 'square';

    my @candidates;
    if ( $shape eq 'square' || $shape eq 'any' ) {
        push @candidates, @SQUARE_ENTRIES;
    }
    if ( $shape eq 'rectangular' || $shape eq 'any' ) {
        push @candidates, @RECT_ENTRIES;
    }

    # Sort by dataCW ascending, then by symbol area for ties.
    @candidates = sort {
           $a->{dataCW}  <=> $b->{dataCW}
        || $a->{symbolRows} * $a->{symbolCols}
               <=> $b->{symbolRows} * $b->{symbolCols}
    } @candidates;

    for my $e (@candidates) {
        return $e if $e->{dataCW} >= $cw_count;
    }

    croak "InputTooLong: encoded data requires $cw_count codewords, "
        . "exceeds maximum 1558 (144x144 symbol).";
}

# =============================================================================
# Block splitting and interleaving
# =============================================================================
#
# For multi-block symbols, the padded data codewords are split across
# numBlocks independent RS blocks. Each block gets its own ECC.
#
# Split rule (ISO interleaving convention):
#   baseLen   = floor(dataCW / numBlocks)
#   extraBlocks = dataCW mod numBlocks
#   The first extraBlocks blocks get baseLen+1 codewords; the rest get baseLen.
#
# After computing ECC, the streams are interleaved:
#   interleaved = data[0][0], data[1][0], ..., data[n-1][0],
#                 data[0][1], data[1][1], ...,
#                 ...
#                 ecc[0][0],  ecc[1][0],  ..., ecc[n-1][0],
#                 ...
#
# Interleaving distributes burst errors: a physical scratch destroying K
# contiguous codewords affects at most ceil(K/numBlocks) codewords per block,
# well within the block's correction capacity.

sub _compute_interleaved {
    my ( $data_ref, $entry ) = @_;
    my ( $data_cw, $num_blocks, $ecc_per_block ) =
        @{$entry}{qw(dataCW numBlocks eccPerBlock)};

    # Split data into numBlocks blocks.
    my $base_len    = int( $data_cw / $num_blocks );
    my $extra_blocks = $data_cw % $num_blocks;

    my @data_blocks;
    my $offset = 0;
    for my $b ( 0 .. $num_blocks - 1 ) {
        my $len = ( $b < $extra_blocks ) ? $base_len + 1 : $base_len;
        push @data_blocks, [ @{$data_ref}[ $offset .. $offset + $len - 1 ] ];
        $offset += $len;
    }

    # Compute ECC for each block.
    my @ecc_blocks = map { _rs_encode_block( $_, $ecc_per_block ) } @data_blocks;

    # Interleave data (round-robin across blocks).
    my @interleaved;
    my $max_data_len = 0;
    for my $b (@data_blocks) {
        $max_data_len = scalar @$b if scalar @$b > $max_data_len;
    }
    for my $pos ( 0 .. $max_data_len - 1 ) {
        for my $b ( 0 .. $num_blocks - 1 ) {
            if ( $pos < scalar @{ $data_blocks[$b] } ) {
                push @interleaved, $data_blocks[$b][$pos];
            }
        }
    }

    # Interleave ECC (round-robin across blocks).
    for my $pos ( 0 .. $ecc_per_block - 1 ) {
        for my $b ( 0 .. $num_blocks - 1 ) {
            push @interleaved, $ecc_blocks[$b][$pos];
        }
    }

    return \@interleaved;
}

# =============================================================================
# Grid initialization — finder + timing border + alignment borders
# =============================================================================
#
# The physical module grid starts with all-light modules. Then we layer on the
# structural elements in a specific order so that the finder pattern (which has
# the highest visual priority) overrides everything else:
#
#   Step 1: alignment borders (between data regions, multi-region only)
#   Step 2: top row (timing: alternating dark/light starting dark)
#   Step 3: right column (timing: alternating dark/light starting dark)
#   Step 4: left column (all dark — left leg of L-finder)
#   Step 5: bottom row (all dark — bottom leg of L-finder, HIGHEST priority)
#
# The L-shaped dark bar is the finder pattern. A scanner locates the dark-L
# corner at the lower-left and knows the symbol's orientation. The alternating
# top row and right column are timing patterns — they tell the scanner the
# module pitch for distortion correction.
#
# Alignment borders (for symbols with multiple data regions) follow the same
# pattern: a solid-dark bar adjacent to an alternating bar. They break the
# interior into independently locatable sub-regions.

sub _init_grid {
    my ($entry) = @_;
    my ( $sr, $sc, $rr, $rc, $drh, $drw ) =
        @{$entry}{qw(symbolRows symbolCols regionRows regionCols
                     dataRegionHeight dataRegionWidth)};

    # Start with all-light (0).
    my @grid;
    for my $r ( 0 .. $sr - 1 ) {
        push @grid, [ (0) x $sc ];
    }

    # ── Step 1: alignment borders (between adjacent data region pairs)
    #
    # Between region rows rr_i and rr_i+1, we need two rows:
    #   AB row 0 = all dark
    #   AB row 1 = alternating dark/light starting dark
    #
    # The physical row of the first AB row between region row rr_i and rr_i+1:
    #   abRow0 = 1 (outer border) + (rr_i+1)*drh + rr_i*2 (previous ABs)
    #          = 1 + (rr_i+1)*drh + 2*rr_i
    for my $ri ( 0 .. $rr - 2 ) {
        my $ab_row0 = 1 + ( $ri + 1 ) * $drh + $ri * 2;
        my $ab_row1 = $ab_row0 + 1;
        for my $c ( 0 .. $sc - 1 ) {
            $grid[$ab_row0][$c] = 1;
            $grid[$ab_row1][$c] = ( $c % 2 == 0 ) ? 1 : 0;
        }
    }

    for my $ci ( 0 .. $rc - 2 ) {
        my $ab_col0 = 1 + ( $ci + 1 ) * $drw + $ci * 2;
        my $ab_col1 = $ab_col0 + 1;
        for my $r ( 0 .. $sr - 1 ) {
            $grid[$r][$ab_col0] = 1;
            $grid[$r][$ab_col1] = ( $r % 2 == 0 ) ? 1 : 0;
        }
    }

    # ── Step 2: top row — timing pattern (alternating, starts dark at col 0)
    for my $c ( 0 .. $sc - 1 ) {
        $grid[0][$c] = ( $c % 2 == 0 ) ? 1 : 0;
    }

    # ── Step 3: right column — timing pattern (alternating, starts dark at row 0)
    for my $r ( 0 .. $sr - 1 ) {
        $grid[$r][ $sc - 1 ] = ( $r % 2 == 0 ) ? 1 : 0;
    }

    # ── Step 4: left column — all dark (vertical leg of the L-finder)
    # Written after the timing patterns so col-0 timing is overridden by dark.
    for my $r ( 0 .. $sr - 1 ) {
        $grid[$r][0] = 1;
    }

    # ── Step 5: bottom row — all dark (horizontal leg of the L-finder)
    # Written last so the L-finder bottom row overrides alignment borders,
    # right-column timing, and everything else. The L-finder has the
    # HIGHEST precedence.
    for my $c ( 0 .. $sc - 1 ) {
        $grid[ $sr - 1 ][$c] = 1;
    }

    return \@grid;
}

# =============================================================================
# Utah placement algorithm — boundary wrapping
# =============================================================================
#
# The Utah diagonal placement algorithm scans the logical data matrix and
# places 8-bit codewords using the "Utah" shape. Near the edges of the logical
# grid, the Utah shape's positions can fall outside the valid range [0..nRows-1]
# × [0..nCols-1]. These out-of-bounds positions are "wrapped" back into the
# grid using specific rules from ISO/IEC 16022:2006, Annex F.
#
# Wrap rules (applied AFTER computing raw positions from the Utah offsets):
#
#   If row < 0 AND col == 0:          row = 1; col = 3    (top-left corner singularity)
#   If row < 0 AND col == nCols:      row = 0; col -= 2   (off-top-right)
#   If row < 0 (general):             row += nRows; col -= 4
#   If col < 0 (general):             col += nCols; row -= 4

sub _apply_wrap {
    my ( $row, $col, $n_rows, $n_cols ) = @_;

    # Special case: top-left corner singularity.
    if ( $row < 0 && $col == 0 ) {
        return ( 1, 3 );
    }

    # Special case: off the top-right.
    if ( $row < 0 && $col == $n_cols ) {
        return ( 0, $col - 2 );
    }

    # General top wrap.
    if ( $row < 0 ) {
        return ( $row + $n_rows, $col - 4 );
    }

    # General left wrap.
    if ( $col < 0 ) {
        return ( $row - 4, $col + $n_cols );
    }

    return ( $row, $col );
}

# =============================================================================
# Utah placement — placing one codeword with the standard shape
# =============================================================================
#
# The "Utah" shape is named for the US state because its 8-module outline
# resembles the silhouette: a rectangle with the top-left corner removed.
#
# Standard Utah placement at reference (row, col):
#
#   col:  c-2  c-1   c
#   r-2:   .   [b1] [b2]
#   r-1: [b3]  [b4] [b5]
#   r  : [b6]  [b7] [b8]
#
# Bit 8 (MSB) lands at (row, col); bit 1 (LSB) at (row-2, col-1).
#
# After boundary wrapping, if a position is still in bounds and not already
# set (via the "used" tracking array), the bit value is written.

sub _place_utah {
    my ( $cw, $row, $col, $n_rows, $n_cols, $grid_ref, $used_ref ) = @_;

    # The 8 raw (row, col, bit_index) triples.
    # bit_index 7 = MSB, 0 = LSB.
    my @placements = (
        [ $row,     $col,     7 ],   # bit 8
        [ $row,     $col - 1, 6 ],   # bit 7
        [ $row,     $col - 2, 5 ],   # bit 6
        [ $row - 1, $col,     4 ],   # bit 5
        [ $row - 1, $col - 1, 3 ],   # bit 4
        [ $row - 1, $col - 2, 2 ],   # bit 3
        [ $row - 2, $col,     1 ],   # bit 2
        [ $row - 2, $col - 1, 0 ],   # bit 1
    );

    for my $p (@placements) {
        my ( $r, $c, $bit ) = @$p;
        ( $r, $c ) = _apply_wrap( $r, $c, $n_rows, $n_cols );
        next if $r < 0 || $r >= $n_rows || $c < 0 || $c >= $n_cols;
        next if $used_ref->[$r][$c];
        $grid_ref->[$r][$c] = ( ( $cw >> $bit ) & 1 );
        $used_ref->[$r][$c] = 1;
    }
}

# =============================================================================
# Utah corner patterns
# =============================================================================
#
# Four special-case patterns handle codewords that fall near the grid
# boundary in ways the standard wrap rules cannot resolve. Each pattern
# uses absolute positions within the logical grid rather than offsets from
# a reference.
#
# Corner pattern 1: triggered at top-left boundary (row==nRows, col==0)
# Corner pattern 2: triggered at top-right boundary (row==nRows-2, col==0, nCols%4!=0)
# Corner pattern 3: triggered at bottom-left boundary (row==nRows-2, col==0, nCols%8==4)
# Corner pattern 4: right-edge wrap for odd-dimension matrices

sub _place_corner1 {
    my ( $cw, $n_rows, $n_cols, $grid_ref, $used_ref ) = @_;
    my @positions = (
        [ 0,           $n_cols - 2, 7 ],
        [ 0,           $n_cols - 1, 6 ],
        [ 1,           0,           5 ],
        [ 2,           0,           4 ],
        [ $n_rows - 2, 0,           3 ],
        [ $n_rows - 1, 0,           2 ],
        [ $n_rows - 1, 1,           1 ],
        [ $n_rows - 1, 2,           0 ],
    );
    for my $p (@positions) {
        my ( $r, $c, $bit ) = @$p;
        next if $used_ref->[$r][$c];
        $grid_ref->[$r][$c] = ( ( $cw >> $bit ) & 1 );
        $used_ref->[$r][$c] = 1;
    }
}

sub _place_corner2 {
    my ( $cw, $n_rows, $n_cols, $grid_ref, $used_ref ) = @_;
    my @positions = (
        [ 0,           $n_cols - 2, 7 ],
        [ 0,           $n_cols - 1, 6 ],
        [ 1,           $n_cols - 1, 5 ],
        [ 2,           $n_cols - 1, 4 ],
        [ $n_rows - 1, 0,           3 ],
        [ $n_rows - 1, 1,           2 ],
        [ $n_rows - 1, 2,           1 ],
        [ $n_rows - 1, 3,           0 ],
    );
    for my $p (@positions) {
        my ( $r, $c, $bit ) = @$p;
        next if $used_ref->[$r][$c];
        $grid_ref->[$r][$c] = ( ( $cw >> $bit ) & 1 );
        $used_ref->[$r][$c] = 1;
    }
}

sub _place_corner3 {
    my ( $cw, $n_rows, $n_cols, $grid_ref, $used_ref ) = @_;
    my @positions = (
        [ 0,           $n_cols - 1, 7 ],
        [ 1,           0,           6 ],
        [ 2,           0,           5 ],
        [ $n_rows - 2, 0,           4 ],
        [ $n_rows - 1, 0,           3 ],
        [ $n_rows - 1, 1,           2 ],
        [ $n_rows - 1, 2,           1 ],
        [ $n_rows - 1, 3,           0 ],
    );
    for my $p (@positions) {
        my ( $r, $c, $bit ) = @$p;
        next if $used_ref->[$r][$c];
        $grid_ref->[$r][$c] = ( ( $cw >> $bit ) & 1 );
        $used_ref->[$r][$c] = 1;
    }
}

sub _place_corner4 {
    my ( $cw, $n_rows, $n_cols, $grid_ref, $used_ref ) = @_;
    my @positions = (
        [ $n_rows - 3, $n_cols - 1, 7 ],
        [ $n_rows - 2, $n_cols - 1, 6 ],
        [ $n_rows - 1, $n_cols - 3, 5 ],
        [ $n_rows - 1, $n_cols - 2, 4 ],
        [ $n_rows - 1, $n_cols - 1, 3 ],
        [ 0,           0,            2 ],
        [ 1,           0,            1 ],
        [ 2,           0,            0 ],
    );
    for my $p (@positions) {
        my ( $r, $c, $bit ) = @$p;
        next if $used_ref->[$r][$c];
        $grid_ref->[$r][$c] = ( ( $cw >> $bit ) & 1 );
        $used_ref->[$r][$c] = 1;
    }
}

# =============================================================================
# Utah placement algorithm — full diagonal traversal
# =============================================================================
#
# This is the most distinctive part of Data Matrix encoding. Named "Utah"
# because the 8-module shape used per codeword resembles the US state of Utah.
#
# Algorithm overview:
#   1. Start at reference (row=4, col=0).
#   2. Check for corner special cases at this reference.
#   3. Scan upward-right diagonal (row-=2, col+=2) placing codewords.
#   4. Step to next diagonal start: row+=1, col+=3.
#   5. Scan downward-left diagonal (row+=2, col-=2) placing codewords.
#   6. Step to next diagonal start: row+=3, col+=1.
#   7. Repeat until row>=nRows AND col>=nCols, or all codewords placed.
#   8. Fill any residual unset positions with the (r+c)%2==1 pattern.
#
# The residual fill handles symbols where the data area is not evenly
# divisible by the 8-module codeword placement scheme.
#
# Returns an nRows×nCols grid (logical data matrix).

sub _utah_placement {
    my ( $codewords_ref, $n_rows, $n_cols ) = @_;

    my @grid;
    my @used;
    for my $r ( 0 .. $n_rows - 1 ) {
        push @grid, [ (0) x $n_cols ];
        push @used, [ (0) x $n_cols ];
    }

    my $cw_idx = 0;
    my $n_cw   = scalar @$codewords_ref;
    my $row    = 4;
    my $col    = 0;

    # Helper: place next codeword using a named corner function.
    my $place_corner = sub {
        my ($fn_ref) = @_;
        return unless $cw_idx < $n_cw;
        $fn_ref->( $codewords_ref->[$cw_idx], $n_rows, $n_cols, \@grid, \@used );
        $cw_idx++;
    };

    while (1) {
        # ── Corner special cases — exact (row, col) triggers.

        # Corner 1: fires when row==nRows, col==0, and either dimension is
        # divisible by 4. This handles the "bottom fell off the left edge" wrap.
        if ( $row == $n_rows && $col == 0
            && ( $n_rows % 4 == 0 || $n_cols % 4 == 0 ) )
        {
            $place_corner->( \&_place_corner1 );
        }

        # Corner 2: fires when row==nRows-2, col==0, and nCols is NOT divisible by 4.
        if ( $row == $n_rows - 2 && $col == 0 && $n_cols % 4 != 0 ) {
            $place_corner->( \&_place_corner2 );
        }

        # Corner 3: fires when row==nRows-2, col==0, and nCols mod 8 == 4.
        if ( $row == $n_rows - 2 && $col == 0 && $n_cols % 8 == 4 ) {
            $place_corner->( \&_place_corner3 );
        }

        # Corner 4: fires when row==nRows+4, col==2, and nCols is divisible by 8.
        if ( $row == $n_rows + 4 && $col == 2 && $n_cols % 8 == 0 ) {
            $place_corner->( \&_place_corner4 );
        }

        # ── Standard upward-right diagonal: row-=2, col+=2 each step.
        do {
            if (   $row >= 0 && $row < $n_rows
                && $col >= 0 && $col < $n_cols
                && !$used[$row][$col] )
            {
                if ( $cw_idx < $n_cw ) {
                    _place_utah(
                        $codewords_ref->[$cw_idx],
                        $row, $col, $n_rows, $n_cols, \@grid, \@used
                    );
                    $cw_idx++;
                }
            }
            $row -= 2;
            $col += 2;
        } while ( $row >= 0 && $col < $n_cols );

        # Step to next diagonal start.
        $row += 1;
        $col += 3;

        # ── Standard downward-left diagonal: row+=2, col-=2 each step.
        do {
            if (   $row >= 0 && $row < $n_rows
                && $col >= 0 && $col < $n_cols
                && !$used[$row][$col] )
            {
                if ( $cw_idx < $n_cw ) {
                    _place_utah(
                        $codewords_ref->[$cw_idx],
                        $row, $col, $n_rows, $n_cols, \@grid, \@used
                    );
                    $cw_idx++;
                }
            }
            $row += 2;
            $col -= 2;
        } while ( $row < $n_rows && $col >= 0 );

        # Step to next diagonal start.
        $row += 3;
        $col += 1;

        # Termination: reference has fully passed the logical grid.
        last if $row >= $n_rows && $col >= $n_cols;
        last if $cw_idx >= $n_cw;
    }

    # ── Residual fill: any modules not set by the diagonal walk are filled
    # with the (r+c) mod 2 == 1 pattern (dark at odd sum). This matches
    # ISO/IEC 16022 §10's "right and bottom fill" rule.
    for my $r ( 0 .. $n_rows - 1 ) {
        for my $c ( 0 .. $n_cols - 1 ) {
            if ( !$used[$r][$c] ) {
                $grid[$r][$c] = ( ( $r + $c ) % 2 == 1 ) ? 1 : 0;
            }
        }
    }

    return \@grid;
}

# =============================================================================
# Logical → Physical coordinate mapping
# =============================================================================
#
# The Utah algorithm works on the logical data matrix — a flat grid of size
# (regionRows × dataRegionHeight) × (regionCols × dataRegionWidth). The
# physical symbol grid includes the outer border and alignment borders.
#
# Mapping formula for a symbol with rr×rc regions each of size rh×rw:
#
#   physRow = floor(r / rh) * (rh + 2) + (r mod rh) + 1
#   physCol = floor(c / rw) * (rw + 2) + (c mod rw) + 1
#
# The "+2" accounts for each 2-module alignment border between regions.
# The "+1" accounts for the 1-module outer border (L-finder / timing).
#
# For single-region symbols (rr=rc=1): physRow = r+1, physCol = c+1.

sub _logical_to_physical {
    my ( $r, $c, $entry ) = @_;
    my ( $rh, $rw ) = @{$entry}{qw(dataRegionHeight dataRegionWidth)};
    my $phys_row = int( $r / $rh ) * ( $rh + 2 ) + ( $r % $rh ) + 1;
    my $phys_col = int( $c / $rw ) * ( $rw + 2 ) + ( $c % $rw ) + 1;
    return ( $phys_row, $phys_col );
}

# =============================================================================
# Public API
# =============================================================================

# encode_data_matrix — main entry point. Alias: encode.
#
# Arguments:
#   $data    — input scalar (string or bytes). Wide characters are treated
#              as their raw byte values (callers should pre-encode to UTF-8).
#   $options — optional hashref:
#                { shape => 'square' | 'rectangular' | 'any' }
#              'square' is the default and the most common choice.
#
# Returns a ModuleGrid hashref compatible with CodingAdventures::Barcode2D:
#   {
#     rows         => $symbol_rows,
#     cols         => $symbol_cols,
#     modules      => \@aoa,   # 2D arrayref of 0/1 (1 = dark module)
#     module_shape => 'square',
#   }
#
# Croaks with "InputTooLong: ..." if the data exceeds the 144×144 symbol.
sub encode_data_matrix {
    my ( $data, $options ) = @_;
    my $shape = ( $options && defined $options->{shape} )
        ? $options->{shape}
        : 'square';

    # Normalize input to a byte array.
    # unpack 'C*' returns one integer per byte (the raw octet value).
    my @bytes = unpack( 'C*', defined $data ? $data : '' );

    # ── Step 1: ASCII encode the input bytes.
    my $codewords = _encode_ascii( \@bytes );

    # ── Step 2: Select the smallest symbol that fits.
    my $entry = _select_symbol( scalar @$codewords, $shape );

    # ── Step 3: Pad to the symbol's data capacity.
    my $padded = _pad_codewords( $codewords, $entry->{dataCW} );

    # ── Steps 4–6: Split into RS blocks, compute ECC, interleave.
    my $interleaved = _compute_interleaved( $padded, $entry );

    # ── Step 7: Initialize the physical module grid.
    my $phys_grid = _init_grid($entry);

    # ── Step 8: Run Utah placement on the logical data matrix.
    my $n_rows = $entry->{regionRows} * $entry->{dataRegionHeight};
    my $n_cols = $entry->{regionCols} * $entry->{dataRegionWidth};
    my $logical_grid = _utah_placement( $interleaved, $n_rows, $n_cols );

    # ── Step 9: Map logical coordinates to physical coordinates.
    for my $r ( 0 .. $n_rows - 1 ) {
        for my $c ( 0 .. $n_cols - 1 ) {
            my ( $pr, $pc ) = _logical_to_physical( $r, $c, $entry );
            $phys_grid->[$pr][$pc] = $logical_grid->[$r][$c];
        }
    }

    # ── Step 10: Return ModuleGrid. No masking step — Data Matrix never masks.
    # Each row is shallow-copied so callers mutating the result don't share
    # state with internal arrays.
    my @final_modules = map { [ @$_ ] } @$phys_grid;

    return {
        rows         => $entry->{symbolRows},
        cols         => $entry->{symbolCols},
        modules      => \@final_modules,
        module_shape => CodingAdventures::Barcode2D::SHAPE_SQUARE,
    };
}

# "encode" is an alias so callers can use either name.
*encode = \&encode_data_matrix;

1;
