package CodingAdventures::Brotli;

use strict;
use warnings;
use CodingAdventures::HuffmanTree;

our $VERSION = '0.1.0';
use Exporter 'import';
our @EXPORT_OK = qw(compress decompress);

=head1 NAME

CodingAdventures::Brotli - CMP06 Brotli-style compression/decompression

=head1 SYNOPSIS

  use CodingAdventures::Brotli qw(compress decompress);

  my $data       = "hello hello hello world";
  my $compressed = compress($data);
  my $original   = decompress($compressed);  # "hello hello hello world"

=head1 DESCRIPTION

Brotli (2013, RFC 7932) is a lossless compression algorithm developed at
Google. This CMP06 implementation captures Brotli's three key innovations
while omitting the 122,784-entry static dictionary to keep the implementation
tractable and consistent across all nine language targets.

=head2 Innovation 1: Context-Dependent Literal Trees

Instead of one Huffman tree for all literal bytes (as in DEFLATE), CMP06 uses
B<four separate literal trees>, one for each I<context bucket>. The bucket is
chosen by examining the last byte emitted:

  bucket 0 — last byte was space/punctuation (catch-all, also used at start)
  bucket 1 — last byte was a digit ('0'–'9')
  bucket 2 — last byte was uppercase ('A'–'Z')
  bucket 3 — last byte was lowercase ('a'–'z')

Why does this help? In English text, the letter following a space is very
different from the letter following another letter. If you write C<"the ">,
the next byte after the space is almost always a new word starting with a
consonant. If you write C<"qu">, the next byte is almost always C<'i'> or
C<'a'>. By using separate trees per context, each tree can be finely tuned to
its own letter distribution, giving shorter average codes.

=head2 Innovation 2: Insert-and-Copy Commands

DEFLATE interleaves individual literal tokens and back-reference tokens. Brotli
bundles them into B<commands>:

  Command {
    insert_length  — how many raw literals follow
    copy_length    — how many bytes to copy from the history window
    copy_distance  — how far back to look (1 = last byte)
  }

Insert length and copy length are encoded together as a single B<insert-copy
code (ICC)> Huffman symbol. This reduces overhead compared to emitting two
separate Huffman symbols.

=head2 Innovation 3: 65535-Byte Sliding Window

CMP05 (DEFLATE) used a 4096-byte window. CMP06 extends this to B<65535 bytes>,
letting the LZ matcher reference repetitions thousands of bytes apart.

=head2 Trailing Literals

The ICC table has no code for "insert only with no copy". When the LZ pass
finishes and there are remaining literal bytes that could not be attached to a
copy command, the encoder collects them as B<flush literals> and emits them
I<after> the sentinel ICC code 63 in the bit stream.

  [... last real command ...] [ICC=63] [flush literal bytes, if any]

The decompressor, after exiting the command loop on the sentinel, reads
C<original_length - len(output)> more literal bytes using the same
context-bucket logic. This avoids any synthetic copy pollution.

=head2 Wire Format (CMP06)

  Header (10 bytes):
    [4B] original_length      big-endian uint32
    [1B] icc_entry_count      entries in ICC code-length table (1–64)
    [1B] dist_entry_count     entries in dist code-length table (0–32)
    [1B] ctx0_entry_count     entries in literal tree 0 (space/punct)
    [1B] ctx1_entry_count     entries in literal tree 1 (digit)
    [1B] ctx2_entry_count     entries in literal tree 2 (uppercase)
    [1B] ctx3_entry_count     entries in literal tree 3 (lowercase)

  ICC code-length table (icc_entry_count × 2 bytes):
    [1B] symbol (0–63)
    [1B] code_length (1–16)
    Sorted: (code_length ASC, symbol ASC)

  Dist code-length table (dist_entry_count × 2 bytes):
    [1B] symbol (0–31)
    [1B] code_length (1–16)

  Literal tree 0–3 tables (ctx_N_entry_count × 3 bytes each):
    [2B] symbol (byte value 0–255, big-endian uint16)
    [1B] code_length (1–16)
    Sorted: (code_length ASC, symbol ASC)

  Bit stream (remaining bytes):
    LSB-first packed. Zero-padded to byte boundary.
    Format: [commands...] [ICC=63 sentinel] [flush literal bytes, if any]
    Flush literals: any bytes at end of input not covered by a copy command.
    Encoded as plain Huffman symbols (per context bucket) after the sentinel.

