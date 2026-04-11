package CodingAdventures::LZ78;

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::LZ78 - LZ78 lossless compression algorithm (1978)

=head1 SYNOPSIS

  use CodingAdventures::LZ78 qw(encode decode compress decompress);

  my $data = "hello hello hello";
  my $compressed = compress($data);
  my $original   = decompress($compressed);
  # $original eq "hello hello hello"

=head1 DESCRIPTION

LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary of byte
sequences as it encodes. Both encoder and decoder build the same dictionary
independently — no dictionary is transmitted on the wire.

=head2 The Dictionary vs. The Sliding Window

LZ77 (CMP00) uses a I<sliding window>: it forgets bytes that fall off the back
of the window. LZ78 grows a I<global dictionary> that never forgets — making
it better for repetitive data spread throughout a file.

=head2 Token

Each token is a C<{dict_index, next_char}> pair:

=over 4

=item * C<dict_index> — ID of the longest dictionary prefix matched (0 = literal)

=item * C<next_char>  — The byte following the match (0 = flush sentinel at end)

=back

=head2 Wire Format (CMP01)

  Bytes 0–3:  original length (big-endian uint32)
  Bytes 4–7:  token count    (big-endian uint32)
  Bytes 8+:   N × 4 bytes each:
                [0..1]  dict_index (big-endian uint16)
                [2]     next_char  (uint8)
                [3]     reserved   (0x00)

=head2 Series

  CMP00 (LZ77,    1977) — Sliding-window backreferences.
  CMP01 (LZ78,    1978) — Explicit dictionary (trie). ← this module
  CMP02 (LZSS,    1982) — LZ77 + flag bits.
  CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman, 1952) — Entropy coding.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG.

=cut

use Exporter 'import';
our @EXPORT_OK = qw(
    encode decode compress decompress serialise_tokens deserialise_tokens
    new_cursor cursor_step cursor_insert cursor_reset cursor_dict_id cursor_at_root cursor_entries
);

# ─── TrieCursor ────────────────────────────────────────────────────────────────
#
# A step-by-step cursor for navigating a byte-keyed trie.
#
# Unlike a full trie API (which operates on complete keys), TrieCursor
# maintains a current position and advances one byte at a time. This is
# the core abstraction for streaming dictionary algorithms:
#
#   LZ78 (CMP01): step($cursor, $byte) → emit token on miss, insert new entry
#   LZW  (CMP03): same pattern with a pre-seeded 256-entry alphabet
#
# Trie storage
# ------------
#
# The trie is an arena: an arrayref of nodes, each a hashref:
#   { dict_id => $id, children => { $byte => $node_idx } }
# Node 0 = root. Indices are 0-based (consistent with dict_ids).
#
# TrieCursor is represented as a hashref:
#   { arena => [...], current => $idx }
#

=head2 TrieCursor

A step-by-step cursor for navigating a byte-keyed trie. Used by encode/decode
for streaming dictionary management. Can be reused for LZW (CMP03).

=head3 new_cursor

  my $cursor = new_cursor();

Create a new cursor with an empty trie (root only). Returns a hashref.

=cut

sub new_cursor {
    return {
        arena   => [ { dict_id => 0, children => {} } ],  # node 0 = root
        current => 0,
    };
}

=head3 cursor_step

  my $hit = cursor_step($cursor, $byte);

Try to follow the child edge for C<$byte> from the current position.
Returns 1 and advances if the child exists; returns 0 otherwise (cursor
stays at current position).

=cut

sub cursor_step {
    my ($cursor, $byte) = @_;
    my $child_idx = $cursor->{arena}[$cursor->{current}]{children}{$byte};
    if (defined $child_idx) {
        $cursor->{current} = $child_idx;
        return 1;
    }
    return 0;
}

=head3 cursor_insert

  cursor_insert($cursor, $byte, $dict_id);

Add a child edge for C<$byte> at the current position with C<$dict_id>.
Does not advance the cursor — call C<cursor_reset> to return to root.

