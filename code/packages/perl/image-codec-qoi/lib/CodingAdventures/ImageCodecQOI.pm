package CodingAdventures::ImageCodecQOI;

# ============================================================================
# CodingAdventures::ImageCodecQOI — QOI (Quite OK Image) Encode / Decode
# ============================================================================
#
# # What Is QOI?
#
# QOI (Quite OK Image Format) was designed by Dominic Szablewski in 2021.
# The goal: a lossless image codec simpler than PNG yet nearly as fast.
# The entire spec is ~300 lines; the reference decoder is ~200 lines of C.
#
# QOI achieves compression by exploiting common patterns in natural images:
#   * Many consecutive pixels are the same colour (run-length encoding).
#   * Many pixels were seen recently (64-entry hash table / "seen pixels").
#   * Many pixels differ only slightly from the previous one (delta coding).
#
# # File Structure
#
#   Bytes       Content
#   --------    -------
#   0–3         Magic: 'q', 'o', 'i', 'f'  (ASCII)
#   4–7         Width  (uint32 big-endian)
#   8–11        Height (uint32 big-endian)
#   12          Channels: 3 = RGB, 4 = RGBA
#   13          Colorspace: 0 = sRGB with linear alpha, 1 = all linear
#   14..N-8     Chunk stream (see below)
#   N-7..N      End marker: 8 bytes = 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01
#
# # The Six Chunk Types
#
# Every chunk starts with a tag in the top bits of the first byte:
#
#   Tag       Bits      Format        Description
#   -------   ------    ------        -----------
#   QOI_OP_RGB    11111110 (0xFE)    1+3 bytes  set R,G,B; keep A
#   QOI_OP_RGBA   11111111 (0xFF)    1+4 bytes  set R,G,B,A
#   QOI_OP_INDEX  00xxxxxx           1 byte     use previously-seen pixel at index x
#   QOI_OP_DIFF   01xxxxxx           1 byte     small delta (dr, dg, db each -2..1)
#   QOI_OP_LUMA   10xxxxxx yyyyyyyy  2 bytes    medium delta; dg -32..31, dr/db relative
#   QOI_OP_RUN    11xxxxxx           1 byte     run of (x+1) identical pixels (1..62)
#
# # The Seen-Pixels Hash Table
#
#   index = (R*3 + G*5 + B*7 + A*11) % 64
#
# When we see a pixel, we store it at this index.  If the next pixel hashes to
# the same index and matches, we emit QOI_OP_INDEX.
#
# # QOI_OP_DIFF Encoding
#
# Differences dr, dg, db each fit in a 2-bit signed value (-2..1).
# They are stored with a bias of +2 (so the range 0..3 fits in 2 bits):
#
#   stored = delta + 2    (0..3)
#   delta  = stored - 2   (-2..1)
#
#   Byte layout:
#     bits 7-6: 01  (tag)
#     bits 5-4: dr + 2
#     bits 3-2: dg + 2
#     bits 1-0: db + 2
#
# # QOI_OP_LUMA Encoding
#
# The green channel delta dg has a wider range (-32..31) than red/blue.
# Red and blue are stored relative to dg: dr_rel = dr - dg, db_rel = db - dg.
# This works because in natural images, chrominance channels tend to change
# together (correlation between R-G and B-G differences is high).
#
#   dg       in -32..31  (bias +32 → stored 0..63)
#   dr_rel   in -8..7    (bias +8  → stored 0..15)
#   db_rel   in -8..7    (bias +8  → stored 0..15)
#
#   Byte 1: 10xxxxxx  where xxxxxx = dg + 32
#   Byte 2: yyyyzzzz  where yyyy = dr_rel + 8, zzzz = db_rel + 8
#
# # Delta Arithmetic and Wrap-Around
#
# Colour channels are uint8 (0..255).  Deltas wrap around modulo 256:
#
#   delta = (new - prev + 256) % 256   (always in 0..255)
#
# Then interpret as a signed byte by mapping values > 127 to negative:
#
#   if delta > 127: delta -= 256   (so 255 → -1, 254 → -2, etc.)
#
# Helper function: `_signed_delta($new, $prev)` returns the signed difference
# in the range -128..127 using this wrap-around arithmetic.
#
# # Run-Length Encoding
#
# QOI_OP_RUN stores runs of 1..62 pixels (not 63 or 64, to avoid collision
# with 0xFE and 0xFF which are reserved for OP_RGB and OP_RGBA).
# The run length is stored as (count - 1) with bias, so the 6-bit value:
#
#   bits 5-0 = run_length - 1   (0..61)
#   byte = 0xC0 | (run_length - 1)
#
# ============================================================================

