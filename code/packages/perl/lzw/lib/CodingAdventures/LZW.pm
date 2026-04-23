package CodingAdventures::LZW;

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::LZW - LZW lossless compression algorithm (1984)

=head1 SYNOPSIS

  use CodingAdventures::LZW qw(compress decompress);

  my $data       = "ABABABABAB";
  my $compressed = compress($data);
  my $original   = decompress($compressed);

=head1 DESCRIPTION

LZW (Lempel-Ziv-Welch, 1984) is LZ78 with a twist: the dictionary is
pre-seeded with all 256 single-byte sequences before encoding begins.
Because every possible single byte is already in the dictionary, the
encoder never needs to emit a raw literal — it emits pure I<codes>.

This is exactly the algorithm that powers GIF image compression.

=head2 Pre-Seeded Dictionary

In LZ78 the encoder had to emit a C<next_char> byte alongside each code
so the decoder could add the new entry. In LZW there is no next_char — the
decoder can reconstruct every entry because single bytes are already known.

  Codes 0–255    Pre-seeded single bytes (\x00 through \xFF).
  Code  256      ClearCode — reset the dictionary to the initial 256 entries.
  Code  257      StopCode  — signals the end of the code stream.
  Code  258+     Dynamically assigned as new multi-byte sequences are found.

=head2 Variable-Width Bit Packing

Codes are packed LSB-first (GIF convention):

  - Start at 9 bits (covers codes 0–511, giving room for codes 258+).
  - Each time C<next_code> crosses the next power-of-two boundary, the
    bit width increments by one.
  - Maximum bit width is 16; at that point new sequences are silently
    discarded (dictionary full) and a ClearCode resets everything.

=head2 Wire Format (CMP03)

  Bytes 0–3:  original_length (big-endian uint32)
  Bytes 4+:   LSB-first variable-width bit-packed codes

=head2 The Tricky Token

During decoding, the decoder may receive a code C<$code> where
C<$code == scalar @dec_dict> (not yet added to the dictionary). This
happens when the input has the repeated-prefix form C<xyx...x>. The fix:

  $entry = [ @{$dec_dict[$prev_code]}, $dec_dict[$prev_code][0] ]

=head2 The Series: CMP00 to CMP05

  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie), no window.
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialized dict; GIF. (this module)
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

=cut

use Exporter 'import';
our @EXPORT_OK = qw(
    compress decompress
    encode_codes decode_codes pack_codes unpack_codes
);

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

=head1 CONSTANTS

=over 4

=item CLEAR_CODE (256)

Instructs the decoder to reset its dictionary and code size back to the
initial 9-bit, 258-entry state.

=item STOP_CODE (257)

Marks the end of the compressed code stream. The decoder stops here.

=item INITIAL_NEXT_CODE (258)

The first code number available for dynamically assigned entries.

=item INITIAL_CODE_SIZE (9)

Starting bit-width for codes. 9 bits covers codes 0–511, which is enough
room for the initial 258 entries plus a few new ones before the width grows.

=item MAX_CODE_SIZE (16)

Maximum bit-width. The dictionary can hold at most 2**16 = 65536 entries.
When C<next_code> would exceed this cap, a ClearCode is emitted and the
dictionary is reset.

=back

=cut

use constant CLEAR_CODE        => 256;
use constant STOP_CODE         => 257;
use constant INITIAL_NEXT_CODE => 258;
use constant INITIAL_CODE_SIZE => 9;
use constant MAX_CODE_SIZE     => 16;

# ---------------------------------------------------------------------------
# Bit Writer
# ---------------------------------------------------------------------------

=head1 INTERNAL: BIT WRITER

C<_bw_new>, C<_bw_write>, C<_bw_flush> implement a LSB-first bit packer.

  my $w = _bw_new();
  _bw_write($w, $code, $code_size);
  _bw_flush($w);
  my @bytes = @{$w->{bytes}};

The writer stores a 64-bit accumulator (C<buffer>) and a count of how many
bits are currently sitting in that accumulator (C<bit_pos>). Each call to
C<_bw_write> OR-s the new code into the accumulator at position C<bit_pos>,
then drains complete bytes (8 bits at a time) into the C<bytes> array.