=cut

sub cursor_insert {
    my ($cursor, $byte, $dict_id) = @_;
    my $new_idx = scalar @{$cursor->{arena}};
    push @{$cursor->{arena}}, { dict_id => $dict_id, children => {} };
    $cursor->{arena}[$cursor->{current}]{children}{$byte} = $new_idx;
}

=head3 cursor_reset

  cursor_reset($cursor);

Return the cursor to the trie root.

=cut

sub cursor_reset {
    my ($cursor) = @_;
    $cursor->{current} = 0;
}

=head3 cursor_dict_id

  my $id = cursor_dict_id($cursor);

Dictionary ID at the current cursor position (0 when at root).

=cut

sub cursor_dict_id {
    my ($cursor) = @_;
    return $cursor->{arena}[$cursor->{current}]{dict_id};
}

=head3 cursor_at_root

  my $bool = cursor_at_root($cursor);

Returns true if the cursor is at the root node.

=cut

sub cursor_at_root {
    my ($cursor) = @_;
    return $cursor->{current} == 0;
}

=head3 cursor_entries

  my @entries = cursor_entries($cursor);

Return all C<([@path], $dict_id)> pairs in the trie (DFS, sorted by dict_id).

=cut

sub cursor_entries {
    my ($cursor) = @_;
    my @results;
    my $dfs;
    $dfs = sub {
        my ($node_idx, $path) = @_;
        my $node = $cursor->{arena}[$node_idx];
        if ($node->{dict_id} > 0) {
            push @results, [ [@$path], $node->{dict_id} ];
        }
        for my $byte (sort keys %{$node->{children}}) {
            push @$path, $byte;
            $dfs->($node->{children}{$byte}, $path);
            pop @$path;
        }
    };
    $dfs->(0, []);
    return sort { $a->[1] <=> $b->[1] } @results;
}

# ─── Encoder ──────────────────────────────────────────────────────────────────

=head2 encode

  my @tokens = encode($data, $max_dict_size);

Encode a binary string into an LZ78 token array.

Uses C<TrieCursor> to walk the dictionary one byte at a time.
When C<cursor_step> returns false, emits a token for the current dict_id
plus byte, records the new sequence, and resets to root.

If input ends mid-match, a flush token with C<next_char=0> is emitted.

=head3 Parameters

=over 4

=item C<$data>         — Binary string (use C<:raw> or C<:bytes>).

=item C<$max_dict_size> — Maximum dictionary entries (default 65536).

=back

=head3 Returns

List of C<{dict_index => $n, next_char => $n}> hashrefs.

=head3 Example

  my @tokens = encode("ABCDE");
  # All 5 tokens have dict_index => 0 (all literals)

=cut

sub encode {
    my ($data, $max_dict_size) = @_;
    $max_dict_size //= 65536;

    my $cursor  = new_cursor();
    my $next_id = 1;
    my @tokens;

    for my $byte (unpack 'C*', $data) {
        if (!cursor_step($cursor, $byte)) {
            push @tokens, { dict_index => cursor_dict_id($cursor), next_char => $byte };
            if ($next_id < $max_dict_size) {
                cursor_insert($cursor, $byte, $next_id);
                $next_id++;
            }
            cursor_reset($cursor);
        }
    }

    # Flush partial match at end of stream.
    unless (cursor_at_root($cursor)) {
        push @tokens, { dict_index => cursor_dict_id($cursor), next_char => 0 };
    }

    return @tokens;
}

# ─── Decoder ──────────────────────────────────────────────────────────────────

# Walk the parent chain to reconstruct a dictionary entry.
# Returns a listref of bytes in correct forward order.
sub _reconstruct {
    my ($dict_table, $index) = @_;
    return [] if $index == 0;
    my @rev;
    my $idx = $index;
    while ($idx != 0) {
        my $entry = $dict_table->[$idx];
        push @rev, $entry->[1];   # byte
        $idx = $entry->[0];       # parent_id
    }
    return [ reverse @rev ];
}