use strict;
use warnings;
use 5.026;
use Exporter 'import';

use CodingAdventures::PixelContainer;

our $VERSION   = '0.01';
our @EXPORT_OK = qw(encode_qoi decode_qoi mime_type);

# QOI chunk op-code tags
use constant QOI_OP_RGB   => 0xFE;   # 11111110
use constant QOI_OP_RGBA  => 0xFF;   # 11111111
use constant QOI_OP_INDEX => 0x00;   # 00xxxxxx  (tag bits = 00)
use constant QOI_OP_DIFF  => 0x40;   # 01xxxxxx  (tag bits = 01)
use constant QOI_OP_LUMA  => 0x80;   # 10xxxxxx  (tag bits = 10)
use constant QOI_OP_RUN   => 0xC0;   # 11xxxxxx  (tag bits = 11)

# QOI end-of-stream marker: 7 zero bytes followed by a 0x01 byte
use constant QOI_END_MARKER => "\x00\x00\x00\x00\x00\x00\x00\x01";

# ============================================================================
# mime_type() -> 'image/qoi'
# ============================================================================
sub mime_type { 'image/qoi' }

# ============================================================================
# _hash_pixel($r, $g, $b, $a) -> index (0..63)
# ============================================================================
#
# Compute the 64-entry seen-pixels table index for a pixel.
#
# Hash function from the QOI spec:
#   index = (R*3 + G*5 + B*7 + A*11) % 64
#
# The primes 3, 5, 7, 11 are chosen so that different channels affect
# different bit positions in the result, reducing collisions.
#
# @param $r,$g,$b,$a  Channel values 0..255
# @return             Index 0..63
# ============================================================================
sub _hash_pixel {
    my ($r, $g, $b, $a) = @_;
    return ($r * 3 + $g * 5 + $b * 7 + $a * 11) % 64;
}

# ============================================================================
# _signed_delta($new, $prev) -> signed integer -128..127
# ============================================================================
#
# Compute the signed difference between two uint8 channel values, with
# correct modular wrap-around (the QOI spec requires this).
#
# Examples:
#   _signed_delta(5, 3)     →  2
#   _signed_delta(3, 5)     → -2
#   _signed_delta(0, 255)   →  1   (wraps: 0 - 255 + 256 = 1)
#   _signed_delta(255, 0)   → -1   (wraps: 255 - 0 = 255 → -1 in signed)
#
# @param $new   New channel value  (0..255)
# @param $prev  Previous channel value (0..255)
# @return       Signed delta -128..127
# ============================================================================
sub _signed_delta {
    my ($new, $prev) = @_;
    my $d = ($new - $prev + 256) % 256;   # always 0..255
    $d -= 256 if $d > 127;                 # interpret as signed byte
    return $d;
}

