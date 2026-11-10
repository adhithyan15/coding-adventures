package CodingAdventures::HuffmanCompression;

use strict;
use warnings;
use CodingAdventures::HuffmanTree;

our $VERSION = '0.1.1';

=head1 NAME

CodingAdventures::HuffmanCompression - CMP04 Huffman entropy compression

=head1 SYNOPSIS

  use CodingAdventures::HuffmanCompression qw(compress decompress);

  my $data       = "AAABBC";
  my $compressed = compress($data);
  my $original   = decompress($compressed);  # "AAABBC"

=head1 DESCRIPTION

Huffman coding (1952) is a B<entropy compression> algorithm: it assigns shorter
bit sequences to symbols that appear more often and longer bit sequences to
symbols that appear less often. Unlike dictionary methods (LZ77, LZW), Huffman
works at the B<symbol level> — it doesn't find repeated strings, only counts
symbol frequencies and optimises bit allocations accordingly.

Think of it as a postal-code optimisation. If you live in a busy city (high
frequency), you get a short zipcode. If you live in a remote village (low
frequency), your address is longer. The total postal work is minimised.

=head2 Canonical Codes (DEFLATE-style)

The standard (code-table-in-the-bitstream) approach embeds the full tree.
Canonical Huffman only needs the I<code lengths> — the actual bit codes can be
reconstructed by the decoder using a simple deterministic algorithm:

  1. Sort symbols by (code_length, symbol_value) ascending.
  2. First code = 0.
  3. Increment code each step; shift left when code_length increases.

This is exactly how DEFLATE/gzip/PNG encode their Huffman tables.

=head2 CMP04 Wire Format

  Bytes 0–3:    original_length  (big-endian uint32)
  Bytes 4–7:    symbol_count     (big-endian uint32)  — distinct byte count
  Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
                  [0] symbol value  (uint8, 0–255)
                  [1] code length   (uint8, 1–16)
                Sorted by (code_length, symbol_value) ascending.
  Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.

=head2 Algorithm Overview: compress

  1. Count byte frequencies in the input.
  2. Build a Huffman tree from the (symbol, freq) pairs.
  3. Compute canonical codes (lengths only matter).
  4. Emit the wire-format header: original_length, symbol_count, sorted lengths.
  5. Encode each input byte to its canonical bit string.
  6. Pack the concatenated bit string LSB-first into bytes.

=head2 Algorithm Overview: decompress

  1. Parse original_length and symbol_count from the 8-byte header.
  2. Read the code-lengths table (symbol_count pairs of 2 bytes).
  3. Reconstruct canonical codes from the sorted (sym, len) pairs.
  4. Unpack the bit stream from bytes back to a bit string.
  5. Decode exactly original_length symbols using the canonical code table.

=head2 In the Series

  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie), no window.
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialized dict; powers GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE. (this module)
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

=head2 Dependency

This module requires L<CodingAdventures::HuffmanTree> (DT27) for tree
construction and canonical code generation. Ensure C<PERL5LIB> includes
C<../huffman-tree/lib> when running tests.

=cut

use Exporter 'import';
our @EXPORT_OK = qw(compress decompress);

# ---------------------------------------------------------------------------
# Internal: bit helpers
# ---------------------------------------------------------------------------

=head1 INTERNAL: BIT HELPERS

=head2 _pack_bits_lsb_first

  my $bytes = _pack_bits_lsb_first($bits);

Takes a string of C<'0'> and C<'1'> characters (a concatenated bit sequence)
and packs them into bytes, least-significant bit first.

LSB-first means bit[0] goes into position 0 of byte 0, bit[1] into position 1,
bit[7] into position 7 (the MSB), bit[8] into position 0 of byte 1, etc.

Example:

  bit string: "10110000"
  packed byte: bit[0]='1' → 0b00000001
               bit[1]='0' → no change
               bit[2]='1' → 0b00000101
               bit[3]='1' → 0b00001101
               → 0x0D = 13

The bit stream is zero-padded to the next byte boundary.

=cut

sub _pack_bits_lsb_first {
    my ($bits) = @_;
    my @output;
    my ($buffer, $bit_pos) = (0, 0);
    for my $b (split //, $bits) {
        $buffer |= $b << $bit_pos;
        $bit_pos++;
        if ($bit_pos == 8) {
            push @output, $buffer;
            $buffer = $bit_pos = 0;
        }
    }
    # Flush any remaining bits as a partial byte (zero-padded to MSB).
    push @output, $buffer if $bit_pos > 0;
    return pack("C*", @output);
}

# ---------------------------------------------------------------------------
# Internal: canonical code reconstruction
# ---------------------------------------------------------------------------

=head1 INTERNAL: CANONICAL CODE RECONSTRUCTION

=head2 _canonical_codes_from_lengths

  my ($code_to_sym, $max_len) = _canonical_codes_from_lengths(@lengths);

Reconstructs the canonical Huffman code table from a sorted list of
C<[$symbol, $code_length]> pairs and B<validates> it, because the table comes
straight from untrusted input.