=head2 decode

  my $data = decode(\@tokens, $original_length);

Decode an LZ78 token list back into the original bytes.

Mirrors C<encode>: maintains a parallel dictionary as an arrayref of
C<[$parent_id, $byte]> pairs. For each token, reconstructs the sequence
for C<dict_index>, emits it, emits C<next_char>, then adds a new entry.

=head3 Parameters

=over 4

=item C<\@tokens>        — Token list from C<encode>.

=item C<$original_length> — If defined, truncates output to that many bytes
(strips flush sentinel). Pass C<undef> to return all bytes.

=back

=head3 Returns

Binary string.

=head3 Example

  my @tokens = encode("hello");
  my $s      = decode(\@tokens, 5);
  # $s eq "hello"

=cut

sub decode {
    my ($tokens, $original_length) = @_;
    # dict_table->[0] = [0, 0] — root sentinel (index 0)
    my @dict_table = ([0, 0]);
    my @out;

    for my $tok (@$tokens) {
        my $seq = _reconstruct(\@dict_table, $tok->{dict_index});
        push @out, @$seq;

        if (!defined $original_length || scalar @out < $original_length) {
            push @out, $tok->{next_char};
        }

        push @dict_table, [$tok->{dict_index}, $tok->{next_char}];
    }

    if (defined $original_length && scalar @out > $original_length) {
        @out = @out[0 .. $original_length - 1];
    }

    return pack 'C*', @out;
}

# ─── Serialisation ────────────────────────────────────────────────────────────

=head2 serialise_tokens

  my $bytes = serialise_tokens(\@tokens, $original_length);

Serialise tokens to the CMP01 wire format.

=cut

sub serialise_tokens {
    my ($tokens, $original_length) = @_;
    my $buf = pack 'N', $original_length;
    $buf   .= pack 'N', scalar @$tokens;
    for my $tok (@$tokens) {
        $buf .= pack 'n', $tok->{dict_index};
        $buf .= pack 'C', $tok->{next_char};
        $buf .= "\x00";
    }
    return $buf;
}

=head2 deserialise_tokens

  my ($tokens_ref, $original_length) = deserialise_tokens($bytes);

Deserialise CMP01 wire-format bytes back into tokens and original length.

=cut

sub deserialise_tokens {
    my ($data) = @_;
    return ([], 0) if length($data) < 8;

    my $original_length = unpack 'N', substr($data, 0, 4);
    my $token_count     = unpack 'N', substr($data, 4, 4);
    my @tokens;

    for my $i (0 .. $token_count - 1) {
        my $base = 8 + $i * 4;
        last if $base + 4 > length($data);
        my $dict_index = unpack 'n', substr($data, $base,     2);
        my $next_char  = unpack 'C', substr($data, $base + 2, 1);
        push @tokens, { dict_index => $dict_index, next_char => $next_char };
    }

    return (\@tokens, $original_length);
}

# ─── One-shot API ─────────────────────────────────────────────────────────────

=head2 compress

  my $bytes = compress($data, $max_dict_size);

Compress a binary string using LZ78, returning the CMP01 wire format.

=head3 Example

  my $c = compress("AAAAAAA");
  decompress($c) eq "AAAAAAA"  # true

=cut

sub compress {
    my ($data, $max_dict_size) = @_;
    $max_dict_size //= 65536;
    my @tokens = encode($data, $max_dict_size);
    return serialise_tokens(\@tokens, length($data));
}

=head2 decompress

  my $data = decompress($bytes);

Decompress bytes that were compressed with C<compress>.

=head3 Example

  my $original = "hello hello hello";
  decompress(compress($original)) eq $original  # true

=cut

sub decompress {
    my ($data) = @_;
    my ($tokens, $original_length) = deserialise_tokens($data);
    return decode($tokens, $original_length);
}

1;
__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT
