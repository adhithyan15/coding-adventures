package CodingAdventures::Zstd;

use strict;
use warnings;

our $VERSION = '0.1.0';

use Exporter 'import';
our @EXPORT_OK = qw(compress decompress);

use CodingAdventures::LZSS qw(encode);

=head1 NAME

CodingAdventures::Zstd - ZStd (RFC 8878) lossless compression from scratch — CMP07

=head1 SYNOPSIS

  use CodingAdventures::Zstd qw(compress decompress);

  my $data       = "the quick brown fox jumps over the lazy dog " x 25;
  my $compressed = compress($data);
  my $original   = decompress($compressed);

=head1 DESCRIPTION

Zstandard (RFC 8878) is a high-ratio, fast compression format designed by
Yann Collet at Meta (Facebook) in 2015. It combines two powerful ideas:

  1. LZ77 back-references (via LZSS token generation) to exploit repetition
     in data — the same "copy from earlier output" trick used by DEFLATE,
     but with a 32 KB sliding window.

  2. FSE (Finite State Entropy) coding for the sequence descriptor symbols.
     FSE is an asymmetric numeral system (ANS variant) that approaches the
     Shannon entropy limit in a single pass — better than Huffman for many
     distributions.

=head2 Frame Layout (RFC 8878 §3)

  +--------+-----+----------------------+--------+------------------+
  | Magic  | FHD | Frame_Content_Size   | Blocks | [Checksum]       |
  | 4 B LE | 1 B | 1/2/4/8 B (LE)      | ...    | 4 B (optional)   |
  +--------+-----+----------------------+--------+------------------+

Each block has a 3-byte header:
  bit 0        = Last_Block flag
  bits [2:1]   = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
  bits [23:3]  = Block_Size

=head2 Compression Strategy

  1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
  2. For each block, try in order:
     a. RLE     — all bytes identical → 5 bytes total.
     b. Compressed (LZ77 + FSE) — if output < input length.
     c. Raw     — verbatim copy (fallback).

=head2 Series

  CMP00 (LZ77)     — Sliding-window back-references
  CMP01 (LZ78)     — Explicit dictionary (trie)
  CMP02 (LZSS)     — LZ77 + flag bits                     <- dependency
  CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
  CMP04 (Huffman)  — Entropy coding
  CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
  CMP06 (Brotli)   — DEFLATE + context modelling + static dict
  CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed       <- this module

=cut

# ============================================================================
# Constants
# ============================================================================

# MAGIC is the ZStd frame magic number: 0xFD2FB528 little-endian.
# In bytes: 0x28, 0xB5, 0x2F, 0xFD.
# Every valid ZStd frame starts with these 4 bytes. The value was chosen to be
# unlikely to appear at the start of plaintext files.
use constant MAGIC          => 0xFD2FB528;

# MAX_BLOCK_SIZE is 128 KB. ZStd allows blocks up to 128 KB; larger inputs are
# split across multiple blocks.
use constant MAX_BLOCK_SIZE => 128 * 1024;

# MAX_OUTPUT guards against decompression bombs (crafted tiny inputs that expand
# to gigabytes). 256 MB is a generous limit for library use.
use constant MAX_OUTPUT     => 256 * 1024 * 1024;

# FSE table accuracy logs (== log2 of table size).
#   LL_ACC_LOG = 6 → 64 slots
#   ML_ACC_LOG = 6 → 64 slots
#   OF_ACC_LOG = 5 → 32 slots
use constant LL_ACC_LOG     => 6;
use constant ML_ACC_LOG     => 6;
use constant OF_ACC_LOG     => 5;

# ============================================================================
# FSE Predefined Distributions (RFC 8878 Appendix B)
# ============================================================================
#
# "Predefined_Mode" means no per-frame table description is transmitted.
# Both encoder and decoder independently build the same table from these
# fixed distributions.
#
# Entries of -1 mean "probability 1/table_size": these symbols each get exactly
# one slot. Their encoder state never needs extra bits.

# LL_NORM: predefined normalised distribution for Literal Length FSE.
# 36 entries (codes 0..35). Table size = 64 slots.
my @LL_NORM = (
     4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
    -1, -1, -1, -1,
);

# ML_NORM: predefined normalised distribution for Match Length FSE.
# 53 entries (codes 0..52). Table size = 64 slots.
my @ML_NORM = (
     1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
    -1, -1, -1, -1, -1,
);

# OF_NORM: predefined normalised distribution for Offset FSE.
# 29 entries (codes 0..28). Table size = 32 slots.
my @OF_NORM = (
     1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
);

# ============================================================================
# LL / ML / OF Code Tables (RFC 8878 §3.1.1.3)
# ============================================================================
#
# Each table maps a code number to [baseline, extra_bits].
#
# To decode a value from a code:
#   value = baseline + read(extra_bits bits from bitstream)
#
# Example: LL code 17 → baseline=18, extra_bits=1
#   if we read 0 extra bits → literal_length = 18
#   if we read 1 extra bit  → literal_length = 19

# LL_CODES: Literal Length codes 0..35 → [baseline, extra_bits].
# Codes 0..15 are identity (one literal length each).
# Codes 16+ cover increasingly large ranges.
my @LL_CODES = (
    [0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0],
    [8,0],[9,0],[10,0],[11,0],[12,0],[13,0],[14,0],[15,0],
    [16,1],[18,1],[20,1],[22,1],
    [24,2],[28,2],
    [32,3],[40,3],
    [48,4],[64,6],
    [128,7],[256,8],[512,9],[1024,10],[2048,11],[4096,12],
    [8192,13],[16384,14],[32768,15],[65536,16],
);