The input must be sorted by C<(code_length, symbol_value)> ascending — the
same order used by C<compress> when writing the header table.

The algorithm:

  1. Start with code = 0.
  2. For each (symbol, len) pair in order:
     a. If len > prev_len, shift code left by (len - prev_len).
     b. Check the code still fits in len bits (Kraft sum <= 1).
     c. Map (len, code) -> symbol in the output hash.
     d. Increment code.
     e. Update prev_len.

Dies when a length is 0 or above 56, when the table is not sorted by length,
or when the table over-subscribes the code space.

Returns a hashref keyed by C<"$len:$code"> (no per-symbol bit strings are
built) and the longest code length.

=cut

# Longest code length the decoder accepts.  Real codes over 256 symbols are far
# shorter; 56 keeps every shift well inside a 64-bit integer.
use constant MAX_CODE_LEN => 56;

# Symbols are single bytes, so a valid table has at most 256 entries.
use constant MAX_SYMBOLS => 256;

sub _canonical_codes_from_lengths {
    my (@lengths) = @_;   # list of [$sym, $len] sorted by ($len, $sym)
    my %code_to_sym;
    my $code     = 0;
    my $prev_len = @lengths ? $lengths[0][1] : 0;
    my $max_len  = 0;
    for my $entry (@lengths) {
        my ($sym, $len) = @$entry;

        # SECURITY: a zero-length code matches nothing; a huge length would
        # push the shift past the width of an integer.
        die "invalid code length $len in table\n"
            if $len == 0 || $len > MAX_CODE_LEN;

        # The encoder always sorts by length; an unsorted table is hostile.
        die "code-lengths table is not sorted by length\n" if $len < $prev_len;

        # When code length increases, shift left to maintain prefix-free property.
        $code <<= ($len - $prev_len);

        # SECURITY: over-subscribed table (Kraft sum > 1).  The running code
        # must still fit in $len bits; if it does not, the code space is
        # exhausted and no prefix-free assignment exists.
        die "code-lengths table over-subscribes the code space\n"
            if ($code >> $len) != 0;

        $code_to_sym{"$len:$code"} = $sym;
        $code++;
        $prev_len = $len;
        $max_len  = $len;
    }
    return (\%code_to_sym, $max_len);
}

# ---------------------------------------------------------------------------
# Public API: compress
# ---------------------------------------------------------------------------

=head1 PUBLIC API

=head2 compress

  my $compressed = compress($data);

Compresses C<$data> (a byte string) using canonical Huffman coding and
returns a binary string in CMP04 wire format.

Steps:

=over 4

=item 1. Count byte frequencies with C<unpack("C*", $data)>.

=item 2. If the input is empty, return an 8-byte header with all zeros.

=item 3. Build a Huffman tree: C<CodingAdventures::HuffmanTree-E<gt>build(...)>.

=item 4. Fetch canonical codes: C<$tree-E<gt>canonical_code_table()>.

=item 5. Sort symbols by C<(code_length, symbol_value)> to produce the
         code-lengths table.

=item 6. Concatenate bit strings for each input byte.

=item 7. Pack the bit stream LSB-first using C<_pack_bits_lsb_first>.

=item 8. Assemble wire format:
         C<pack("NN", $orig_len, $sym_count)> +
         code-lengths pairs +
         packed bit stream.

=back

=cut

sub compress {
    my ($data) = @_;

    # Step 1: count byte frequencies.
    # unpack("C*", $data) expands the string into a list of unsigned byte values.
    # We tally each byte value into %freq.
    my %freq;
    $freq{$_}++ for unpack("C*", $data);

    # Step 2: handle empty input.
    # No symbols → 8-byte header (orig_len=0, sym_count=0) with no table or bits.
    if (!%freq) {
        return pack("NN", 0, 0);
    }

    # Step 3: build the Huffman tree.
    # Convert %freq to the [[sym, freq], ...] format expected by HuffmanTree.
    my @weights = map { [$_, $freq{$_}] } sort { $a <=> $b } keys %freq;
    my $tree    = CodingAdventures::HuffmanTree->build(\@weights);

    # Step 4: get canonical code table: hashref { sym => bit_string }.
    my $table = $tree->canonical_code_table();

    # Step 5: sort by (code_length, symbol_value) ascending.
    # This produces the canonical sorted order required for deterministic
    # reconstruction during decompression.
    my @sorted_lengths = sort {
        length($table->{$a}) <=> length($table->{$b}) || $a <=> $b
    } keys %$table;

    # Step 6: encode each byte of $data to its canonical bit string.
    # Each byte's code is looked up and appended to the growing bit string.
    my $bits = "";
    for my $byte (unpack("C*", $data)) {
        $bits .= $table->{$byte};
    }

    # Step 7: pack the bit stream LSB-first.
    my $bitstream = _pack_bits_lsb_first($bits);

    # Step 8: assemble the wire format.
    # Header: original_length (4 bytes), symbol_count (4 bytes).
    my $orig_len  = length($data);
    my $sym_count = scalar @sorted_lengths;
    my $header    = pack("NN", $orig_len, $sym_count);

    # Code-lengths table: each entry is 2 bytes: [symbol, code_length].
    my $lengths_table = "";
    for my $sym (@sorted_lengths) {
        $lengths_table .= pack("CC", $sym, length($table->{$sym}));
    }

    return $header . $lengths_table . $bitstream;
}

