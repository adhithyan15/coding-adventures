package CodingAdventures::LZSS;

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::LZSS - LZSS lossless compression algorithm (1982)

=head1 SYNOPSIS

  use CodingAdventures::LZSS qw(encode decode compress decompress);

  my $data = "hello hello hello world";
  my $compressed = compress($data);
  my $original   = decompress($compressed);

=head1 DESCRIPTION

LZSS (Storer & Szymanski, 1982) refines LZ77 by eliminating the mandatory
C<next_char> byte appended after every token. Instead, a flag-bit scheme
distinguishes the two token kinds:

  Literal(byte)         => 1 byte  (flag bit = 0)
  Match(offset, length) => 3 bytes (flag bit = 1)

Tokens are grouped in blocks of 8. Each block begins with a 1-byte flag
(LSB = first token, bit 7 = eighth token).

=head2 Break-Even Point

A match token costs 3 bytes; three literals also cost 3 bytes. So
min_match = 3 is the minimum length that yields any saving (length >= 4
yields net gain). LZ77 per match is 4 bytes; LZSS is 3 bytes plus 1/8
flag byte amortised overhead.

=head2 Wire Format (CMP02)

  Bytes 0-3:  original_length  (big-endian uint32)
  Bytes 4-7:  block_count      (big-endian uint32)
  Bytes 8+:   blocks
    Each block:
      [1 byte]  flag -- bit i (LSB-first): 0 = literal, 1 = match
      [variable] up to 8 items:
                   flag=0: 1 byte  (literal value)
                   flag=1: 3 bytes (offset BE uint16 + length uint8)

=head2 The Series: CMP00 to CMP05

  CMP00 (LZ77, 1977)    -- Sliding-window backreferences.
  CMP01 (LZ78, 1978)    -- Explicit dictionary (trie), no sliding window.
  CMP02 (LZSS, 1982)    -- LZ77 + flag bits; eliminates wasted literals. This module.
  CMP03 (LZW,  1984)    -- Pre-initialized dictionary; powers GIF.
  CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

=cut

use Exporter 'import';
our @EXPORT_OK = qw(
    encode decode compress decompress
    make_literal make_match
);

# -- Token constructors -------------------------------------------------------

=head2 make_literal

  my $tok = make_literal($byte);

Creates a Literal token: C<{kind => 'literal', byte => $byte}>.

=cut

sub make_literal {
    my ($byte) = @_;
    return { kind => 'literal', byte => $byte };
}

=head2 make_match

  my $tok = make_match($offset, $length);

Creates a Match token: C<{kind => 'match', offset => $offset, length => $length}>.

=cut

sub make_match {
    my ($offset, $length) = @_;
    return { kind => 'match', offset => $offset, length => $length };
}

# -- Default parameters -------------------------------------------------------

use constant DEFAULT_WINDOW_SIZE => 4096;
use constant DEFAULT_MAX_MATCH   => 255;
use constant DEFAULT_MIN_MATCH   => 3;

# -- Encoder ------------------------------------------------------------------

# _find_longest_match scans data[$search_start..$cursor-1] for the best match.
#
# LZSS does NOT reserve 1 byte for next_char -- lookahead extends to EOF.
#
# @param $data_ref  (ref to array of ints) input bytes.
# @param $cursor    (int) current 0-indexed position.
# @param $win_size  (int) maximum lookback distance.
# @param $max_match (int) maximum match length.
# @return ($best_offset, $best_length) -- both 0 if no match found.
sub _find_longest_match {
    my ($data_ref, $cursor, $win_size, $max_match) = @_;
    my @data     = @$data_ref;
    my $data_len = scalar @data;
    my $best_offset = 0;
    my $best_length = 0;

    my $search_start = $cursor - $win_size;
    $search_start = 0 if $search_start < 0;

    # LZSS: no next_char reservation; lookahead goes to end of data.
    my $lookahead_end = $cursor + $max_match;
    $lookahead_end = $data_len if $lookahead_end > $data_len;

    for my $pos ($search_start .. $cursor - 1) {
        my $length = 0;
        while (($cursor + $length) < $lookahead_end
            && $data[$pos + $length] == $data[$cursor + $length])
        {
            $length++;
        }
        if ($length > $best_length) {
            $best_length = $length;
            $best_offset = $cursor - $pos;
        }
    }

    return ($best_offset, $best_length);
}

=head2 encode

  my @tokens = encode($data, $window_size, $max_match, $min_match);

Encodes a byte string into an LZSS token stream.

Emits C<make_literal> tokens for single bytes and C<make_match> tokens for
back-references. Unlike LZ77, there is no trailing C<next_char> -- the cursor
advances by exactly C<$length> positions (not C<$length + 1>).

Parameters:

=over 4

=item * $data        (string) input bytes.
=item * $window_size (int) maximum lookback distance (default 4096).
=item * $max_match   (int) maximum match length (default 255).
=item * $min_match   (int) minimum match length for a Match token (default 3).

=back

Returns an array of hashrefs: C<{kind, byte}> or C<{kind, offset, length}>.

=cut