# ML_CODES: Match Length codes 0..52 → [baseline, extra_bits].
# The minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
# Codes 0..31 cover individual values 3..34 (no extra bits).
# Codes 32+ cover grouped ranges with extra bits.
my @ML_CODES = (
    [3,0],[4,0],[5,0],[6,0],[7,0],[8,0],[9,0],[10,0],
    [11,0],[12,0],[13,0],[14,0],[15,0],[16,0],[17,0],[18,0],
    [19,0],[20,0],[21,0],[22,0],[23,0],[24,0],[25,0],[26,0],
    [27,0],[28,0],[29,0],[30,0],[31,0],[32,0],[33,0],[34,0],
    [35,1],[37,1],[39,1],[41,1],
    [43,2],[47,2],
    [51,3],[59,3],
    [67,4],[83,4],
    [99,5],[131,7],
    [259,8],[515,9],[1027,10],[2051,11],
    [4099,12],[8195,13],[16387,14],[32771,15],[65539,16],
);

# ============================================================================
# FSE Decode Table Builder
# ============================================================================
#
# The FSE decode table is an array of { sym, nb, base } records.
# Given FSE state S (an index into the table):
#   1. sym  = the decoded symbol
#   2. nb   = number of extra bits to read for the next state
#   3. base = base value; next_state = base + read(nb bits)

sub _build_decode_table {
    my ($norm_ref, $acc_log) = @_;
    my @norm = @$norm_ref;
    my $sz   = 1 << $acc_log;   # table size (e.g. 64 for acc_log=6)

    # The step function visits every slot exactly once because sz is a power
    # of two and step is co-prime to sz.
    # step = (sz >> 1) + (sz >> 3) + 3
    my $step = ($sz >> 1) + ($sz >> 3) + 3;

    # Pre-fill the table with default entries.
    my @tbl = map { { sym => 0, nb => 0, base => 0 } } (0 .. $sz - 1);

    # sym_next[s] tracks how many times symbol s has been placed so far.
    # During Phase 3 it becomes the "next state counter" for each symbol.
    my @sym_next = (0) x scalar(@norm);

    # ── Phase 1: probability -1 symbols ──────────────────────────────────
    # Symbols with probability -1 each get one slot at the HIGH end of the
    # table (indices sz-1 downward). They are the rarest symbols and their
    # state transition uses the full acc_log bits.
    my $high = $sz - 1;
    for my $s (0 .. $#norm) {
        if ($norm[$s] == -1) {
            $tbl[$high]{sym} = $s;
            $high-- if $high > 0;
            $sym_next[$s] = 1;
        }
    }

    # ── Phase 2: spread remaining symbols ────────────────────────────────
    # Two-pass: first symbols with count > 1, then count == 1.
    # This deterministic ordering matches the reference decoder.
    my $pos = 0;
    for my $pass (0, 1) {
        for my $s (0 .. $#norm) {
            next if $norm[$s] <= 0;
            my $cnt = $norm[$s];
            # pass 0 handles multi-slot symbols; pass 1 handles single-slot
            next if ($pass == 0) != ($cnt > 1);
            $sym_next[$s] = $cnt;
            for my $i (1 .. $cnt) {
                $tbl[$pos]{sym} = $s;
                $pos = ($pos + $step) & ($sz - 1);
                # Skip slots reserved for probability -1 symbols (above $high).
                while ($pos > $high) {
                    $pos = ($pos + $step) & ($sz - 1);
                }
            }
        }
    }

    # ── Phase 3: assign nb (state bits) and base ─────────────────────────
    # For symbol s with count c:
    #   The j-th slot for s (in ascending index order) has:
    #     ns = c + j   (starting at ns = c for j=0)
    #     nb = acc_log - floor(log2(ns))
    #     base = ns * (1 << nb) - sz
    #
    # Why these formulas?
    #   The encoder state E lives in [sz, 2*sz). When the decoder reads nb
    #   bits and computes base + bits, it gets a new state in [sz, 2*sz).
    #   The mapping ensures the encoder's next state matches exactly what
    #   the decoder reconstructs.
    my @sn = @sym_next;  # working copy of sym_next counters
    for my $i (0 .. $sz - 1) {
        my $s  = $tbl[$i]{sym};
        my $ns = $sn[$s];
        $sn[$s]++;
        # floor(log2(ns)): find highest set bit position
        my $log2_ns = 0;
        my $tmp = $ns;
        while ($tmp > 1) { $log2_ns++; $tmp >>= 1; }
        my $nb   = $acc_log - $log2_ns;
        my $base = ($ns << $nb) - $sz;
        $tbl[$i]{nb}   = $nb;
        $tbl[$i]{base} = $base;
    }

    return \@tbl;
}

# ============================================================================
# FSE Encode Symbol Table Builder
# ============================================================================
#
# Returns:
#   $ee_ref: arrayref of { delta_nb, delta_fs } per symbol
#   $st_ref: arrayref of encoder output states (slot -> state in [sz, 2*sz))
#
# The encode step for symbol s, current state E in [sz, 2*sz):
#   nb_out   = (E + delta_nb) >> 16
#   emit the low nb_out bits of E to the backward bitstream
#   new_E    = st[(E >> nb_out) + delta_fs]