# ============================================================================
# encode_qoi($container) -> $bytes
# ============================================================================
#
# Encode a PixelContainer as a QOI binary string.
#
# Algorithm:
#   1. Write 14-byte header.
#   2. Initialise: prev_pixel = (0,0,0,255), run = 0, seen[64] = all (0,0,0,0).
#   3. For each pixel (in raster order):
#      a. If same as prev: increment run; flush run chunk when run = 62.
#      b. Else: flush any pending run, then try (in priority order):
#         - QOI_OP_INDEX: if seen[hash] matches this pixel.
#         - QOI_OP_DIFF:  if all three deltas fit in -2..1.
#         - QOI_OP_LUMA:  if dg in -32..31 and dr_rel/db_rel in -8..7.
#         - QOI_OP_RGB or QOI_OP_RGBA: raw pixel.
#         Then store pixel in seen[hash].
#   4. Flush final run.
#   5. Append 8-byte end marker.
#
# @param $container  CodingAdventures::PixelContainer
# @return            Binary QOI file as a string
# @die               "EncodeError: ..." on bad input
# ============================================================================
sub encode_qoi {
    my ($container) = @_;

    die "EncodeError: expected a PixelContainer object"
        unless defined $container
            && ref($container)
            && $container->isa('CodingAdventures::PixelContainer');

    my $w = $container->width;
    my $h = $container->height;

    # -----------------------------------------------------------------------
    # Header (14 bytes)
    # -----------------------------------------------------------------------
    # Magic:      'q','o','i','f'  (4 bytes)
    # Width:       uint32 big-endian (4 bytes)
    # Height:      uint32 big-endian (4 bytes)
    # Channels:    4 = RGBA         (1 byte)
    # Colorspace:  0 = sRGB         (1 byte)
    my $out = pack('a4 N N C C', 'qoif', $w, $h, 4, 0);

    # -----------------------------------------------------------------------
    # Encoder state
    # -----------------------------------------------------------------------
    my ($pr, $pg, $pb, $pa) = (0, 0, 0, 255);   # previous pixel
    my $run  = 0;                                  # current run length
    # seen[]: 64-entry hash table initialised to (0,0,0,0).
    # Each entry is an arrayref [$r, $g, $b, $a].
    my @seen = map { [0, 0, 0, 0] } 0..63;

    # -----------------------------------------------------------------------
    # Encode pixels
    # -----------------------------------------------------------------------
    my $total = $w * $h;
    for my $i (0 .. $total - 1) {
        my $y = int($i / $w);
        my $x = $i % $w;
        my ($r, $g, $b, $a) = $container->pixel_at($x, $y);

        if ($r == $pr && $g == $pg && $b == $pb && $a == $pa) {
            # ---------------------------------------------------------------
            # QOI_OP_RUN: same as previous pixel
            # ---------------------------------------------------------------
            $run++;
            # Flush run at max length 62 (stored as 61 in 6 bits)
            if ($run == 62) {
                $out .= pack('C', QOI_OP_RUN | ($run - 1));
                $run = 0;
            }
        } else {
            # Flush any pending run before emitting a new chunk
            if ($run > 0) {
                $out .= pack('C', QOI_OP_RUN | ($run - 1));
                $run = 0;
            }

            my $idx = _hash_pixel($r, $g, $b, $a);

            if ($seen[$idx][0] == $r && $seen[$idx][1] == $g
             && $seen[$idx][2] == $b && $seen[$idx][3] == $a) {
                # -----------------------------------------------------------
                # QOI_OP_INDEX: pixel is in the seen table
                # -----------------------------------------------------------
                # Tag 00xxxxxx, where xxxxxx = table index
                $out .= pack('C', QOI_OP_INDEX | $idx);

            } else {
                # Compute signed deltas for DIFF / LUMA consideration
                my $dr = _signed_delta($r, $pr);
                my $dg = _signed_delta($g, $pg);
                my $db = _signed_delta($b, $pb);

                if ($a == $pa
                 && $dr >= -2 && $dr <= 1
                 && $dg >= -2 && $dg <= 1
                 && $db >= -2 && $db <= 1) {
                    # -------------------------------------------------------
                    # QOI_OP_DIFF: small deltas, alpha unchanged
                    # -------------------------------------------------------
                    # Pack three 2-bit biased values into one byte:
                    #   bits 7-6: 01 (tag)
                    #   bits 5-4: dr + 2   (0..3)
                    #   bits 3-2: dg + 2   (0..3)
                    #   bits 1-0: db + 2   (0..3)
                    $out .= pack('C',
                        QOI_OP_DIFF
                        | (($dr + 2) << 4)
                        | (($dg + 2) << 2)
                        |  ($db + 2)
                    );

                } else {
                    my $dr_rel = $dr - $dg;   # dr relative to dg
                    my $db_rel = $db - $dg;   # db relative to dg

                    if ($a == $pa
                     && $dg     >= -32 && $dg     <= 31
                     && $dr_rel >= -8  && $dr_rel <= 7
                     && $db_rel >= -8  && $db_rel <= 7) {
                        # ---------------------------------------------------
                        # QOI_OP_LUMA: medium deltas, alpha unchanged
                        # ---------------------------------------------------
                        # Byte 1: 10xxxxxx  where xxxxxx = dg + 32
                        # Byte 2: yyyyzzzz  where yyyy = dr_rel+8, zzzz = db_rel+8
                        $out .= pack('CC',
                            QOI_OP_LUMA | ($dg + 32),
                            (($dr_rel + 8) << 4) | ($db_rel + 8)
                        );

                    } elsif ($a == $pa) {
                        # ---------------------------------------------------
                        # QOI_OP_RGB: large delta but alpha unchanged
                        # ---------------------------------------------------
                        $out .= pack('CCCC', QOI_OP_RGB, $r, $g, $b);

                    } else {
                        # ---------------------------------------------------
                        # QOI_OP_RGBA: alpha changed
                        # ---------------------------------------------------
                        $out .= pack('CCCCC', QOI_OP_RGBA, $r, $g, $b, $a);
                    }
                }

                # Update the seen table with this pixel
                $seen[$idx] = [$r, $g, $b, $a];
            }

            # Update previous pixel
            ($pr, $pg, $pb, $pa) = ($r, $g, $b, $a);
        }
    }

    # Flush final run (if any)
    if ($run > 0) {
        $out .= pack('C', QOI_OP_RUN | ($run - 1));
    }

    # Append end-of-stream marker
    $out .= QOI_END_MARKER;

    return $out;
}