=cut

# _bw_new returns a fresh bit-writer state.
sub _bw_new {
    return { buffer => 0, bit_pos => 0, bytes => [] };
}

# _bw_write packs a single code into the bit stream.
#
# @param $w         (hashref) bit-writer state.
# @param $code      (int) the code to write.
# @param $code_size (int) bit-width for this code.
sub _bw_write {
    my ($w, $code, $code_size) = @_;
    $w->{buffer} |= ($code << $w->{bit_pos});
    $w->{bit_pos} += $code_size;
    while ($w->{bit_pos} >= 8) {
        push @{$w->{bytes}}, $w->{buffer} & 0xFF;
        $w->{buffer} >>= 8;
        $w->{bit_pos} -= 8;
    }
}

# _bw_flush flushes any remaining bits as a final partial byte.
#
# @param $w (hashref) bit-writer state.
sub _bw_flush {
    my ($w) = @_;
    if ($w->{bit_pos} > 0) {
        push @{$w->{bytes}}, $w->{buffer} & 0xFF;
        $w->{buffer}  = 0;
        $w->{bit_pos} = 0;
    }
}

# ---------------------------------------------------------------------------
# Bit Reader
# ---------------------------------------------------------------------------

=head1 INTERNAL: BIT READER

C<_br_new> and C<_br_read> implement a LSB-first bit unpacker matching the
writer exactly.

  my $r = _br_new(\@bytes);
  my $code = _br_read($r, $code_size);   # undef at end of stream

The reader loads bytes from the input array into a 64-bit accumulator on
demand. Each call extracts C<$code_size> bits from the low end.

=cut

# _br_new creates a bit-reader over a byte array.
#
# @param $bytes_ref (arrayref of ints) the packed bit stream.
# @return (hashref) reader state.
sub _br_new {
    my ($bytes_ref) = @_;
    return { bytes => $bytes_ref, pos => 0, buffer => 0, bit_pos => 0 };
}

# _br_read reads the next $code_size-bit code from the stream.
#
# Returns undef when the stream is exhausted.
#
# @param $r         (hashref) reader state.
# @param $code_size (int) number of bits to read.
# @return (int|undef) decoded code, or undef at EOF.
sub _br_read {
    my ($r, $code_size) = @_;
    while ($r->{bit_pos} < $code_size) {
        return undef if $r->{pos} >= scalar @{$r->{bytes}};
        $r->{buffer} |= ($r->{bytes}[$r->{pos}] << $r->{bit_pos});
        $r->{pos}++;
        $r->{bit_pos} += 8;
    }
    my $mask = (1 << $code_size) - 1;
    my $code = $r->{buffer} & $mask;
    $r->{buffer}  >>= $code_size;
    $r->{bit_pos}  -= $code_size;
    return $code;
}

# ---------------------------------------------------------------------------
# Encoder
# ---------------------------------------------------------------------------

=head1 INTERNAL: ENCODER

=head2 encode_codes

  my @codes = encode_codes($data);

Encodes a byte string into an array of LZW integer codes including the
leading ClearCode and trailing StopCode.

Algorithm:

=over 4

=item 1.

Initialise C<enc_dict> with all 256 single-byte string keys mapped to
codes 0–255.

=item 2.

Walk the input byte by byte, extending the current prefix C<$w>. If
C<$w . $byte> exists in the dictionary, extend C<$w>. Otherwise, emit the
code for C<$w>, add C<$w . $byte> as C<$next_code>, and reset C<$w = $byte>.

=item 3.

If the dictionary would overflow (C<next_code == 2**MAX_CODE_SIZE>), emit a
ClearCode and rebuild the dictionary from scratch before adding the new entry.

=item 4.

After the last byte, flush the current prefix and append StopCode.

=back

=cut