sub _build_encode_sym {
    my ($norm_ref, $acc_log) = @_;
    my @norm = @$norm_ref;
    my $sz   = 1 << $acc_log;

    # ── Step 1: cumulative counts ─────────────────────────────────────────
    # cumul[s] = sum of counts of all symbols before s.
    # This gives each symbol a contiguous block of encode slots.
    my @cumul;
    my $total = 0;
    for my $s (0 .. $#norm) {
        $cumul[$s] = $total;
        my $cnt = ($norm[$s] == -1) ? 1 : ($norm[$s] < 0 ? 0 : $norm[$s]);
        $total += $cnt;
    }

    # ── Step 2: build spread table ────────────────────────────────────────
    # Same spreading algorithm as the decode table to ensure symmetry.
    my $step = ($sz >> 1) + ($sz >> 3) + 3;
    my @spread = (0) x $sz;
    my $idx_high = $sz - 1;

    for my $s (0 .. $#norm) {
        if ($norm[$s] == -1) {
            $spread[$idx_high] = $s;
            $idx_high-- if $idx_high > 0;
        }
    }
    my $idx_limit = $idx_high;

    my $pos = 0;
    for my $pass (0, 1) {
        for my $s (0 .. $#norm) {
            next if $norm[$s] <= 0;
            my $cnt = $norm[$s];
            next if ($pass == 0) != ($cnt > 1);
            for my $i (1 .. $cnt) {
                $spread[$pos] = $s;
                $pos = ($pos + $step) & ($sz - 1);
                while ($pos > $idx_limit) {
                    $pos = ($pos + $step) & ($sz - 1);
                }
            }
        }
    }

    # ── Step 3: build state table ─────────────────────────────────────────
    # For each table index i (in ascending order), the j-th occurrence of
    # symbol s = spread[i] maps to encode slot cumul[s]+j.
    # The encoder output state for that slot is i + sz.
    my @sym_occ = (0) x scalar(@norm);
    my @st      = (0) x $sz;
    for my $i (0 .. $sz - 1) {
        my $s    = $spread[$i];
        my $j    = $sym_occ[$s];
        $sym_occ[$s]++;
        my $slot = $cumul[$s] + $j;
        $st[$slot] = $i + $sz;   # output state in [sz, 2*sz)
    }

    # ── Step 4: FseEe entries ─────────────────────────────────────────────
    # For symbol s with count c and max_bits_out mbo:
    #   delta_nb = (mbo << 16) - (c << mbo)
    #   delta_fs = cumul[s] - c
    #
    # These precomputed deltas let the hot-path encode loop use just:
    #   nb_out = (E + delta_nb) >> 16
    #   new_E  = st[(E >> nb_out) + delta_fs]
    my @ee;
    for my $s (0 .. $#norm) {
        my $cnt = ($norm[$s] == -1) ? 1 : ($norm[$s] < 0 ? 0 : $norm[$s]);
        if ($cnt == 0) {
            push @ee, { delta_nb => 0, delta_fs => 0 };
            next;
        }
        my $mbo;
        if ($cnt == 1) {
            $mbo = $acc_log;
        } else {
            my $log2_cnt = 0;
            my $tmp = $cnt;
            while ($tmp > 1) { $log2_cnt++; $tmp >>= 1; }
            $mbo = $acc_log - $log2_cnt;
        }
        my $delta_nb = ($mbo << 16) - ($cnt << $mbo);
        my $delta_fs = $cumul[$s] - $cnt;
        push @ee, { delta_nb => $delta_nb, delta_fs => $delta_fs };
    }

    return (\@ee, \@st);
}

# ============================================================================
# Reverse Bit Writer
# ============================================================================
#
# ZStd's sequence bitstream is written *backwards*: bits that the decoder
# reads last are written first. This lets the decoder read a forward-only
# stream while decoding sequences in order.
#
# Byte layout: [byte0, byte1, ..., byteN] where byteN is the LAST byte
# written. It contains a sentinel bit — the highest set bit — that marks
# the end of meaningful data.
#
# Bit layout within each byte: LSB = first bit written.
#
# Example: write bits 1,0,1,1 (4 bits) then flush:
#   reg=0b1011, bits=4
#   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
#   buf = [0x1B]
# Decoder finds MSB (bit 4 = sentinel), then reads bits 3..0 = 0b1011.

package RevBitWriter;

sub new { bless { buf => [], reg => 0, bits => 0 }, shift }

# add_bits adds the low $nb bits of $val to the backward bitstream.
# Bits accumulate in $reg from LSB. When $reg has 8+ bits, the low byte
# is flushed to $buf.
sub add_bits {
    my ($self, $val, $nb) = @_;
    return if $nb == 0;

    # Mask to exactly $nb bits, then OR into the register at the current
    # bit position ($self->{bits} bits already occupied from LSB).
    my $mask = ($nb == 64) ? ~0 : ((1 << $nb) - 1);
    $self->{reg} |= ($val & $mask) << $self->{bits};
    $self->{reg} &= ~0;  # keep within UV (64-bit unsigned on 64-bit Perl)
    $self->{bits} += $nb;

    # Drain full bytes from the LSB of the register.
    while ($self->{bits} >= 8) {
        push @{$self->{buf}}, $self->{reg} & 0xFF;
        $self->{reg} >>= 8;
        $self->{bits} -= 8;
    }
}

# flush writes any remaining partial byte with a sentinel bit at position
# $self->{bits}. The sentinel is the lowest bit above all data bits:
#   last_byte = (remaining_data_bits) | (1 << bits_used)
sub flush {
    my $self = shift;
    my $sentinel = 1 << $self->{bits};   # bit just above the data bits
    push @{$self->{buf}}, ($self->{reg} & 0xFF) | $sentinel;
}

# finish returns the accumulated byte array.
sub finish { return $_[0]->{buf} }

# ============================================================================
# Reverse Bit Reader
# ============================================================================
#
# Mirrors RevBitWriter: reads bits from the END of the buffer going backward.
# The last byte written (with sentinel) is the first byte consumed.
#
# Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side of
# a 64-bit register). read_bits(n) extracts the top n bits and shifts left.
#
# Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
# byte, bit 0 = earliest written, bit N = latest written. The reader processes
# the latest bits first (they are at the end of the stream / in high positions
# of the sentinel byte), so a left-aligned register gives them precedence.

package RevBitReader;

sub new {
    my ($class, $bytes_ref) = @_;  # $bytes_ref: arrayref of 0-255 integers
    my @bytes = @$bytes_ref;
    my $n     = scalar @bytes;
    die "RevBitReader: empty bitstream\n" if $n == 0;

    my $last = $bytes[$n - 1];
    die "RevBitReader: last byte is zero (no sentinel)\n" if $last == 0;

    # Find the sentinel bit: the highest set bit in the last byte.
    # sentinel_pos = floor(log2(last))  (bit index, 0 = LSB)
    my $sp = 0;
    while ((1 << ($sp + 1)) <= $last) { $sp++ }
    # valid_bits = number of data bits below the sentinel
    my $valid_bits = $sp;

    # Place the $valid_bits data bits of the sentinel byte at the TOP (MSB
    # side) of the 64-bit register.
    #   data portion = last & ((1 << valid_bits) - 1)
    #   shift to top = 64 - valid_bits
    my $reg = 0;
    if ($valid_bits > 0) {
        my $mask = (1 << $valid_bits) - 1;
        $reg = (($last & $mask) << (64 - $valid_bits)) & ~0;
    }

    my $self = bless {
        bytes => \@bytes,
        reg   => $reg,
        bits  => $valid_bits,
        pos   => $n - 1,  # index of sentinel byte (already consumed above)
    }, $class;

    # Fill the register with more bytes from the stream.
    $self->_reload();
    return $self;
}