# ---------------------------------------------------------------------------
# Public API: decompress
# ---------------------------------------------------------------------------

=head2 decompress

  my $data = decompress($compressed);

Decompresses a CMP04 wire-format byte string and returns the original data.

Steps:

=over 4

=item 1. Parse C<original_length> and C<symbol_count> from bytes 0–7.

=item 2. If C<symbol_count == 0>, return C<"">.

=item 3. Parse the code-lengths table: C<symbol_count> pairs of 2 bytes each
         starting at byte 8.

=item 4. Reconstruct canonical codes: C<_canonical_codes_from_lengths(@table)>.

=item 5. Extract the bit stream: bytes C<8 + 2*symbol_count> onward.

=item 6. Unpack the bit stream from bytes to a bit string using
         C<_unpack_bits_lsb_first>.

=item 7. Decode exactly C<original_length> symbols: walk through the bit
         string matching prefixes against the C<%code_to_sym> table.

=back

The decoder greedily consumes the minimum bits needed to match a code at each
step — this is safe because canonical codes are prefix-free.

=cut

sub decompress {
    my ($data) = @_;

    # Step 1: parse 8-byte header.
    return "" if length($data) < 8;
    my ($orig_len, $sym_count) = unpack("NN", substr($data, 0, 8));

    # Step 2: empty input short-circuit.
    return "" if $sym_count == 0 || $orig_len == 0;

    # SECURITY: the header is attacker-controlled.  Symbols are single bytes,
    # so a table can name at most 256 of them.  Check before computing the
    # table size so a huge count cannot drive the arithmetic below.
    die "symbol_count $sym_count exceeds " . MAX_SYMBOLS . "\n"
        if $sym_count > MAX_SYMBOLS;

    # Step 3: parse code-lengths table.
    # Each entry: 2 bytes (symbol byte, code_length byte).
    # The table starts at offset 8 and is 2*sym_count bytes long.
    my $table_offset = 8;
    my $table_size   = $sym_count * 2;
    return "" if length($data) < $table_offset + $table_size;

    my @lengths;
    my %seen;
    for my $i (0 .. $sym_count - 1) {
        my ($sym, $len) = unpack("CC", substr($data, $table_offset + $i * 2, 2));
        # A byte value can have only one code; a repeat makes the table ambiguous.
        die "symbol $sym appears twice in code-lengths table\n" if $seen{$sym}++;
        push @lengths, [$sym, $len];
    }

    # Step 4: reconstruct (and validate) canonical codes.
    # _canonical_codes_from_lengths expects [@lengths] sorted by (len, sym),
    # which is exactly the order compress wrote them.
    my ($code_to_sym, $max_len) = _canonical_codes_from_lengths(@lengths);

    # Step 5: bound the declared length by the payload.
    # SECURITY: every code is at least one bit, so N payload bytes hold at most
    # 8*N symbols.  Reject a larger declared length before doing any work sized
    # from it.
    my $bitstream_offset = $table_offset + $table_size;
    my $bitstream_bytes  = substr($data, $bitstream_offset);
    my $total_bits       = length($bitstream_bytes) * 8;
    die "declared length $orig_len exceeds the $total_bits bits available\n"
        if $orig_len > $total_bits;

    # Step 6: decode straight from the payload bytes.
    # vec($bytes, $i, 1) reads bit $i with bit 0 being the LEAST significant
    # bit of the first byte -- exactly the LSB-first order the encoder wrote --
    # in O(1), so we never expand the payload into a huge '0'/'1' string (or a
    # 56x-larger list via unpack "C*").  The candidate code is built MSB-first
    # as an integer.  Canonical codes are prefix-free, so the first
    # "$len:$code" hit is the symbol.  Once the accumulated length reaches the
    # longest code without a match, no later bit can match (an incomplete
    # table fed unmatched bits), so we die immediately.
    my @output;
    my $pos     = 0;
    my $acc     = 0;
    my $acc_len = 0;
    while (@output < $orig_len) {
        die "bit stream exhausted after " . scalar(@output) . " symbols\n"
            if $pos >= $total_bits;

        $acc = ($acc << 1) | vec($bitstream_bytes, $pos, 1);
        $pos++;
        $acc_len++;

        my $key = "$acc_len:$acc";
        if (exists $code_to_sym->{$key}) {
            push @output, $code_to_sym->{$key};
            $acc     = 0;
            $acc_len = 0;
        }
        elsif ($acc_len >= $max_len) {
            die "bit pattern does not match any code\n";
        }
    }

    return pack("C*", @output);
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