sub encode_codes {
    my ($data) = @_;

    my @bytes       = unpack 'C*', $data;
    my $max_entries = (1 << MAX_CODE_SIZE);

    # Build the initial encode dictionary.
    # Keys are binary strings (pack 'C*'): single bytes map to codes 0–255.
    # Using pack() gives a consistent key scheme for all sequence lengths.
    my %enc_dict;
    for my $b (0 .. 255) {
        $enc_dict{ pack('C', $b) } = $b;
    }

    my $next_code = INITIAL_NEXT_CODE;
    my @codes     = (CLEAR_CODE);
    my $w         = '';    # current working prefix as a binary string

    for my $b (@bytes) {
        my $wb = $w . pack('C', $b);

        if (exists $enc_dict{$wb}) {
            # Extend the current prefix.
            $w = $wb;
        } else {
            # Emit code for current prefix w.
            push @codes, $enc_dict{$w};

            if ($next_code < $max_entries) {
                # Add new sequence to dictionary.
                $enc_dict{$wb} = $next_code;
                $next_code++;
            } elsif ($next_code == $max_entries) {
                # Dictionary full — emit ClearCode and rebuild from scratch.
                push @codes, CLEAR_CODE;
                %enc_dict = ();
                for my $i (0 .. 255) {
                    $enc_dict{ pack('C', $i) } = $i;
                }
                $next_code = INITIAL_NEXT_CODE;
            }

            # Reset prefix to the current byte alone.
            $w = pack('C', $b);
        }
    }

    # Flush remaining prefix.
    if (length($w) > 0) {
        push @codes, $enc_dict{$w};
    }

    push @codes, STOP_CODE;
    return @codes;
}

# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------

=head1 INTERNAL: DECODER

=head2 decode_codes

  my $data = decode_codes(\@codes);

Decodes an array of LZW codes back into a byte string.

The decode dictionary is an array of array-refs (byte sequences). It starts
with the 258-entry base (256 bytes + 2 placeholder slots for ClearCode and
StopCode). New entries are added as C<$dec_dict[$prev_code]> extended by the
first byte of the current entry.

=head3 The Tricky Token

When the decoder receives a code C<$code == scalar @dec_dict> (the very next
code that I<would> be added), the encoder has emitted a sequence of the form
C<xyx...x> where the new entry is C<prev_entry . prev_entry[0]>. The
decoder constructs this speculatively:

  $entry = [ @{$dec_dict[$prev_code]}, $dec_dict[$prev_code][0] ]

=cut

sub decode_codes {
    my ($codes_ref) = @_;

    # Initialise the decode dictionary.
    # Slot $i (0..255) = [ $i ] (single byte).
    # Slots 256 and 257 are placeholders for ClearCode and StopCode.
    my @dec_dict;
    for my $b (0 .. 255) {
        $dec_dict[$b] = [$b];
    }
    $dec_dict[CLEAR_CODE] = undef;
    $dec_dict[STOP_CODE]  = undef;

    my $next_code = INITIAL_NEXT_CODE;
    my @output;
    my $prev_code = undef;  # undef = no previous code yet

    for my $code (@$codes_ref) {

        if ($code == CLEAR_CODE) {
            # Reset dictionary to base 256 entries.
            @dec_dict = ();
            for my $b (0 .. 255) {
                $dec_dict[$b] = [$b];
            }
            $dec_dict[CLEAR_CODE] = undef;
            $dec_dict[STOP_CODE]  = undef;
            $next_code = INITIAL_NEXT_CODE;
            $prev_code = undef;
            next;
        }

        last if $code == STOP_CODE;

        my $entry;

        if ($code < scalar @dec_dict && defined $dec_dict[$code]) {
            # Normal case: code is already in the dictionary.
            $entry = $dec_dict[$code];
        } elsif ($code == scalar @dec_dict) {
            # Tricky token: code is the one we're about to add.
            # This happens with sequences like xyx...x.
            # Guard: we must have a previous code.
            unless (defined $prev_code) {
                next;  # malformed — skip
            }
            my @prev = @{$dec_dict[$prev_code]};
            $entry = [@prev, $prev[0]];
        } else {
            next;  # invalid code — skip
        }

        push @output, @$entry;

        # Add new dictionary entry: prev_entry + entry[0].
        if (defined $prev_code && $next_code < (1 << MAX_CODE_SIZE)) {
            my @new_entry = (@{$dec_dict[$prev_code]}, $entry->[0]);
            push @dec_dict, \@new_entry;
            $next_code++;
        }

        $prev_code = $code;
    }

    return pack 'C*', @output;
}