# _reload loads bytes into the register from the stream going backward.
# Each new byte is placed just BELOW the currently loaded bits.
# "Just below" means at bit position (64 - bits - 8) in the left-aligned reg.
sub _reload {
    my $self = shift;
    while ($self->{bits} <= 56 && $self->{pos} > 0) {
        $self->{pos}--;
        my $shift = 64 - $self->{bits} - 8;
        $self->{reg} |= ($self->{bytes}[$self->{pos}] << $shift) & ~0;
        $self->{bits} += 8;
    }
}

# read_bits extracts the top $nb bits of the register (the bits written last
# by the encoder = logically first in the backward stream).
sub read_bits {
    my ($self, $nb) = @_;
    return 0 if $nb == 0;

    # Extract top $nb bits: shift right by (64 - $nb).
    my $val = ($self->{reg} >> (64 - $nb)) & ((1 << $nb) - 1);

    # Shift the register left to consume those bits.
    if ($nb == 64) {
        $self->{reg} = 0;
    } else {
        $self->{reg} = ($self->{reg} << $nb) & ~0;
    }
    $self->{bits} -= $nb;
    $self->{bits} = 0 if $self->{bits} < 0;

    $self->_reload() if $self->{bits} < 24;
    return $val;
}

# ============================================================================
# FSE Encode/Decode Helpers
# ============================================================================

package CodingAdventures::Zstd;

# _fse_encode_sym encodes one symbol into the backward bitstream.
# The FSE encoder state E must be in [sz, 2*sz) before each call.
#
# Encode step:
#   nb_out   = (E + delta_nb) >> 16    # bits to emit
#   emit low nb_out bits of E to bw
#   new_E    = st[(E >> nb_out) + delta_fs]
#
# After all symbols are encoded, the final state (E - sz) is written as
# acc_log bits so the decoder can initialise.
sub _fse_encode_sym {
    my ($state_ref, $sym, $ee_ref, $st_ref, $bw) = @_;
    my $e       = $ee_ref->[$sym];
    # (E + delta_nb) >> 16 gives the number of bits to emit.
    # delta_nb = (mbo<<16) - (cnt<<mbo), so when E is just past the threshold
    # for this symbol, this evaluates to mbo or mbo-1.
    my $nb      = ($$state_ref + $e->{delta_nb}) >> 16;
    $bw->add_bits($$state_ref, $nb);
    my $slot    = ($$state_ref >> $nb) + $e->{delta_fs};
    $$state_ref = $st_ref->[$slot];
}

# _fse_decode_sym decodes one symbol from the backward bitstream.
#
# Decode step:
#   sym   = tbl[state]{sym}
#   nb    = tbl[state]{nb}
#   base  = tbl[state]{base}
#   new_state = base + read(nb bits)
sub _fse_decode_sym {
    my ($state_ref, $tbl_ref, $br) = @_;
    my $e       = $tbl_ref->[$$state_ref];
    my $sym     = $e->{sym};
    my $next    = $e->{base} + $br->read_bits($e->{nb});
    $$state_ref = $next;
    return $sym;
}

# ============================================================================
# LL / ML Code Number Computation
# ============================================================================

# _ll_to_code maps a literal length value to its LL code number (0..35).
# Codes 0..15 are identity; codes 16+ cover ranges.
# Scan LL_CODES from left to right; the last entry whose baseline <= ll wins.
sub _ll_to_code {
    my ($ll) = @_;
    my $code = 0;
    for my $i (0 .. $#LL_CODES) {
        if ($LL_CODES[$i][0] <= $ll) { $code = $i; }
        else                         { last; }
    }
    return $code;
}

# _ml_to_code maps a match length value to its ML code number (0..52).
sub _ml_to_code {
    my ($ml) = @_;
    my $code = 0;
    for my $i (0 .. $#ML_CODES) {
        if ($ML_CODES[$i][0] <= $ml) { $code = $i; }
        else                         { last; }
    }
    return $code;
}

# ============================================================================
# Token Conversion: LZSS → ZStd Sequences
# ============================================================================
#
# LZSS produces a stream of { kind=>'literal', byte } and
# { kind=>'match', offset, length } tokens.
#
# ZStd groups consecutive literals before each match into a single sequence:
#   Sequence = (literal_length, match_length, match_offset)
#
# Any trailing literals (after the last match) go into the literals buffer
# without a corresponding sequence entry.
#
# Returns: (\@lits, \@seqs)
#   @lits: flat array of literal byte values
#   @seqs: array of hashrefs { ll, ml, off }

sub _tokens_to_seqs {
    my ($tokens_ref) = @_;
    my @lits;
    my @seqs;
    my $lit_run = 0;

    for my $tok (@$tokens_ref) {
        if ($tok->{kind} eq 'literal') {
            push @lits, $tok->{byte};
            $lit_run++;
        } else {
            # match token: emit sequence (ll, ml, off)
            push @seqs, {
                ll  => $lit_run,
                ml  => $tok->{length},
                off => $tok->{offset},
            };
            $lit_run = 0;
        }
    }
    # Trailing literals remain in @lits; no sequence for them.
    return (\@lits, \@seqs);
}