=cut

# ---------------------------------------------------------------------------
# ICC table — 64 insert-copy codes
# ---------------------------------------------------------------------------
#
# Each entry: [icc_code, insert_base, insert_extra_bits, copy_base, copy_extra_bits]
#
# actual insert_length = insert_base + value(insert_extra_bits raw bits)
# actual copy_length   = copy_base   + value(copy_extra_bits raw bits)
#
# Code 63 is the end-of-data sentinel (insert=0, copy=0).
#
# Important: there is no ICC code that encodes insert > 0 with copy = 0.
# Trailing literals that cannot be attached to a real back-reference must be
# bundled with a synthetic copy of 4 bytes from distance 1. Those extra bytes
# are trimmed by the decompressor using original_length.

my @ICC_TABLE = (
    #  code  ins_base  ins_extra  copy_base  copy_extra
    [  0,  0, 0,   4, 0 ],
    [  1,  0, 0,   5, 0 ],
    [  2,  0, 0,   6, 0 ],
    [  3,  0, 0,   8, 1 ],
    [  4,  0, 0,  10, 1 ],
    [  5,  0, 0,  14, 2 ],
    [  6,  0, 0,  18, 2 ],
    [  7,  0, 0,  26, 3 ],
    [  8,  0, 0,  34, 3 ],
    [  9,  0, 0,  50, 4 ],
    [ 10,  0, 0,  66, 4 ],
    [ 11,  0, 0,  98, 5 ],
    [ 12,  0, 0, 130, 5 ],
    [ 13,  0, 0, 194, 6 ],
    [ 14,  0, 0, 258, 7 ],
    [ 15,  0, 0, 514, 8 ],
    [ 16,  1, 0,   4, 0 ],
    [ 17,  1, 0,   5, 0 ],
    [ 18,  1, 0,   6, 0 ],
    [ 19,  1, 0,   8, 1 ],
    [ 20,  1, 0,  10, 1 ],
    [ 21,  1, 0,  14, 2 ],
    [ 22,  1, 0,  18, 2 ],
    [ 23,  1, 0,  26, 3 ],
    [ 24,  2, 0,   4, 0 ],
    [ 25,  2, 0,   5, 0 ],
    [ 26,  2, 0,   6, 0 ],
    [ 27,  2, 0,   8, 1 ],
    [ 28,  2, 0,  10, 1 ],
    [ 29,  2, 0,  14, 2 ],
    [ 30,  2, 0,  18, 2 ],
    [ 31,  2, 0,  26, 3 ],
    [ 32,  3, 1,   4, 0 ],
    [ 33,  3, 1,   5, 0 ],
    [ 34,  3, 1,   6, 0 ],
    [ 35,  3, 1,   8, 1 ],
    [ 36,  3, 1,  10, 1 ],
    [ 37,  3, 1,  14, 2 ],
    [ 38,  3, 1,  18, 2 ],
    [ 39,  3, 1,  26, 3 ],
    [ 40,  5, 2,   4, 0 ],
    [ 41,  5, 2,   5, 0 ],
    [ 42,  5, 2,   6, 0 ],
    [ 43,  5, 2,   8, 1 ],
    [ 44,  5, 2,  10, 1 ],
    [ 45,  5, 2,  14, 2 ],
    [ 46,  5, 2,  18, 2 ],
    [ 47,  5, 2,  26, 3 ],
    [ 48,  9, 3,   4, 0 ],
    [ 49,  9, 3,   5, 0 ],
    [ 50,  9, 3,   6, 0 ],
    [ 51,  9, 3,   8, 1 ],
    [ 52,  9, 3,  10, 1 ],
    [ 53,  9, 3,  14, 2 ],
    [ 54,  9, 3,  18, 2 ],
    [ 55,  9, 3,  26, 3 ],
    [ 56, 17, 4,   4, 0 ],
    [ 57, 17, 4,   5, 0 ],
    [ 58, 17, 4,   6, 0 ],
    [ 59, 17, 4,   8, 1 ],
    [ 60, 17, 4,  10, 1 ],
    [ 61, 17, 4,  14, 2 ],
    [ 62, 17, 4,  18, 2 ],
    [ 63,  0, 0,   0, 0 ],   # end-of-data sentinel
);