# ============================================================================
# decode_qoi($bytes) -> $container
# ============================================================================
#
# Decode a QOI binary string into a PixelContainer.
#
# Algorithm:
#   1. Validate magic 'qoif'.
#   2. Read width, height, channels, colorspace from header.
#   3. Initialise: prev = (0,0,0,255), run = 0, seen[64] = (0,0,0,0).
#   4. For each pixel slot (W*H total):
#      a. If run > 0: decrement run, use prev pixel.
#      b. Else: read next chunk byte and decode the appropriate op.
#   5. Verify end marker is present.
#
# @param $bytes  Binary string — raw QOI file
# @return        CodingAdventures::PixelContainer
# @die           "QOI: ..." on format errors
# ============================================================================
sub decode_qoi {
    my ($bytes) = @_;

    die "QOI: input must be a defined string" unless defined $bytes;
    die "QOI: file too short (need at least 22 bytes)"
        if length($bytes) < 22;   # 14 header + 8 end marker

    # --- Header -------------------------------------------------------------
    my $magic = substr($bytes, 0, 4);
    die "QOI: invalid magic '$magic' (expected 'qoif')" unless $magic eq 'qoif';

    # Width and height: uint32 big-endian (pack 'N')
    my ($w, $h) = unpack('N N', substr($bytes, 4, 8));
    die "QOI: width must be positive" unless $w > 0;
    die "QOI: height must be positive" unless $h > 0;

    my $channels    = unpack('C', substr($bytes, 12, 1));
    # colorspace at offset 13 — we read it but don't use it (display only)

    # --- Decoder state ------------------------------------------------------
    my ($pr, $pg, $pb, $pa) = (0, 0, 0, 255);   # previous pixel
    my $run = 0;                                   # remaining run pixels
    my @seen = map { [0, 0, 0, 0] } 0..63;        # seen-pixels table

    my $pos       = 14;          # current read position in $bytes
    my $file_len  = length($bytes);
    my $end_limit = $file_len - 8;   # end marker occupies the last 8 bytes

    my $container = CodingAdventures::PixelContainer->new($w, $h);
    my $total     = $w * $h;

    for my $i (0 .. $total - 1) {
        my $r = $pr; my $g = $pg; my $b = $pb; my $a = $pa;

        if ($run > 0) {
            # ---------------------------------------------------------------
            # Continuing a run: emit prev pixel again, consume one run slot
            # ---------------------------------------------------------------
            $run--;
            # r,g,b,a already set to prev pixel values above — no further action
        } else {
            # ---------------------------------------------------------------
            # Read next chunk
            # ---------------------------------------------------------------
            die "QOI: unexpected end of data at pixel $i"
                if $pos >= $end_limit;

            my $b1 = unpack('C', substr($bytes, $pos, 1));
            $pos++;

            if ($b1 == QOI_OP_RGBA) {
                # -----------------------------------------------------------
                # QOI_OP_RGBA (0xFF): next 4 bytes = R G B A
                # -----------------------------------------------------------
                die "QOI: unexpected end of data reading RGBA"
                    if $pos + 4 > $file_len;
                ($r, $g, $b, $a) = unpack('CCCC', substr($bytes, $pos, 4));
                $pos += 4;

            } elsif ($b1 == QOI_OP_RGB) {
                # -----------------------------------------------------------
                # QOI_OP_RGB (0xFE): next 3 bytes = R G B; A unchanged
                # -----------------------------------------------------------
                die "QOI: unexpected end of data reading RGB"
                    if $pos + 3 > $file_len;
                ($r, $g, $b) = unpack('CCC', substr($bytes, $pos, 3));
                $pos += 3;
                $a = $pa;   # keep previous alpha

            } elsif (($b1 & 0xC0) == QOI_OP_RUN) {
                # -----------------------------------------------------------
                # QOI_OP_RUN (11xxxxxx): run of identical pixels
                # xxxxxx = run_length - 1  (0..61)
                # The current pixel is the same as prev; the run covers
                # (xxxxxx + 1) pixels total, but we're emitting this first
                # pixel now, so we set run = xxxxxx (remaining after this one).
                # -----------------------------------------------------------
                $run = ($b1 & 0x3F);   # 0..61 remaining after current
                # r,g,b,a already == prev pixel (unchanged from start of iteration)

            } elsif (($b1 & 0xC0) == QOI_OP_INDEX) {
                # -----------------------------------------------------------
                # QOI_OP_INDEX (00xxxxxx): recall seen pixel at index xxxxxx
                # -----------------------------------------------------------
                my $idx = $b1 & 0x3F;
                ($r, $g, $b, $a) = @{$seen[$idx]};

            } elsif (($b1 & 0xC0) == QOI_OP_DIFF) {
                # -----------------------------------------------------------
                # QOI_OP_DIFF (01xxxxxx): small deltas packed into 6 bits
                # bits 5-4: dr + 2   → dr = bits - 2
                # bits 3-2: dg + 2   → dg = bits - 2
                # bits 1-0: db + 2   → db = bits - 2
                # Apply modulo-256 addition (Perl % 256 handles wrap-around)
                # -----------------------------------------------------------
                my $dr = (($b1 >> 4) & 0x03) - 2;
                my $dg = (($b1 >> 2) & 0x03) - 2;
                my $db = ( $b1       & 0x03) - 2;
                $r = ($pr + $dr) & 0xFF;
                $g = ($pg + $dg) & 0xFF;
                $b = ($pb + $db) & 0xFF;
                $a = $pa;

            } elsif (($b1 & 0xC0) == QOI_OP_LUMA) {
                # -----------------------------------------------------------
                # QOI_OP_LUMA (10xxxxxx + 1 more byte): medium deltas
                # Byte 1 bits 5-0: dg + 32
                # Byte 2 bits 7-4: dr_rel + 8  (dr_rel = dr - dg)
                # Byte 2 bits 3-0: db_rel + 8  (db_rel = db - dg)
                # -----------------------------------------------------------
                die "QOI: unexpected end of data reading LUMA"
                    if $pos >= $end_limit;
                my $b2 = unpack('C', substr($bytes, $pos, 1));
                $pos++;

                my $dg     = ($b1 & 0x3F) - 32;
                my $dr_rel = (($b2 >> 4) & 0x0F) - 8;
                my $db_rel = ( $b2       & 0x0F) - 8;
                my $dr     = $dr_rel + $dg;
                my $db_d   = $db_rel + $dg;
                $r = ($pr + $dr)  & 0xFF;
                $g = ($pg + $dg)  & 0xFF;
                $b = ($pb + $db_d) & 0xFF;
                $a = $pa;

            } else {
                die "QOI: unknown chunk byte 0x" . sprintf('%02X', $b1);
            }
        }

        # Update the seen table and prev pixel.
        # During a run, r/g/b/a == pr/pg/pb/pa already, so the seen
        # table entry is already correct — but we still update prev to be safe.
        my $idx = _hash_pixel($r, $g, $b, $a);
        $seen[$idx] = [$r, $g, $b, $a];

        # Update previous pixel
        ($pr, $pg, $pb, $pa) = ($r, $g, $b, $a);

        # Store in container
        my $y = int($i / $w);
        my $x = $i % $w;
        $container->set_pixel($x, $y, $r, $g, $b, $a);
    }

    # Verify end marker
    my $end = substr($bytes, $file_len - 8, 8);
    die "QOI: missing or corrupt end marker"
        unless $end eq QOI_END_MARKER;

    return $container;
}