# ============================================================================
# Literals Section Encoding (RFC 8878 §3.1.1.2)
# ============================================================================
#
# We use Raw_Literals (type=0): no Huffman coding, bytes stored verbatim.
# The header format depends on how many literal bytes there are:
#
#   ≤ 31 bytes:  1-byte header  = (len << 3) | 0b000
#                                 (5-bit size, Size_Format=00, Type=00)
#   ≤ 4095 bytes: 2-byte header = (len << 4) | 0b0100
#                                 (12-bit size, Size_Format=01, Type=00)
#   otherwise:   3-byte header  = (len << 4) | 0b1100
#                                 (20-bit size, Size_Format=11, Type=00)

sub _encode_literals_section {
    my ($lits_ref) = @_;
    my @lits = @$lits_ref;
    my $n    = scalar @lits;
    my @out;

    if ($n <= 31) {
        # 1-byte header: top 5 bits = size, low 3 bits = 0b000
        push @out, ($n << 3) & 0xFF;
    } elsif ($n <= 4095) {
        # 2-byte header: top 12 bits = size, low 4 bits = 0b0100
        my $hdr = ($n << 4) | 0b0100;
        push @out, $hdr & 0xFF, ($hdr >> 8) & 0xFF;
    } else {
        # 3-byte header: bits [19:4] = size, low 4 bits = 0b1100
        my $hdr = ($n << 4) | 0b1100;
        push @out, $hdr & 0xFF, ($hdr >> 8) & 0xFF, ($hdr >> 16) & 0xFF;
    }

    push @out, @lits;
    return \@out;
}

# _decode_literals_section parses a Raw_Literals section from a byte array.
# Returns (\@lits, $bytes_consumed).
sub _decode_literals_section {
    my ($data_ref, $start) = @_;
    $start //= 0;
    my @data = @$data_ref;

    die "decode_literals: empty section\n" unless @data > $start;

    my $b0    = $data[$start];
    my $ltype = $b0 & 0b11;    # Literals_Block_Type: low 2 bits

    die "decode_literals: unsupported type $ltype (only Raw=0 supported)\n"
        if $ltype != 0;

    my $size_fmt = ($b0 >> 2) & 0b11;

    my ($n, $header_bytes);
    if ($size_fmt == 0 || $size_fmt == 2) {
        # 1-byte header: size in bits [7:3] (values 0..31)
        $n            = $b0 >> 3;
        $header_bytes = 1;
    } elsif ($size_fmt == 1) {
        # 2-byte header: 12-bit size — bits [7:4] of b0 + all 8 bits of b1
        die "decode_literals: truncated 2-byte header\n"
            unless @data >= $start + 2;
        $n            = (($b0 >> 4) & 0xF) | ($data[$start + 1] << 4);
        $header_bytes = 2;
    } else {
        # 3-byte header: 20-bit size
        die "decode_literals: truncated 3-byte header\n"
            unless @data >= $start + 3;
        $n = (($b0 >> 4) & 0xF)
           | ($data[$start + 1] << 4)
           | ($data[$start + 2] << 12);
        $header_bytes = 3;
    }

    my $data_start = $start + $header_bytes;
    my $data_end   = $data_start + $n;
    die "decode_literals: data truncated (need $data_end, have " . scalar(@data) . ")\n"
        if $data_end > scalar @data;

    my @lits = @data[$data_start .. $data_end - 1];
    return (\@lits, $header_bytes + $n);
}

# ============================================================================
# Sequences Section Encoding (RFC 8878 §3.1.1.3)
# ============================================================================
#
# Layout:
#   [sequence_count: 1-3 bytes]
#   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
#   [FSE bitstream: variable, backward]
#
# Symbol compression modes byte:
#   bits [7:6] = LL mode
#   bits [5:4] = OF mode
#   bits [3:2] = ML mode
#   bits [1:0] = reserved (0)
# Mode 0 = Predefined (tables built from fixed distributions above).
# We always write 0x00 (all Predefined).
#
# FSE bitstream (backward bit-stream):
#   Sequences are encoded in REVERSE ORDER (last first).
#   For each sequence:
#     OF extra bits, ML extra bits, LL extra bits  (in this order)
#     then FSE symbol for OF, ML, LL              (in this order)
#   After all sequences, flush final FSE states:
#     (state_of - sz_of) as OF_ACC_LOG bits
#     (state_ml - sz_ml) as ML_ACC_LOG bits
#     (state_ll - sz_ll) as LL_ACC_LOG bits
#   Then add sentinel and flush.
#
# The decoder mirrors this:
#   1. Read LL_ACC_LOG bits → initial state_ll
#   2. Read ML_ACC_LOG bits → initial state_ml
#   3. Read OF_ACC_LOG bits → initial state_of
#   4. For each sequence:
#       decode LL symbol (state transition)
#       decode OF symbol
#       decode ML symbol
#       read LL extra bits
#       read ML extra bits
#       read OF extra bits
#   5. Apply sequences to output.

# _encode_seq_count encodes the number of sequences per RFC 8878 §3.1.1.3.1.
#
#   count < 128        → 1 byte: byte[0] = count
#   count < 0x7F00     → 2 bytes: byte[0] = (count>>8) | 0x80
#                                  byte[1] = count & 0xFF
#   count >= 0x7F00    → 3 bytes: byte[0] = 0xFF
#                                  (count - 0x7F00) as LE u16 in bytes 1-2
#
# The 2-byte encoding puts the high octet (with flag bit 0x80) FIRST, so the
# decoder can always determine the encoding length by checking byte[0].
sub _encode_seq_count {
    my ($count) = @_;
    if ($count < 128) {
        return [$count];
    }
    if ($count < 0x7F00) {
        # byte[0] = high octet | 0x80, byte[1] = low octet
        return [($count >> 8) | 0x80, $count & 0xFF];
    }
    my $r = $count - 0x7F00;
    return [0xFF, $r & 0xFF, ($r >> 8) & 0xFF];
}

