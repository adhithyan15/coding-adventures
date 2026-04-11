package CodingAdventures::LZ77;

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::LZ77 - LZ77 lossless compression algorithm (1977)

=head1 SYNOPSIS

  use CodingAdventures::LZ77 qw(encode decode compress decompress);

  my $data = "hello hello hello world";
  my $compressed = compress($data);
  my $original   = decompress($compressed);

=head1 DESCRIPTION

LZ77 (Lempel & Ziv, 1977) is the foundational sliding-window compression
algorithm. It replaces repeated byte sequences with compact backreferences
into a window of recently seen data. It is the ancestor of DEFLATE, gzip,
PNG, and zlib.

=head2 The Sliding Window Model

    +-----------------------------------+------------------+
    |         SEARCH BUFFER            | LOOKAHEAD BUFFER |
    |  (already processed, last        | (not yet seen,   |
    |   window_size bytes)             |  next max_match) |
    +-----------------------------------+------------------+
                                        ^
                                    cursor (current position)

At each step the encoder finds the longest match in the search buffer. If
found and long enough (>= min_match), emit a backreference token. Otherwise
emit a literal token.

=head2 Token: {offset, length, next_char}

=over 4

=item * offset:    distance back the match starts (1..window_size), or 0.

=item * length:    number of bytes the match covers (0 = literal).

=item * next_char: literal byte immediately after the match.

=back

=head2 Overlapping Matches

When offset < length, the match extends into bytes not yet decoded. The
decoder must copy byte-by-byte (not bulk) to handle this correctly.

=head2 The Series: CMP00 to CMP05

  CMP00 (LZ77, 1977) -- Sliding-window backreferences. This module.
  CMP01 (LZ78, 1978) -- Explicit dictionary (trie), no sliding window.
  CMP02 (LZSS, 1982) -- LZ77 + flag bits; eliminates wasted literals.
  CMP03 (LZW,  1984) -- Pre-initialized dictionary; powers GIF.
  CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

=cut

use Exporter 'import';
our @EXPORT_OK = qw(encode decode compress decompress);

# _find_longest_match scans the search buffer for the longest match.
#
# @param $data_ref  (ref to array of ints) input bytes.
# @param $cursor    (int) current 0-indexed position.
# @param $win_size  (int) maximum lookback distance.
# @param $max_match (int) maximum match length.
# @return ($best_offset, $best_length) — both 0 if no match found.
sub _find_longest_match {
    my ($data_ref, $cursor, $win_size, $max_match) = @_;
    my @data     = @$data_ref;
    my $data_len = scalar @data;
    my $best_offset = 0;
    my $best_length = 0;

    # The search buffer starts at most win_size bytes back.
    my $search_start = $cursor - $win_size;
    $search_start = 0 if $search_start < 0;

    # Reserve 1 byte for next_char; lookahead stops 1 before end.
    my $lookahead_end = $cursor + $max_match;
    $lookahead_end = $data_len - 1 if $lookahead_end > $data_len - 1;

    for my $pos ($search_start .. $cursor - 1) {
        my $length = 0;
        # Match byte by byte. Matches may overlap (extend past cursor).
        while (($cursor + $length) < $lookahead_end
            && $data[$pos + $length] == $data[$cursor + $length])
        {
            $length++;
        }
        if ($length > $best_length) {
            $best_length = $length;
            $best_offset = $cursor - $pos;  # Distance back from cursor.
        }
    }

    return ($best_offset, $best_length);
}

=head1 FUNCTIONS

=head2 encode

  my @tokens = encode($data, $window_size, $max_match, $min_match);

Encodes a byte string into an LZ77 token stream.

Parameters:

=over 4

=item * $data        (string) input bytes.
=item * $window_size (int) maximum lookback distance (default 4096).
=item * $max_match   (int) maximum match length (default 255).
=item * $min_match   (int) minimum match length for backreference (default 3).

=back

Returns an array of hashrefs: C<{offset, length, next_char}>.

=cut

sub encode {
    my ($data, $window_size, $max_match, $min_match) = @_;
    $window_size //= 4096;
    $max_match   //= 255;
    $min_match   //= 3;

    my @bytes  = unpack 'C*', $data;
    my $data_len = scalar @bytes;
    my @tokens;
    my $cursor = 0;

    while ($cursor < $data_len) {
        # Edge case: last byte has no room for next_char after a match.
        if ($cursor == $data_len - 1) {
            push @tokens, { offset => 0, length => 0, next_char => $bytes[$cursor] };
            $cursor++;
            next;
        }

        my ($offset, $length) = _find_longest_match(\@bytes, $cursor, $window_size, $max_match);

        if ($length >= $min_match) {
            # Emit a backreference token.
            push @tokens, {
                offset    => $offset,
                length    => $length,
                next_char => $bytes[$cursor + $length],
            };
            $cursor += $length + 1;
        } else {
            # Emit a literal token.
            push @tokens, { offset => 0, length => 0, next_char => $bytes[$cursor] };
            $cursor++;
        }
    }

    return @tokens;
}

=head2 decode

  my $data = decode(\@tokens, $initial_buffer);

Decodes an LZ77 token stream back to a byte string.

Parameters:

=over 4

=item * $tokens_ref     (arrayref of hashrefs) token stream (output of encode).
=item * $initial_buffer (string) optional seed for search buffer (default '').

=back

=cut

sub decode {
    my ($tokens_ref, $initial_buffer) = @_;
    $initial_buffer //= '';

    my @output = unpack 'C*', $initial_buffer;

    for my $token (@$tokens_ref) {
        if ($token->{length} > 0) {
            # Copy length bytes from position (output_len - offset).
            my $start = scalar(@output) - $token->{offset};
            # Copy byte-by-byte to handle overlapping matches (offset < length).
            for my $i (0 .. $token->{length} - 1) {
                push @output, $output[$start + $i];
            }
        }
        # Always append next_char.
        push @output, $token->{next_char};
    }

    return pack 'C*', @output;
}

=head2 compress

  my $compressed = compress($data, $window_size, $max_match, $min_match);

One-shot API: encode then serialise to bytes.

=cut

sub compress {
    my ($data, $window_size, $max_match, $min_match) = @_;
    my @tokens = encode($data, $window_size, $max_match, $min_match);
    return _serialise_tokens(\@tokens);
}

=head2 decompress

  my $data = decompress($compressed);

One-shot API: deserialise then decode.

=cut

sub decompress {
    my ($compressed) = @_;
    my @tokens = _deserialise_tokens($compressed);
    return decode(\@tokens);
}

# _serialise_tokens serialises a token list to a binary string.
#
# Format:
#   4 bytes: token count (big-endian uint32)
#   N x 4 bytes: (offset: uint16 BE, length: uint8, next_char: uint8)
sub _serialise_tokens {
    my ($tokens_ref) = @_;
    my @tokens = @$tokens_ref;

    my $buf = pack 'N', scalar @tokens;
    for my $t (@tokens) {
        $buf .= pack 'nCC', $t->{offset}, $t->{length}, $t->{next_char};
    }
    return $buf;
}

# _deserialise_tokens deserialises bytes back into a token list.
sub _deserialise_tokens {
    my ($data) = @_;
    return () if length($data) < 4;

    my ($count) = unpack 'N', substr($data, 0, 4);
    my @tokens;

    for my $i (0 .. $count - 1) {
        my $base = 4 + $i * 4;
        last if $base + 4 > length($data);

        my ($offset, $length, $next_char) = unpack 'nCC', substr($data, $base, 4);
        push @tokens, { offset => $offset, length => $length, next_char => $next_char };
    }

    return @tokens;
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