# Quick-lookup arrays indexed by ICC code (0–63).
my @ICC_INS_BASE   = map { $_->[1] } @ICC_TABLE;
my @ICC_INS_EXTRA  = map { $_->[2] } @ICC_TABLE;
my @ICC_COPY_BASE  = map { $_->[3] } @ICC_TABLE;
my @ICC_COPY_EXTRA = map { $_->[4] } @ICC_TABLE;

# ---------------------------------------------------------------------------
# Distance code table — 32 codes (CMP06 extends CMP05's 24 to 32)
# ---------------------------------------------------------------------------
#
# Each entry: [code, base_distance, extra_bits]
# actual distance = base + value(extra_bits raw bits)

my @DIST_TABLE = (
    [  0,     1,  0 ], [  1,     2,  0 ], [  2,     3,  0 ], [  3,     4,  0 ],
    [  4,     5,  1 ], [  5,     7,  1 ], [  6,     9,  2 ], [  7,    13,  2 ],
    [  8,    17,  3 ], [  9,    25,  3 ], [ 10,    33,  4 ], [ 11,    49,  4 ],
    [ 12,    65,  5 ], [ 13,    97,  5 ], [ 14,   129,  6 ], [ 15,   193,  6 ],
    [ 16,   257,  7 ], [ 17,   385,  7 ], [ 18,   513,  8 ], [ 19,   769,  8 ],
    [ 20,  1025,  9 ], [ 21,  1537,  9 ], [ 22,  2049, 10 ], [ 23,  3073, 10 ],
    [ 24,  4097, 11 ], [ 25,  6145, 11 ], [ 26,  8193, 12 ], [ 27, 12289, 12 ],
    [ 28, 16385, 13 ], [ 29, 24577, 13 ], [ 30, 32769, 14 ], [ 31, 49153, 14 ],
);

my @DIST_BASE  = map { $_->[1] } @DIST_TABLE;
my @DIST_EXTRA = map { $_->[2] } @DIST_TABLE;

# ---------------------------------------------------------------------------
# Helper: _literal_context($last_byte_or_undef) → bucket 0..3
# ---------------------------------------------------------------------------
#
# Decision table:
#   last byte undefined (start of stream)  → bucket 0
#   last byte 'a'..'z'                     → bucket 3
#   last byte 'A'..'Z'                     → bucket 2
#   last byte '0'..'9'                     → bucket 1
#   anything else (space, punctuation, …)  → bucket 0

sub _literal_context {
    my ($last) = @_;
    return 0 unless defined $last;
    return 3 if $last >= 97  && $last <= 122;  # 'a'..'z'
    return 2 if $last >= 65  && $last <= 90;   # 'A'..'Z'
    return 1 if $last >= 48  && $last <= 57;   # '0'..'9'
    return 0;
}

# ---------------------------------------------------------------------------
# Helper: _icc_code($insert_length, $copy_length) → ICC code index or undef
# ---------------------------------------------------------------------------
#
# Find the ICC entry (code 0–62) whose insert range contains insert_length
# AND whose copy range contains copy_length. Returns undef if none found.
# Code 63 (sentinel) is excluded.

sub _icc_code {
    my ($ins, $copy) = @_;
    for my $e (@ICC_TABLE[0..62]) {
        my ($code, $ins_base, $ins_extra, $copy_base, $copy_extra) = @$e;
        my $ins_max  = $ins_base  + (1 << $ins_extra)  - 1;
        my $copy_max = $copy_base + (1 << $copy_extra) - 1;
        next if $ins  < $ins_base  || $ins  > $ins_max;
        next if $copy < $copy_base || $copy > $copy_max;
        return $code;
    }
    return undef;
}