# _decode_seq_count decodes the sequence count from a byte array at position $pos.
# Returns ($count, $bytes_consumed).
sub _decode_seq_count {
    my ($data_ref, $pos) = @_;
    my @data = @$data_ref;
    $pos //= 0;

    die "decode_seq_count: empty\n" unless @data > $pos;

    my $b0 = $data[$pos];
    if ($b0 < 128) {
        # 1-byte encoding: count = b0
        return ($b0, 1);
    } elsif ($b0 < 0xFF) {
        # 2-byte encoding: count = ((b0 & 0x7F) << 8) | b1
        die "decode_seq_count: truncated 2-byte\n" unless @data >= $pos + 2;
        my $count = (($b0 & 0x7F) << 8) | $data[$pos + 1];
        return ($count, 2);
    } else {
        # 3-byte encoding: count = b1 + (b2<<8) + 0x7F00
        die "decode_seq_count: truncated 3-byte\n" unless @data >= $pos + 3;
        my $count = 0x7F00 + $data[$pos + 1] + ($data[$pos + 2] << 8);
        return ($count, 3);
    }
}

# _encode_sequences_section builds the full FSE-encoded sequences section.
sub _encode_sequences_section {
    my ($seqs_ref) = @_;
    my @seqs = @$seqs_ref;

    # Build FSE encode tables from predefined distributions.
    my ($ee_ll, $st_ll) = _build_encode_sym(\@LL_NORM, LL_ACC_LOG);
    my ($ee_ml, $st_ml) = _build_encode_sym(\@ML_NORM, ML_ACC_LOG);
    my ($ee_of, $st_of) = _build_encode_sym(\@OF_NORM, OF_ACC_LOG);

    my $sz_ll = 1 << LL_ACC_LOG;   # 64
    my $sz_ml = 1 << ML_ACC_LOG;   # 64
    my $sz_of = 1 << OF_ACC_LOG;   # 32

    # FSE encoder states start at table_size (= sz). Valid range: [sz, 2*sz).
    my $state_ll = $sz_ll;
    my $state_ml = $sz_ml;
    my $state_of = $sz_of;

    my $bw = RevBitWriter->new();

    # Encode sequences in REVERSE order (so the decoder, reading a backward
    # stream forward, reconstructs them in the original order).
    for my $seq (reverse @seqs) {
        my $ll_code = _ll_to_code($seq->{ll});
        my $ml_code = _ml_to_code($seq->{ml});

        # Offset encoding (RFC 8878 §3.1.1.3.2.1):
        #   raw_offset = offset + 3  (avoids raw values 1, 2, 3 which are
        #                              reserved for "repeat offsets")
        #   of_code   = floor(log2(raw_offset))
        #   of_extra  = raw_offset - (1 << of_code)
        my $raw_off = $seq->{off} + 3;
        my $of_code = 0;
        { my $tmp = $raw_off; while ($tmp > 1) { $of_code++; $tmp >>= 1; } }
        my $of_extra = $raw_off - (1 << $of_code);

        # Write extra bits (OF, ML, LL — in this order for backward stream).
        $bw->add_bits($of_extra, $of_code);
        $bw->add_bits($seq->{ml} - $ML_CODES[$ml_code][0], $ML_CODES[$ml_code][1]);
        $bw->add_bits($seq->{ll} - $LL_CODES[$ll_code][0], $LL_CODES[$ll_code][1]);

        # FSE encode symbols.
        # Decode order: LL first, then OF, then ML.
        # Backward write order (reversed): ML → OF → LL.
        _fse_encode_sym(\$state_ml, $ml_code, $ee_ml, $st_ml, $bw);
        _fse_encode_sym(\$state_of, $of_code, $ee_of, $st_of, $bw);
        _fse_encode_sym(\$state_ll, $ll_code, $ee_ll, $st_ll, $bw);
    }

    # Flush final states (initial states for the decoder).
    # Written in order: OF, ML, LL → decoder reads LL first, then ML, then OF.
    $bw->add_bits($state_of - $sz_of, OF_ACC_LOG);
    $bw->add_bits($state_ml - $sz_ml, ML_ACC_LOG);
    $bw->add_bits($state_ll - $sz_ll, LL_ACC_LOG);
    $bw->flush();

    return $bw->finish();  # arrayref of byte values
}

# ============================================================================
# Block-Level Compress / Decompress
# ============================================================================

# _compress_block compresses one block using LZ77 (LZSS) + FSE encoding.
# Returns undef if the compressed form is no smaller than the input.
sub _compress_block {
    my ($block_ref) = @_;
    my @block = @$block_ref;

    # Use LZSS for LZ77 token generation.
    # Window=32768 (32 KB), max_match=255, min_match=3.
    my $block_str = pack('C*', @block);
    my @tokens = CodingAdventures::LZSS::encode($block_str, 32768, 255, 3);

    # Convert LZSS tokens to ZStd sequences.
    my ($lits_ref, $seqs_ref) = _tokens_to_seqs(\@tokens);

    # If LZ77 found no matches, a compressed block would only add overhead.
    return undef unless @$seqs_ref;

    my @out;

    # Literals section (Raw_Literals, no Huffman).
    my $lits_section = _encode_literals_section($lits_ref);
    push @out, @$lits_section;

    # Sequence count.
    my $sc = _encode_seq_count(scalar @$seqs_ref);
    push @out, @$sc;

    # Symbol compression modes: 0x00 = all Predefined.
    push @out, 0x00;

    # FSE bitstream.
    my $bitstream = _encode_sequences_section($seqs_ref);
    push @out, @$bitstream;

    # Fall back to Raw if we didn't gain anything.
    return undef if scalar @out >= scalar @block;

    return \@out;
}