1;

__END__

=head1 NAME

CodingAdventures::ImageCodecQOI - QOI (Quite OK Image) encode/decode (IC03)

=head1 SYNOPSIS

    use lib '../pixel-container/lib';
    use CodingAdventures::PixelContainer;
    use CodingAdventures::ImageCodecQOI qw(encode_qoi decode_qoi);

    # Encode
    my $img   = CodingAdventures::PixelContainer->new(64, 64);
    $img->fill_pixels(255, 0, 0, 255);   # solid red
    my $bytes = encode_qoi($img);

    # Decode
    my $img2          = decode_qoi($bytes);
    my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
    # ($r, $g, $b, $a) == (255, 0, 0, 255)

=head1 DESCRIPTION

Implements the QOI (Quite OK Image) lossless codec for
L<CodingAdventures::PixelContainer> objects.  Part of the IC03 layer.

Supports all 6 QOI chunk types: RGB, RGBA, INDEX, DIFF, LUMA, RUN.

=head1 FUNCTIONS

=over 4

=item C<mime_type()>

Returns C<'image/qoi'>.

=item C<encode_qoi($container)>

Encode a C<PixelContainer> as a QOI binary string.
Dies with C<"EncodeError: ..."> if the input is not a PixelContainer.

=item C<decode_qoi($bytes)>

Decode a QOI binary string into a C<PixelContainer>.
Dies with C<"QOI: ..."> on format errors.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
