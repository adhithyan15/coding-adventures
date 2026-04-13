package CodingAdventures::Deflate;

use strict;
use warnings;
use CodingAdventures::HuffmanTree;
use CodingAdventures::LZSS qw(encode);

our $VERSION = '0.1.0';
use Exporter 'import';
our @EXPORT_OK = qw(compress decompress);

=head1 NAME

CodingAdventures::Deflate - CMP05 DEFLATE compression/decompression

=head1 SYNOPSIS

  use CodingAdventures::Deflate qw(compress decompress);

  my $data       = "hello hello hello world";
  my $compressed = compress($data);
  my $original   = decompress($compressed);  # "hello hello hello world"

=head1 DESCRIPTION

DEFLATE (1996, RFC 1951) is the dominant general-purpose lossless compression
algorithm. It combines two complementary techniques:

=over 4

=item * B<LZSS tokenization> (CMP02) — replaces repeated substrings with
back-references into a 4096-byte sliding window.

=item * B<Dual canonical Huffman coding> (DT27) — entropy-codes the token
stream with two Huffman trees:
LL tree (literals 0-255, end-of-data 256, length codes 257-284) and
Dist tree (distance codes 0-23, for offsets 1-4096).

=back

=head2 Wire Format (CMP05)

  [4B] original_length    big-endian uint32
  [2B] ll_entry_count     big-endian uint16
  [2B] dist_entry_count   big-endian uint16 (0 if no matches)
  [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
  [dist_entry_count × 3B] same format
  [remaining bytes]       LSB-first packed bit stream

=cut

# ---------------------------------------------------------------------------
# Length code table (LL symbols 257-284)
# ---------------------------------------------------------------------------
#
# Each length symbol covers a range of match lengths. The exact length within
# the range is encoded as extra_bits raw bits after the Huffman code.
#
# Table: [symbol, base_length, extra_bits]

my @LENGTH_TABLE = (
    [257,   3, 0], [258,   4, 0], [259,   5, 0], [260,   6, 0],
    [261,   7, 0], [262,   8, 0], [263,   9, 0], [264,  10, 0],
    [265,  11, 1], [266,  13, 1], [267,  15, 1], [268,  17, 1],
    [269,  19, 2], [270,  23, 2], [271,  27, 2], [272,  31, 2],
    [273,  35, 3], [274,  43, 3], [275,  51, 3], [276,  59, 3],
    [277,  67, 4], [278,  83, 4], [279,  99, 4], [280, 115, 4],
    [281, 131, 5], [282, 163, 5], [283, 195, 5], [284, 227, 5],
);

my %LENGTH_BASE  = map { $_->[0] => $_->[1] } @LENGTH_TABLE;
my %LENGTH_EXTRA = map { $_->[0] => $_->[2] } @LENGTH_TABLE;

# ---------------------------------------------------------------------------
# Distance code table (codes 0-23)
# ---------------------------------------------------------------------------
#
# Table: [code, base_dist, extra_bits]

my @DIST_TABLE = (
    [ 0,    1,  0], [ 1,    2,  0], [ 2,    3,  0], [ 3,    4,  0],
    [ 4,    5,  1], [ 5,    7,  1], [ 6,    9,  2], [ 7,   13,  2],
    [ 8,   17,  3], [ 9,   25,  3], [10,   33,  4], [11,   49,  4],
    [12,   65,  5], [13,   97,  5], [14,  129,  6], [15,  193,  6],
    [16,  257,  7], [17,  385,  7], [18,  513,  8], [19,  769,  8],
    [20, 1025,  9], [21, 1537,  9], [22, 2049, 10], [23, 3073, 10],
);

my %DIST_BASE  = map { $_->[0] => $_->[1] } @DIST_TABLE;
my %DIST_EXTRA = map { $_->[0] => $_->[2] } @DIST_TABLE;

# ---------------------------------------------------------------------------
# Helper: length_symbol(length) → LL symbol (257-284)
# ---------------------------------------------------------------------------

sub _length_symbol {
    my ($length) = @_;
    for my $e (@LENGTH_TABLE) {
        my $max_len = $e->[1] + (1 << $e->[2]) - 1;
        return $e->[0] if $length <= $max_len;
    }
    return 284;
}

# ---------------------------------------------------------------------------
# Helper: dist_code(offset) → distance code (0-23)
# ---------------------------------------------------------------------------

sub _dist_code {
    my ($offset) = @_;
    for my $e (@DIST_TABLE) {
        my $max_dist = $e->[1] + (1 << $e->[2]) - 1;
        return $e->[0] if $offset <= $max_dist;
    }
    return 23;
}

# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

sub _pack_bits_lsb_first {
    my ($bits) = @_;  # string of '0'/'1' characters
    my @output;
    my $buffer = 0;
    my $bit_pos = 0;
    for my $ch (split //, $bits) {
        $buffer |= 1 << $bit_pos if $ch eq '1';
        $bit_pos++;
        if ($bit_pos == 8) {
            push @output, $buffer;
            $buffer = 0;
            $bit_pos = 0;
        }
    }
    push @output, $buffer if $bit_pos > 0;
    return pack('C*', @output);
}

sub _unpack_bits_lsb_first {
    my ($data) = @_;
    my $bits = '';
    for my $byte (unpack 'C*', $data) {
        for my $i (0..7) {
            $bits .= (($byte >> $i) & 1) ? '1' : '0';
        }
    }
    return $bits;
}

# ---------------------------------------------------------------------------
# Canonical code reconstruction
# ---------------------------------------------------------------------------

sub _reconstruct_canonical_codes {
    my ($lengths) = @_;  # arrayref of [symbol, code_length] sorted
    return {} unless @$lengths;
    if (@$lengths == 1) {
        return { '0' => $lengths->[0][0] };
    }
    my %result;
    my $code = 0;
    my $prev_len = $lengths->[0][1];
    for my $pair (@$lengths) {
        my ($sym, $code_len) = @$pair;
        if ($code_len > $prev_len) {
            $code <<= ($code_len - $prev_len);
        }
        my $bit_str = sprintf("%0${code_len}b", $code);
        $result{$bit_str} = $sym;
        $code++;
        $prev_len = $code_len;
    }
    return \%result;
}

# ---------------------------------------------------------------------------
# Public API: compress
# ---------------------------------------------------------------------------

=head2 compress($data)

Compress a string using DEFLATE (CMP05) and return wire-format bytes.

=cut

sub compress {
    my ($data) = @_;
    my $original_length = length($data);

    if ($original_length == 0) {
        # Empty input: LL tree has only symbol 256 (end-of-data), code "0".
        return pack('N', 0)            # original_length = 0
             . pack('n', 1)            # ll_entry_count = 1
             . pack('n', 0)            # dist_entry_count = 0
             . pack('nC', 256, 1)      # symbol=256, code_length=1
             . "\x00";                 # bit stream: code "0" = 0x00
    }

    # Pass 1: LZSS tokenization.
    # encode() takes a byte string (not an array ref) and returns a list of tokens.
    my @tokens = encode($data);

    # Pass 2a: Tally frequencies.
    my (%ll_freq, %dist_freq);
    for my $tok (@tokens) {
        if ($tok->{kind} eq 'literal') {
            $ll_freq{$tok->{byte}}++;
        } else {
            my $sym = _length_symbol($tok->{length});
            $ll_freq{$sym}++;
            my $dc = _dist_code($tok->{offset});
            $dist_freq{$dc}++;
        }
    }
    $ll_freq{256}++;  # end-of-data marker

    # Pass 2b: Build canonical Huffman trees.
    my $ll_tree = CodingAdventures::HuffmanTree->build(
        [ map { [$_, $ll_freq{$_}] } keys %ll_freq ]
    );
    my $ll_code_table = $ll_tree->canonical_code_table();  # {symbol → bit_string}

    my $dist_code_table = {};
    if (%dist_freq) {
        my $dist_tree = CodingAdventures::HuffmanTree->build(
            [ map { [$_, $dist_freq{$_}] } keys %dist_freq ]
        );
        $dist_code_table = $dist_tree->canonical_code_table();
    }

    # Pass 2c: Encode token stream to bit string.
    my $bits = '';
    for my $tok (@tokens) {
        if ($tok->{kind} eq 'literal') {
            $bits .= $ll_code_table->{$tok->{byte}};
        } else {
            my $sym = _length_symbol($tok->{length});
            my $extra_bits_count = $LENGTH_EXTRA{$sym} // 0;
            my $extra_val = $tok->{length} - ($LENGTH_BASE{$sym} // 0);

            my $dc = _dist_code($tok->{offset});
            my $dist_extra_bits = $DIST_EXTRA{$dc} // 0;
            my $dist_extra_val = $tok->{offset} - ($DIST_BASE{$dc} // 0);

            $bits .= $ll_code_table->{$sym};
            # Extra bits for length, LSB-first.
            for my $i (0 .. $extra_bits_count - 1) {
                $bits .= (($extra_val >> $i) & 1) ? '1' : '0';
            }
            $bits .= $dist_code_table->{$dc};
            # Extra bits for distance, LSB-first.
            for my $i (0 .. $dist_extra_bits - 1) {
                $bits .= (($dist_extra_val >> $i) & 1) ? '1' : '0';
            }
        }
    }
    # End-of-data symbol.
    $bits .= $ll_code_table->{256};

    my $bit_stream = _pack_bits_lsb_first($bits);

    # Assemble wire format.
    my @ll_lengths = sort {
        $a->[1] <=> $b->[1] || $a->[0] <=> $b->[0]
    } map { [$_, length($ll_code_table->{$_})] } keys %$ll_code_table;

    my @dist_lengths = sort {
        $a->[1] <=> $b->[1] || $a->[0] <=> $b->[0]
    } map { [$_, length($dist_code_table->{$_})] } keys %$dist_code_table;

    my $ll_count = scalar @ll_lengths;
    my $dist_count = scalar @dist_lengths;

    my $header = pack('N', $original_length)
               . pack('n', $ll_count)
               . pack('n', $dist_count);

    my $ll_bytes   = join('', map { pack('nC', $_->[0], $_->[1]) } @ll_lengths);
    my $dist_bytes = join('', map { pack('nC', $_->[0], $_->[1]) } @dist_lengths);

    return $header . $ll_bytes . $dist_bytes . $bit_stream;
}

# ---------------------------------------------------------------------------
# Public API: decompress
# ---------------------------------------------------------------------------

=head2 decompress($data)

Decompress CMP05 wire-format data and return the original string.

=cut

sub decompress {
    my ($data) = @_;
    return '' if length($data) < 8;

    my ($original_length, $ll_entry_count, $dist_entry_count)
        = unpack('NnN', $data);
    # Actually parse 4+2+2 = 8 bytes
    ($original_length, $ll_entry_count, $dist_entry_count)
        = unpack('Nnn', $data);

    return '' if $original_length == 0;

    my $off = 8;

    # Parse LL code-length table.
    my @ll_lengths;
    for my $i (0 .. $ll_entry_count - 1) {
        my ($sym, $code_len) = unpack('nC', substr($data, $off, 3));
        push @ll_lengths, [$sym, $code_len];
        $off += 3;
    }

    # Parse dist code-length table.
    my @dist_lengths;
    for my $i (0 .. $dist_entry_count - 1) {
        my ($sym, $code_len) = unpack('nC', substr($data, $off, 3));
        push @dist_lengths, [$sym, $code_len];
        $off += 3;
    }

    # Reconstruct canonical codes (bit_string → symbol).
    my $ll_rev_map   = _reconstruct_canonical_codes(\@ll_lengths);
    my $dist_rev_map = _reconstruct_canonical_codes(\@dist_lengths);

    # Unpack bit stream.
    my $bits = _unpack_bits_lsb_first(substr($data, $off));
    my $bit_pos = 0;

    my $read_bits = sub {
        my ($n) = @_;
        return 0 if $n == 0;
        my $val = 0;
        for my $i (0 .. $n - 1) {
            $val |= (substr($bits, $bit_pos + $i, 1) eq '1') ? (1 << $i) : 0;
        }
        $bit_pos += $n;
        return $val;
    };

    my $next_huffman_symbol = sub {
        my ($rev_map) = @_;
        my $acc = '';
        while (1) {
            $acc .= substr($bits, $bit_pos, 1);
            $bit_pos++;
            my $sym = $rev_map->{$acc};
            return $sym if defined $sym;
        }
    };

    # Decode token stream.
    my @output;
    while (1) {
        my $ll_sym = $next_huffman_symbol->($ll_rev_map);

        if ($ll_sym == 256) {
            last;  # end-of-data
        } elsif ($ll_sym < 256) {
            push @output, $ll_sym;  # literal byte
        } else {
            # Length code 257-284.
            my $extra = $LENGTH_EXTRA{$ll_sym} // 0;
            my $length_val = ($LENGTH_BASE{$ll_sym} // 0) + $read_bits->($extra);

            my $dist_sym  = $next_huffman_symbol->($dist_rev_map);
            my $dextra    = $DIST_EXTRA{$dist_sym} // 0;
            my $dist_off  = ($DIST_BASE{$dist_sym} // 0) + $read_bits->($dextra);

            # Copy byte-by-byte (supports overlapping matches).
            my $start = scalar(@output) - $dist_off;
            for my $i (0 .. $length_val - 1) {
                push @output, $output[$start + $i];
            }
        }
    }

    return pack('C*', @output);
}

1;

__END__

=head1 SEE ALSO

=over 4

=item * L<CodingAdventures::LZSS> — LZSS tokenizer (CMP02)

=item * L<CodingAdventures::HuffmanTree> — Huffman tree builder (DT27)

=back

=head1 AUTHOR

Adhithya Rajasekaran

=head1 LICENSE

MIT

=cut