# ---------------------------------------------------------------------------
# Serialisation: pack_codes
# ---------------------------------------------------------------------------

=head1 INTERNAL: SERIALISATION

=head2 pack_codes

  my $binary = pack_codes(\@codes, $original_length);

Packs an array of LZW codes into the CMP03 wire format:

  [4 bytes big-endian original_length] [variable-width LSB-first codes]

The code size starts at C<INITIAL_CODE_SIZE> (9) and grows whenever
C<next_code> crosses the next power-of-two boundary, matching the same
C<next_code> tracking logic used during encoding.

ClearCode resets C<code_size> and C<next_code> back to their initial values.

=cut

sub pack_codes {
    my ($codes_ref, $original_length) = @_;
    $original_length //= 0;

    my $w         = _bw_new();
    my $code_size = INITIAL_CODE_SIZE;
    my $next_code = INITIAL_NEXT_CODE;

    for my $code (@$codes_ref) {
        _bw_write($w, $code, $code_size);

        if ($code == CLEAR_CODE) {
            $code_size = INITIAL_CODE_SIZE;
            $next_code = INITIAL_NEXT_CODE;
        } elsif ($code != STOP_CODE) {
            # Track next_code for every data code (not CLEAR, not STOP).
            if ($next_code < (1 << MAX_CODE_SIZE)) {
                $next_code++;
                if ($next_code > (1 << $code_size) && $code_size < MAX_CODE_SIZE) {
                    $code_size++;
                }
            }
        }
    }

    _bw_flush($w);

    my $header = pack('N', $original_length);
    return $header . pack('C*', @{$w->{bytes}});
}

# ---------------------------------------------------------------------------
# Serialisation: unpack_codes
# ---------------------------------------------------------------------------

=head2 unpack_codes

  my ($codes_ref, $original_length) = unpack_codes($binary);

Reads a CMP03 wire-format byte string, returning an array-ref of integer
codes and the original uncompressed length.

Returns C<([CLEAR_CODE, STOP_CODE], 0)> for input shorter than 4 bytes.

The C<code_size> and C<next_code> are tracked in lock-step with the encoder:
every non-CLEAR, non-STOP code increments C<next_code> and may trigger a
C<code_size> bump.

=cut

sub unpack_codes {
    my ($data) = @_;
    if (length($data) < 4) {
        return ([CLEAR_CODE, STOP_CODE], 0);
    }

    my $original_length = unpack('N', substr($data, 0, 4));
    my @bytes = unpack('C*', substr($data, 4));

    my $r         = _br_new(\@bytes);
    my $code_size = INITIAL_CODE_SIZE;
    my $next_code = INITIAL_NEXT_CODE;
    my @codes;

    while (1) {
        my $code = _br_read($r, $code_size);
        last unless defined $code;

        push @codes, $code;

        if ($code == STOP_CODE) {
            last;
        } elsif ($code == CLEAR_CODE) {
            $code_size = INITIAL_CODE_SIZE;
            $next_code = INITIAL_NEXT_CODE;
        } else {
            if ($next_code < (1 << MAX_CODE_SIZE)) {
                $next_code++;
                if ($next_code > (1 << $code_size) && $code_size < MAX_CODE_SIZE) {
                    $code_size++;
                }
            }
        }
    }

    return (\@codes, $original_length);
}

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

=head1 PUBLIC API

=head2 compress

  my $compressed = compress($data);

One-shot compression. Encodes C<$data> to LZW codes then packs to CMP03
wire format.

=cut

sub compress {
    my ($data) = @_;
    my @codes = encode_codes($data);
    return pack_codes(\@codes, length($data));
}

=head2 decompress

  my $data = decompress($compressed);

One-shot decompression. Unpacks CMP03 wire format then decodes to the
original byte string.

=cut

sub decompress {
    my ($compressed) = @_;
    my ($codes_ref, $original_length) = unpack_codes($compressed);
    my $result = decode_codes($codes_ref);

    # Trim to original_length in case of rounding from bit-packing.
    if (length($result) > $original_length) {
        return substr($result, 0, $original_length);
    }
    return $result;
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