sub encode {
    my ($data, $window_size, $max_match, $min_match) = @_;
    $window_size //= DEFAULT_WINDOW_SIZE;
    $max_match   //= DEFAULT_MAX_MATCH;
    $min_match   //= DEFAULT_MIN_MATCH;

    my @bytes    = unpack 'C*', $data;
    my $data_len = scalar @bytes;
    my @tokens;
    my $cursor = 0;

    while ($cursor < $data_len) {
        my ($offset, $length) = _find_longest_match(\@bytes, $cursor, $window_size, $max_match);

        if ($length >= $min_match) {
            push @tokens, make_match($offset, $length);
            $cursor += $length;
        } else {
            push @tokens, make_literal($bytes[$cursor]);
            $cursor++;
        }
    }

    return @tokens;
}

=head2 decode

  my $data = decode(\@tokens, $original_length);

Decodes an LZSS token stream back to a byte string.

For Match tokens, bytes are copied one at a time from C<$offset> positions
back in the output. Byte-by-byte copy is essential for overlapping matches
(e.g., Match(1, 6) on [65] produces six copies of 65).

Parameters:

=over 4

=item * $tokens_ref      (arrayref of hashrefs) token stream from encode().
=item * $original_length (int) optional; truncates output to this length.

=back

=cut

sub decode {
    my ($tokens_ref, $original_length) = @_;

    my @output;

    for my $tok (@$tokens_ref) {
        if ($tok->{kind} eq 'literal') {
            push @output, $tok->{byte};
        } else {
            # Match: copy $tok->{length} bytes from $offset positions back.
            # Guard against malformed tokens: offset=0 or offset > output length
            # would give a negative $start or read from the wrong position.
            my $off   = $tok->{offset};
            my $start = scalar(@output) - $off;
            if ($off < 1 || $start < 0) {
                next;  # skip invalid match token
            }
            for my $i (0 .. $tok->{length} - 1) {
                push @output, $output[$start + $i];
            }
        }

        # Honour original_length to trim block padding.
        if (defined $original_length && scalar @output >= $original_length) {
            last;
        }
    }

    if (defined $original_length && scalar @output > $original_length) {
        @output = @output[0 .. $original_length - 1];
    }

    return pack 'C*', @output;
}

=head2 compress

  my $compressed = compress($data, $window_size, $max_match, $min_match);

One-shot API: encode then serialise to CMP02 wire format.

=cut

sub compress {
    my ($data, $window_size, $max_match, $min_match) = @_;
    my @tokens = encode($data, $window_size, $max_match, $min_match);
    return _serialise_tokens(\@tokens, length($data));
}

=head2 decompress

  my $data = decompress($compressed);

One-shot API: deserialise then decode.

=cut

sub decompress {
    my ($compressed) = @_;
    my ($tokens_ref, $original_length) = _deserialise_tokens($compressed);
    return decode($tokens_ref, $original_length);
}

# -- Serialisation ------------------------------------------------------------

# _serialise_tokens serialises a token list to the CMP02 wire format.
#
# Groups up to 8 tokens per block. Each block starts with a 1-byte flag
# (bit i = 0 for Literal, 1 for Match). Literal uses 1 byte; Match uses
# 3 bytes (offset BE uint16 + length uint8).
#
# @param $tokens_ref      (arrayref of hashrefs) token list.
# @param $original_length (int) length of the original input.
# @return (string) binary CMP02 data.
sub _serialise_tokens {
    my ($tokens_ref, $original_length) = @_;
    $original_length //= 0;
    my @tokens = @$tokens_ref;

    my @blocks;
    my $i = 0;

    while ($i < scalar @tokens) {
        my $chunk_end = $i + 7;
        $chunk_end = $#tokens if $chunk_end > $#tokens;

        my $flag   = 0;
        my $symbol = '';

        for my $bit (0 .. $chunk_end - $i) {
            my $tok = $tokens[$i + $bit];
            if ($tok->{kind} eq 'match') {
                $flag |= (1 << $bit);
                $symbol .= pack 'nC', $tok->{offset}, $tok->{length};
            } else {
                $symbol .= pack 'C', $tok->{byte};
            }
        }

        push @blocks, pack('C', $flag) . $symbol;
        $i = $chunk_end + 1;
    }

    my $header = pack 'NN', $original_length, scalar @blocks;
    return $header . join('', @blocks);
}

# _deserialise_tokens deserialises CMP02 bytes to a token list.
#
# Security: caps block_count against the actual payload size to prevent a
# crafted header from causing unbounded iteration on minimal input.
#
# @param $data (string) binary CMP02 data.
# @return ($tokens_ref, $original_length)
sub _deserialise_tokens {
    my ($data) = @_;
    return ([], 0) if length($data) < 8;

    my ($original_length, $block_count) = unpack 'NN', substr($data, 0, 8);

    # Cap block_count against remaining payload to prevent DoS.
    my $max_possible = length($data) - 8;
    $block_count = $max_possible if $block_count > $max_possible;

    my @tokens;
    my $pos = 8;

    for my $b (1 .. $block_count) {
        last if $pos >= length($data);

        my $flag = unpack 'C', substr($data, $pos, 1);
        $pos++;

        for my $bit (0 .. 7) {
            last if $pos >= length($data);

            if ($flag & (1 << $bit)) {
                # Match: 3 bytes
                last if $pos + 2 >= length($data);
                my ($offset, $length) = unpack 'nC', substr($data, $pos, 3);
                push @tokens, make_match($offset, $length);
                $pos += 3;
            } else {
                # Literal: 1 byte
                my ($byte) = unpack 'C', substr($data, $pos, 1);
                push @tokens, make_literal($byte);
                $pos++;
            }
        }
    }

    return (\@tokens, $original_length);
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