# _decompress_block decompresses one ZStd compressed block.
# Appends decoded bytes to $out_ref (arrayref of bytes).
sub _decompress_block {
    my ($data_ref, $out_ref) = @_;
    my @data = @$data_ref;

    # ── Literals section ─────────────────────────────────────────────────
    my ($lits_ref, $lit_consumed) = _decode_literals_section(\@data, 0);
    my $pos = $lit_consumed;

    # ── Sequence count ───────────────────────────────────────────────────
    if ($pos >= scalar @data) {
        # Block has only literals (no sequences section).
        push @$out_ref, @$lits_ref;
        return;
    }

    my ($n_seqs, $sc_bytes) = _decode_seq_count(\@data, $pos);
    $pos += $sc_bytes;

    if ($n_seqs == 0) {
        push @$out_ref, @$lits_ref;
        return;
    }

    # ── Symbol compression modes ─────────────────────────────────────────
    die "decompress_block: missing modes byte\n" if $pos >= scalar @data;
    my $modes_byte = $data[$pos];
    $pos++;

    my $ll_mode = ($modes_byte >> 6) & 3;
    my $of_mode = ($modes_byte >> 4) & 3;
    my $ml_mode = ($modes_byte >> 2) & 3;
    die "decompress_block: unsupported FSE modes LL=$ll_mode OF=$of_mode ML=$ml_mode\n"
        if $ll_mode || $of_mode || $ml_mode;

    # ── FSE bitstream ────────────────────────────────────────────────────
    my @bitstream = @data[$pos .. $#data];
    die "decompress_block: empty FSE bitstream\n" unless @bitstream;

    my $br = RevBitReader->new(\@bitstream);

    # Build FSE decode tables from predefined distributions.
    my $dt_ll = _build_decode_table(\@LL_NORM, LL_ACC_LOG);
    my $dt_ml = _build_decode_table(\@ML_NORM, ML_ACC_LOG);
    my $dt_of = _build_decode_table(\@OF_NORM, OF_ACC_LOG);

    # Initialise FSE states from the bitstream.
    # Encoder wrote: state_ll, state_ml, state_of.
    # Decoder reads: LL first, then ML, then OF.
    my $state_ll = $br->read_bits(LL_ACC_LOG);
    my $state_ml = $br->read_bits(ML_ACC_LOG);
    my $state_of = $br->read_bits(OF_ACC_LOG);

    my $lit_pos = 0;

    for my $seq_i (1 .. $n_seqs) {
        # Decode symbols (one each per state machine), then read extra bits.
        # Decode order: LL, OF, ML.
        my $ll_code = _fse_decode_sym(\$state_ll, $dt_ll, $br);
        my $of_code = _fse_decode_sym(\$state_of, $dt_of, $br);
        my $ml_code = _fse_decode_sym(\$state_ml, $dt_ml, $br);

        die "decompress_block: invalid LL code $ll_code\n"
            if $ll_code >= scalar @LL_CODES;
        die "decompress_block: invalid ML code $ml_code\n"
            if $ml_code >= scalar @ML_CODES;

        # Read extra bits for each field.
        my $ll = $LL_CODES[$ll_code][0] + $br->read_bits($LL_CODES[$ll_code][1]);
        my $ml = $ML_CODES[$ml_code][0] + $br->read_bits($ML_CODES[$ml_code][1]);
        # Offset decode: of_raw = (1 << of_code) | read(of_code bits)
        #   offset = of_raw - 3
        my $of_raw = (1 << $of_code) | $br->read_bits($of_code);
        my $offset = $of_raw - 3;

        die "decompress_block: offset underflow (of_raw=$of_raw)\n" if $offset < 0;

        # Emit $ll literal bytes from the literals buffer.
        my $lit_end = $lit_pos + $ll;
        die "decompress_block: literal run overflows buffer\n"
            if $lit_end > scalar @$lits_ref;
        push @$out_ref, @{$lits_ref}[$lit_pos .. $lit_end - 1];
        $lit_pos = $lit_end;

        # Copy $ml bytes from $offset positions back in the output buffer.
        die "decompress_block: bad match offset $offset (output len " . scalar(@$out_ref) . ")\n"
            if $offset == 0 || $offset > scalar @$out_ref;
        my $copy_start = scalar(@$out_ref) - $offset;
        for my $i (0 .. $ml - 1) {
            push @$out_ref, $out_ref->[$copy_start + $i];
        }
    }

    # Any remaining literals after the last sequence.
    push @$out_ref, @{$lits_ref}[$lit_pos .. $#$lits_ref];
}

# ============================================================================
# Public API
# ============================================================================

=head2 compress

  my $compressed = CodingAdventures::Zstd::compress($data);
  # or:
  my $compressed = CodingAdventures::Zstd->compress($data);

Compresses C<$data> to a ZStd frame (RFC 8878). Returns a binary string.

The output is a valid ZStd frame decompressible by the C<zstd> CLI tool or any
conforming RFC 8878 implementation.

=cut

sub compress {
    my $data = @_ > 1 ? $_[1] : $_[0];

    my @bytes = unpack('C*', $data);
    my @out;

    # ── ZStd frame header ────────────────────────────────────────────────
    # Magic number (4 bytes LE): 0xFD2FB528
    push @out, 0x28, 0xB5, 0x2F, 0xFD;

    # Frame Header Descriptor (FHD):
    #   bits [7:6]: FCS_Field_Size = 11 → 8-byte Frame_Content_Size
    #   bit  [5]:   Single_Segment_Flag = 1 → no Window_Descriptor
    #   bit  [4]:   Content_Checksum_Flag = 0
    #   bits [3:2]: reserved = 0
    #   bits [1:0]: Dict_ID_Flag = 0
    # = 0b1110_0000 = 0xE0
    push @out, 0xE0;

    # Frame_Content_Size: uncompressed length as 8-byte LE integer.
    # Allows decoders to pre-allocate the output buffer.
    my $content_size = scalar @bytes;
    push @out,
        $content_size & 0xFF,
        ($content_size >> 8)  & 0xFF,
        ($content_size >> 16) & 0xFF,
        ($content_size >> 24) & 0xFF,
        0, 0, 0, 0;   # upper 4 bytes are zero (< 4 GB)

    # ── Blocks ───────────────────────────────────────────────────────────
    if (!@bytes) {
        # Empty input: one empty Raw block (Last=1, Type=Raw, Size=0).
        # Header = 0b0000_0001 = 0x01 (padded to 3 bytes LE)
        push @out, 0x01, 0x00, 0x00;
        return pack('C*', @out);
    }

    my $offset = 0;
    while ($offset < scalar @bytes) {
        my $end   = $offset + MAX_BLOCK_SIZE;
        $end      = scalar @bytes if $end > scalar @bytes;
        my @block = @bytes[$offset .. $end - 1];
        my $last  = ($end == scalar @bytes) ? 1 : 0;

        # ── Try RLE block ──────────────────────────────────────────────
        # If all bytes are identical, an RLE block costs just 4 bytes
        # (3-byte header + 1 payload byte), regardless of block size.
        my $all_same = 1;
        for my $b (@block) {
            if ($b != $block[0]) { $all_same = 0; last; }
        }

        if (@block && $all_same) {
            # Block_Type = 01 (RLE).
            # Header bits: [23:3]=block_size, [2:1]=01 (RLE), [0]=last
            my $hdr = (scalar(@block) << 3) | (0b01 << 1) | $last;
            push @out, $hdr & 0xFF, ($hdr >> 8) & 0xFF, ($hdr >> 16) & 0xFF;
            push @out, $block[0];

        } else {
            # ── Try compressed block ──────────────────────────────────
            my $comp = _compress_block(\@block);

            if (defined $comp) {
                # Block_Type = 10 (Compressed).
                my $hdr = (scalar(@$comp) << 3) | (0b10 << 1) | $last;
                push @out, $hdr & 0xFF, ($hdr >> 8) & 0xFF, ($hdr >> 16) & 0xFF;
                push @out, @$comp;
            } else {
                # ── Raw block (fallback) ──────────────────────────────
                # Block_Type = 00 (Raw).
                my $hdr = (scalar(@block) << 3) | (0b00 << 1) | $last;
                push @out, $hdr & 0xFF, ($hdr >> 8) & 0xFF, ($hdr >> 16) & 0xFF;
                push @out, @block;
            }
        }

        $offset = $end;
    }

    return pack('C*', @out);
}

=head2 decompress

  my $data = CodingAdventures::Zstd::decompress($compressed);

Decompresses a ZStd frame. Returns a binary string or dies on error.

Supports:
  - Single-segment or multi-segment frames
  - Raw, RLE, or Compressed blocks
  - Predefined FSE modes (no per-frame table description)

Dies with an error message if the input is truncated, has a bad magic number,
or contains unsupported features (non-predefined FSE tables, Huffman literals,
reserved block types).

=cut

sub decompress {
    my ($data) = @_;
    my @bytes = unpack('C*', $data);

    die "zstd: frame too short\n" if @bytes < 5;

    # ── Validate magic ───────────────────────────────────────────────────
    my $magic = $bytes[0] | ($bytes[1] << 8) | ($bytes[2] << 16) | ($bytes[3] << 24);
    die sprintf("zstd: bad magic 0x%08X (expected 0x%08X)\n", $magic, MAGIC)
        unless $magic == MAGIC;

    my $pos = 4;

    # ── Parse Frame Header Descriptor ───────────────────────────────────
    my $fhd = $bytes[$pos++];

    # FCS_Field_Size: bits [7:6]
    #   00 → 0 bytes if Single_Segment=0, else 1 byte
    #   01 → 2 bytes (value + 256)
    #   10 → 4 bytes
    #   11 → 8 bytes
    my $fcs_flag   = ($fhd >> 6) & 3;
    my $single_seg = ($fhd >> 5) & 1;
    my $dict_flag  = $fhd & 3;

    # ── Window Descriptor ────────────────────────────────────────────────
    # Present only when Single_Segment_Flag = 0.
    $pos++ if $single_seg == 0;

    # ── Dict ID ──────────────────────────────────────────────────────────
    my @dict_id_sizes = (0, 1, 2, 4);
    $pos += $dict_id_sizes[$dict_flag];

    # ── Frame Content Size ───────────────────────────────────────────────
    my $fcs_bytes;
    if ($fcs_flag == 0) {
        $fcs_bytes = ($single_seg == 1) ? 1 : 0;
    } elsif ($fcs_flag == 1) {
        $fcs_bytes = 2;
    } elsif ($fcs_flag == 2) {
        $fcs_bytes = 4;
    } else {
        $fcs_bytes = 8;
    }
    $pos += $fcs_bytes;   # skip FCS (we trust the blocks)

    # ── Blocks ───────────────────────────────────────────────────────────
    my @out;

    while (1) {
        die "zstd: truncated block header at pos $pos\n"
            if $pos + 3 > scalar @bytes;

        my $hdr  = $bytes[$pos] | ($bytes[$pos+1] << 8) | ($bytes[$pos+2] << 16);
        $pos    += 3;

        my $last  = $hdr & 1;
        my $btype = ($hdr >> 1) & 3;
        my $bsize = $hdr >> 3;

        if ($btype == 0) {
            # Raw block: $bsize verbatim bytes.
            die "zstd: raw block truncated\n" if $pos + $bsize > scalar @bytes;
            die "zstd: decompressed size exceeds limit\n"
                if @out + $bsize > MAX_OUTPUT;
            push @out, @bytes[$pos .. $pos + $bsize - 1];
            $pos += $bsize;

        } elsif ($btype == 1) {
            # RLE block: 1 byte repeated $bsize times.
            die "zstd: RLE block missing byte\n" if $pos >= scalar @bytes;
            die "zstd: decompressed size exceeds limit\n"
                if @out + $bsize > MAX_OUTPUT;
            my $byte = $bytes[$pos++];
            push @out, ($byte) x $bsize;

        } elsif ($btype == 2) {
            # Compressed block.
            die "zstd: compressed block truncated\n"
                if $pos + $bsize > scalar @bytes;
            my @block_data = @bytes[$pos .. $pos + $bsize - 1];
            $pos += $bsize;
            _decompress_block(\@block_data, \@out);
            die "zstd: decompressed size exceeds limit\n"
                if @out > MAX_OUTPUT;

        } else {
            die "zstd: reserved block type 3\n";
        }

        last if $last;
    }

    return pack('C*', @out);
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