# ---------------------------------------------------------------------------
# Helper: _best_copy_for_ins($ins, $desired_copy) → best copy length <= desired
# ---------------------------------------------------------------------------
#
# Find the ICC code (0–62) whose insert range contains $ins and whose copy
# range contains a value as close to $desired_copy as possible from below.
# Returns the largest such copy length, or undef if no match exists.
#
# This is used when _icc_code($ins, $desired_copy) returns undef because
# there's a gap in the copy ranges for this insert group. We find the best
# achievable copy length so the encoder can emit a partial command first and
# continue with the remainder.

sub _best_copy_for_ins {
    my ($ins, $desired) = @_;
    my $best = undef;
    for my $e (@ICC_TABLE[0..62]) {
        my ($code, $ins_base, $ins_extra, $copy_base, $copy_extra) = @$e;
        my $ins_max  = $ins_base + (1 << $ins_extra) - 1;
        next if $ins < $ins_base || $ins > $ins_max;
        my $copy_max = $copy_base + (1 << $copy_extra) - 1;
        # Find the largest copy in [copy_base..min(copy_max, desired)].
        next if $copy_base > $desired;  # entire range is above desired
        my $achievable = ($copy_max <= $desired) ? $copy_max : $desired;
        $best = $achievable if !defined($best) || $achievable > $best;
    }
    return $best;
}

# ---------------------------------------------------------------------------
# Helper: _dist_code($offset) → distance code (0–31)
# ---------------------------------------------------------------------------

sub _dist_code {
    my ($offset) = @_;
    for my $e (@DIST_TABLE) {
        my $max_dist = $e->[1] + (1 << $e->[2]) - 1;
        return $e->[0] if $offset <= $max_dist;
    }
    return 31;
}

# ---------------------------------------------------------------------------
# Helper: _find_longest_match($data, $pos, $window_start) → ($offset, $len)
# ---------------------------------------------------------------------------
#
# O(n²) sliding-window scan. Returns (0,0) if no match of length >= 4.
# Max match length: 258.

sub _find_longest_match {
    my ($data, $pos, $window_start) = @_;
    my $n         = length($data);
    my $remaining = $n - $pos;
    return (0, 0) if $remaining < 4;

    my $best_len = 0;
    my $best_off = 0;
    my $max_len  = $remaining < 258 ? $remaining : 258;

    for (my $candidate = $pos - 1; $candidate >= $window_start; $candidate--) {
        my $offset = $pos - $candidate;
        next unless substr($data, $candidate, 1) eq substr($data, $pos, 1);

        my $len = 1;
        while ($len < $max_len) {
            my $src_idx = $candidate + ($len % $offset);
            last unless substr($data, $src_idx, 1) eq substr($data, $pos + $len, 1);
            $len++;
        }

        if ($len > $best_len) {
            $best_len = $len;
            $best_off = $offset;
            last if $best_len == $max_len;
        }
    }

    return ($best_off, $best_len) if $best_len >= 4;
    return (0, 0);
}

# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

sub _pack_bits_lsb_first {
    my ($bits) = @_;
    my @output;
    my $buffer  = 0;
    my $bit_pos = 0;
    for my $ch (split //, $bits) {
        $buffer |= 1 << $bit_pos if $ch eq '1';
        $bit_pos++;
        if ($bit_pos == 8) {
            push @output, $buffer;
            $buffer  = 0;
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
# Canonical code reconstruction (decode side)
# ---------------------------------------------------------------------------
#
# Returns hashref { bit_string => symbol }.

sub _reconstruct_canonical_codes {
    my ($lengths) = @_;
    return {} unless @$lengths;
    if (@$lengths == 1) {
        return { '0' => $lengths->[0][0] };
    }
    my %result;
    my $code     = 0;
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

Compress a byte string using CMP06 Brotli-style encoding.

=cut

sub compress {
    my ($data) = @_;
    my $original_length = length($data);

    # ── Empty input special case ─────────────────────────────────────────────
    # 10-byte header + 1 ICC entry (code 63, length 1) + 1 bit-stream byte.

    if ($original_length == 0) {
        return pack('N', 0)       # original_length
             . pack('C', 1)       # icc_entry_count = 1
             . pack('C', 0)       # dist_entry_count = 0
             . pack('C', 0)       # ctx0_entry_count = 0
             . pack('C', 0)       # ctx1_entry_count = 0
             . pack('C', 0)       # ctx2_entry_count = 0
             . pack('C', 0)       # ctx3_entry_count = 0
             . pack('CC', 63, 1)  # ICC symbol=63, code_length=1
             . "\x00";            # bit stream: "0" padded to 1 byte
    }

    # ── Pass 1: LZ matching → raw commands ──────────────────────────────────
    #
    # Walk the input accumulating an insert buffer. When a match of length >= 4
    # is found, emit a Command. After the loop, handle remaining literals.
    #
    # Each command:
    #   insert_length  — bytes in @literals before the copy
    #   copy_length    — bytes to copy (0 = no copy)
    #   copy_distance  — offset into history (0 = no copy)
    #   literals       — arrayref of byte values

    my @commands;
    my @insert_buf;
    my $pos = 0;

    while ($pos < $original_length) {
        my $window_start = $pos > 65535 ? $pos - 65535 : 0;
        my ($offset, $length) = _find_longest_match($data, $pos, $window_start);

        # Only take an LZ match when insert_buf is small enough to fit in a
        # valid ICC code (max insert per ICC = 32). If insert_buf is already
        # full, accumulate one more literal and try again next iteration.
        if ($length >= 4 && scalar(@insert_buf) <= 32) {
            my $ins = scalar @insert_buf;

            # Try to find a single ICC code that covers (ins, length).
            my $icc = _icc_code($ins, $length);

            unless (defined $icc) {
                # No single ICC covers (ins, length). Two strategies:
                #
                # A. If ins > 0, find the LARGEST copy length that fits with
                #    this ins, emit a first command with that partial copy, then
                #    continue the remaining copy length in subsequent commands.
                #
                # B. If no copy length > 0 fits with this ins at all, flush
                #    the inserts into the trailing flush_literals and retry.

                if ($ins > 0) {
                    # Find the largest copy length <= $length that has a valid
                    # ICC for this insert count.
                    my $best_copy = _best_copy_for_ins($ins, $length);
                    if (defined $best_copy && $best_copy >= 4) {
                        # Emit first chunk: (ins, best_copy). The remaining
                        # $length - $best_copy bytes will be covered by subsequent
                        # commands with ins=0.
                        push @commands, {
                            insert_length  => $ins,
                            copy_length    => $best_copy,
                            copy_distance  => $offset,
                            literals       => [ @insert_buf ],
                        };
                        @insert_buf = ();
                        $ins     = 0;
                        $length -= $best_copy;
                        $pos    += $best_copy;
                        # Continue with remaining $length as new command(s) below.
                    } else {
                        # No valid ICC covers any copy length for this insert count.
                        # Accumulate current byte into insert_buf and move on.
                        push @insert_buf, ord(substr($data, $pos, 1));
                        $pos++;
                        next;
                    }
                }

                # ins=0; find ICC for (0, length). If length < 4, nothing to do.
                if ($length >= 4) {
                    $icc = _icc_code(0, $length);
                    unless (defined $icc) {
                        # Copy too long for any ICC. Find the best achievable length.
                        my $best = _best_copy_for_ins(0, $length);
                        if (defined $best && $best >= 4) {
                            $length = $best;
                            $icc    = _icc_code(0, $length);
                        } else {
                            $length = 0;  # give up on this copy chunk
                        }
                    }
                }
            }

            if ($length >= 4) {
                push @commands, {
                    insert_length  => scalar(@insert_buf),
                    copy_length    => $length,
                    copy_distance  => $offset,
                    literals       => [ @insert_buf ],
                };
                @insert_buf = ();
                $pos += $length;
            } else {
                push @insert_buf, ord(substr($data, $pos, 1));
                $pos++;
            }
        } else {
            push @insert_buf, ord(substr($data, $pos, 1));
            $pos++;
        }
    }

    # ── Handle trailing literals (flush_literals) ────────────────────────────
    #
    # Any remaining bytes in @insert_buf become flush_literals. They are
    # encoded AFTER the sentinel ICC code 63 in the bit stream, so no synthetic
    # copy padding is needed. The decompressor reads them after the loop exits.

    my @flush_literals = @insert_buf;
    @insert_buf = ();

    # ── Append end-of-data sentinel (ICC code 63) ────────────────────────────
    push @commands, { insert_length => 0, copy_length => 0,
                      copy_distance => 0, literals => [], is_sentinel => 1 };

    # ── Pass 2a: Tally frequencies ───────────────────────────────────────────
    #
    # Walk commands to count literal (per context), ICC, and dist frequencies.
    # @history tracks output bytes so we can compute literal_context correctly.

    my @lit_freq  = ({}, {}, {}, {});
    my %icc_freq;
    my %dist_freq;
    my @history;

    for my $cmd (@commands) {
        for my $byte (@{ $cmd->{literals} }) {
            my $ctx = _literal_context(@history ? $history[-1] : undef);
            $lit_freq[$ctx]{$byte} = ($lit_freq[$ctx]{$byte} // 0) + 1;
            push @history, $byte;
        }
        if ($cmd->{copy_length} > 0) {
            my $icc = _icc_code($cmd->{insert_length}, $cmd->{copy_length});
            $icc_freq{$icc} = ($icc_freq{$icc} // 0) + 1;
            my $dc = _dist_code($cmd->{copy_distance});
            $dist_freq{$dc} = ($dist_freq{$dc} // 0) + 1;
            my $start = scalar(@history) - $cmd->{copy_distance};
            push @history, $history[$start + ($_ % $cmd->{copy_distance})]
                for 0 .. $cmd->{copy_length} - 1;
        }
    }
    $icc_freq{63} = ($icc_freq{63} // 0) + 1;

    # Tally flush_literals (emitted after sentinel in bit stream).
    for my $byte (@flush_literals) {
        my $ctx = _literal_context(@history ? $history[-1] : undef);
        $lit_freq[$ctx]{$byte} = ($lit_freq[$ctx]{$byte} // 0) + 1;
        push @history, $byte;
    }

    # ── Pass 2b: Build Huffman trees ─────────────────────────────────────────

    my $icc_tree = CodingAdventures::HuffmanTree->build(
        [ map { [$_, $icc_freq{$_}] } keys %icc_freq ]
    );
    my $icc_codes = $icc_tree->canonical_code_table();

    my $dist_codes = {};
    if (%dist_freq) {
        my $dist_tree = CodingAdventures::HuffmanTree->build(
            [ map { [$_, $dist_freq{$_}] } keys %dist_freq ]
        );
        $dist_codes = $dist_tree->canonical_code_table();
    }

    my @lit_codes;
    for my $ctx (0..3) {
        if (%{ $lit_freq[$ctx] }) {
            my $lit_tree = CodingAdventures::HuffmanTree->build(
                [ map { [$_, $lit_freq[$ctx]{$_}] } keys %{ $lit_freq[$ctx] } ]
            );
            push @lit_codes, $lit_tree->canonical_code_table();
        } else {
            push @lit_codes, {};
        }
    }

    # ── Pass 2c: Encode the command stream ──────────────────────────────────

    # ── Pass 2c: Encode the command stream ──────────────────────────────────
    #
    # The wire format order within each command MUST match the decoder's read
    # order. The decoder does:
    #
    #   1. Read ICC Huffman symbol
    #   2. Read insert_extra bits → insert_length
    #   3. Read copy_extra bits   → copy_length
    #   4. Read insert_length literal Huffman symbols (one per context)
    #   5. If copy_length > 0: read dist Huffman symbol + dist_extra bits
    #
    # Therefore the encoder must emit in that exact order:
    #   ICC → insert_extra → copy_extra → [literals] → dist → dist_extra
    #
    # Note: this differs from the spec pseudocode, which shows literals BEFORE
    # the ICC symbol. The decoder loop structure defines the authoritative order.

    my $bits = '';
    @history = ();

    for my $cmd (@commands) {
        if ($cmd->{copy_length} > 0) {
            my $icc = _icc_code($cmd->{insert_length}, $cmd->{copy_length});

            # 1. ICC Huffman code.
            $bits .= $icc_codes->{$icc};

            # 2. Insert extra bits (LSB-first).
            my $ins_extra = $ICC_INS_EXTRA[$icc];
            if ($ins_extra > 0) {
                my $ins_val = $cmd->{insert_length} - $ICC_INS_BASE[$icc];
                for my $i (0 .. $ins_extra - 1) {
                    $bits .= (($ins_val >> $i) & 1) ? '1' : '0';
                }
            }

            # 3. Copy extra bits (LSB-first).
            my $copy_extra = $ICC_COPY_EXTRA[$icc];
            if ($copy_extra > 0) {
                my $copy_val = $cmd->{copy_length} - $ICC_COPY_BASE[$icc];
                for my $i (0 .. $copy_extra - 1) {
                    $bits .= (($copy_val >> $i) & 1) ? '1' : '0';
                }
            }

            # 4. Literal bytes (encoded per-context, after ICC so decoder knows count).
            for my $byte (@{ $cmd->{literals} }) {
                my $ctx = _literal_context(@history ? $history[-1] : undef);
                $bits .= $lit_codes[$ctx]{$byte};
                push @history, $byte;
            }

            # 5. Distance Huffman code + extra bits.
            my $dc         = _dist_code($cmd->{copy_distance});
            $bits .= $dist_codes->{$dc};
            my $dist_extra = $DIST_EXTRA[$dc];
            if ($dist_extra > 0) {
                my $dist_val = $cmd->{copy_distance} - $DIST_BASE[$dc];
                for my $i (0 .. $dist_extra - 1) {
                    $bits .= (($dist_val >> $i) & 1) ? '1' : '0';
                }
            }

            # Simulate copy for context tracking.
            my $start = scalar(@history) - $cmd->{copy_distance};
            push @history, $history[$start + ($_ % $cmd->{copy_distance})]
                for 0 .. $cmd->{copy_length} - 1;

        } elsif ($cmd->{is_sentinel}) {
            # Emit the sentinel ICC code.
            $bits .= $icc_codes->{63};
            # Emit flush_literals after the sentinel — no copy, just raw Huffman
            # codes. The decompressor reads these after exiting the command loop.
            for my $byte (@flush_literals) {
                my $ctx = _literal_context(@history ? $history[-1] : undef);
                $bits .= $lit_codes[$ctx]{$byte};
                push @history, $byte;
            }
        }
    }

    my $bit_stream = _pack_bits_lsb_first($bits);

    # ── Assemble wire format ─────────────────────────────────────────────────

    my @icc_lengths = sort {
        $a->[1] <=> $b->[1] || $a->[0] <=> $b->[0]
    } map { [$_, length($icc_codes->{$_})] } keys %$icc_codes;

    my @dist_lengths = sort {
        $a->[1] <=> $b->[1] || $a->[0] <=> $b->[0]
    } map { [$_, length($dist_codes->{$_})] } keys %$dist_codes;

    my @lit_lengths;
    for my $ctx (0..3) {
        my @cl = sort {
            $a->[1] <=> $b->[1] || $a->[0] <=> $b->[0]
        } map { [$_, length($lit_codes[$ctx]{$_})] } keys %{ $lit_codes[$ctx] };
        push @lit_lengths, \@cl;
    }

    my $header = pack('N', $original_length)
               . pack('C', scalar @icc_lengths)
               . pack('C', scalar @dist_lengths)
               . pack('C', scalar @{ $lit_lengths[0] })
               . pack('C', scalar @{ $lit_lengths[1] })
               . pack('C', scalar @{ $lit_lengths[2] })
               . pack('C', scalar @{ $lit_lengths[3] });

    my $icc_bytes  = join('', map { pack('CC', $_->[0], $_->[1]) } @icc_lengths);
    my $dist_bytes = join('', map { pack('CC', $_->[0], $_->[1]) } @dist_lengths);
    my $lit_bytes  = '';
    for my $ctx (0..3) {
        $lit_bytes .= join('', map { pack('nC', $_->[0], $_->[1]) } @{ $lit_lengths[$ctx] });
    }

    return $header . $icc_bytes . $dist_bytes . $lit_bytes . $bit_stream;
}

# ---------------------------------------------------------------------------
# Public API: decompress
# ---------------------------------------------------------------------------

=head2 decompress($data)

Decompress CMP06 wire-format data and return the original byte string.

=cut

sub decompress {
    my ($data) = @_;

    return '' if length($data) < 10;

    my ($original_length, $icc_entry_count, $dist_entry_count,
        $ctx0_count, $ctx1_count, $ctx2_count, $ctx3_count)
        = unpack('NCCCCCC', $data);

    return '' if $original_length == 0;

    my $off = 10;

    # Parse ICC code-length table.
    my @icc_lengths;
    for (1 .. $icc_entry_count) {
        my ($sym, $code_len) = unpack('CC', substr($data, $off, 2));
        push @icc_lengths, [$sym, $code_len];
        $off += 2;
    }

    # Parse dist code-length table.
    my @dist_lengths;
    for (1 .. $dist_entry_count) {
        my ($sym, $code_len) = unpack('CC', substr($data, $off, 2));
        push @dist_lengths, [$sym, $code_len];
        $off += 2;
    }

    # Parse four literal tree code-length tables.
    my @lit_lengths_all;
    for my $count ($ctx0_count, $ctx1_count, $ctx2_count, $ctx3_count) {
        my @cl;
        for (1 .. $count) {
            my ($sym, $code_len) = unpack('nC', substr($data, $off, 3));
            push @cl, [$sym, $code_len];
            $off += 3;
        }
        push @lit_lengths_all, \@cl;
    }

    # Reconstruct canonical decode tables.
    my $icc_rev  = _reconstruct_canonical_codes(\@icc_lengths);
    my $dist_rev = _reconstruct_canonical_codes(\@dist_lengths);
    my @lit_rev  = map { _reconstruct_canonical_codes($_) } @lit_lengths_all;

    # Unpack bit stream.
    my $bits    = _unpack_bits_lsb_first(substr($data, $off));
    my $bit_pos = 0;
    my $bit_len = length($bits);

    # Read n LSB-first bits.
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

    # Decode one Huffman symbol.
    my $next_sym = sub {
        my ($rev_map) = @_;
        my $acc = '';
        while (1) {
            return undef if $bit_pos >= $bit_len;
            $acc .= substr($bits, $bit_pos, 1);
            $bit_pos++;
            my $sym = $rev_map->{$acc};
            return $sym if defined $sym;
        }
    };

    my @output;

    while (1) {
        last if scalar(@output) >= $original_length;

        my $icc = $next_sym->($icc_rev);
        last unless defined $icc;

        if ($icc == 63) {
            # End-of-data sentinel. Read any flush_literals that follow.
            # The encoder emits them right after the sentinel, using the same
            # context-bucket Huffman coding as regular insert literals.
            while (scalar(@output) < $original_length) {
                my $ctx  = _literal_context(@output ? $output[-1] : undef);
                my $byte = $next_sym->($lit_rev[$ctx]);
                last unless defined $byte;
                push @output, $byte;
            }
            last;
        }

        my $ins_extra     = $ICC_INS_EXTRA[$icc];
        my $copy_extra    = $ICC_COPY_EXTRA[$icc];
        my $insert_length = $ICC_INS_BASE[$icc]  + $read_bits->($ins_extra);
        my $copy_length   = $ICC_COPY_BASE[$icc] + $read_bits->($copy_extra);

        # Decode insert literals.
        for my $lit_idx (1 .. $insert_length) {
            last if scalar(@output) >= $original_length;
            my $ctx  = _literal_context(@output ? $output[-1] : undef);
            my $byte = $next_sym->($lit_rev[$ctx]);
            last unless defined $byte;
            push @output, $byte;
        }

        # Decode copy.
        if ($copy_length > 0) {
            my $dc            = $next_sym->($dist_rev);
            last unless defined $dc;
            my $dist_extra    = $DIST_EXTRA[$dc];
            my $dist_val      = $read_bits->($dist_extra);
            my $copy_distance = $DIST_BASE[$dc] + $dist_val;

            my $start = scalar(@output) - $copy_distance;
            for my $i (0 .. $copy_length - 1) {
                last if scalar(@output) >= $original_length;
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

=item * L<CodingAdventures::HuffmanTree> — canonical Huffman tree builder (DT27)

=item * L<CodingAdventures::Deflate> — DEFLATE (CMP05), the predecessor

=back

=head1 REFERENCES

=over 4

=item * RFC 7932 — Brotli Compressed Data Format, July 2016

=back

=head1 AUTHOR

Adhithya Rajasekaran

=head1 LICENSE

MIT

=cut
