package CodingAdventures::LZW;

# =============================================================================
# CodingAdventures::LZW
# =============================================================================
#
# LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is LZW?
# ------------
#
# LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
# added before encoding begins (codes 0-255). This eliminates LZ78's mandatory
# next_char byte -- every symbol is already in the dictionary, so the encoder
# can emit pure codes.
#
# With only codes to transmit, LZW uses variable-width bit-packing: codes start
# at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
#
# Reserved Codes
# --------------
#
#   0-255:  Pre-seeded single-byte entries.
#   256:    CLEAR_CODE -- reset to initial 256-entry state.
#   257:    STOP_CODE  -- end of code stream.
#   258+:   Dynamically added entries.
#
# Wire Format (CMP03)
# -------------------
#
#   Bytes 0-3:  original_length (big-endian uint32)
#   Bytes 4+:   bit-packed variable-width codes, LSB-first
#
# The Tricky Token
# ----------------
#
# During decoding the decoder may receive code C == next_code (not yet added).
# This happens when the input has the form xyx...x. The fix:
#
#   entry = dec_dict[prev_code] . chr(ord(substr(dec_dict[prev_code],0,1)))
#
# The Series: CMP00 -> CMP05
# --------------------------
#
#   CMP00 (LZ77,    1977) -- Sliding-window backreferences.
#   CMP01 (LZ78,    1978) -- Explicit dictionary (trie).
#   CMP02 (LZSS,    1982) -- LZ77 + flag bits; no wasted literals.
#   CMP03 (LZW,     1984) -- LZ78 + pre-initialized dict; GIF. (this module)
#   CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
#   CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib.
# =============================================================================

use strict;
use warnings;
use utf8;

our $VERSION = '0.1.0';

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

use constant CLEAR_CODE        => 256;
use constant STOP_CODE         => 257;
use constant INITIAL_NEXT_CODE => 258;
use constant INITIAL_CODE_SIZE => 9;
use constant MAX_CODE_SIZE     => 16;

# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

# A BitWriter is a hash ref:
#   { buf => 0, bit_pos => 0, bytes => [] }
# Bits accumulate in `buf` (integer), spilling into `bytes` LSB-first.

sub _bw_new {
    return { buf => 0, bit_pos => 0, bytes => [] };
}

sub _bw_write {
    my ($w, $code, $code_size) = @_;
    $w->{buf}    += $code * (2 ** $w->{bit_pos});
    $w->{bit_pos} += $code_size;
    while ($w->{bit_pos} >= 8) {
        push @{ $w->{bytes} }, $w->{buf} % 256;
        $w->{buf}     = int($w->{buf} / 256);
        $w->{bit_pos} -= 8;
    }
}

sub _bw_flush {
    my ($w) = @_;
    if ($w->{bit_pos} > 0) {
        push @{ $w->{bytes} }, $w->{buf} % 256;
        $w->{buf}     = 0;
        $w->{bit_pos} = 0;
    }
}

sub _bw_to_string {
    my ($w) = @_;
    return pack('C*', @{ $w->{bytes} });
}

# A BitReader is a hash ref:
#   { data => $str, pos => 0, buf => 0, bit_pos => 0 }

sub _br_new {
    my ($data) = @_;
    return { data => $data, pos => 0, buf => 0, bit_pos => 0 };
}

sub _br_read {
    my ($r, $code_size) = @_;
    while ($r->{bit_pos} < $code_size) {
        return undef if $r->{pos} >= length($r->{data});
        my $byte = ord(substr($r->{data}, $r->{pos}, 1));
        $r->{buf}    += $byte * (2 ** $r->{bit_pos});
        $r->{pos}    += 1;
        $r->{bit_pos} += 8;
    }
    my $mask = (2 ** $code_size) - 1;
    my $code = int($r->{buf}) & $mask;
    $r->{buf}     = int($r->{buf} / (2 ** $code_size));
    $r->{bit_pos} -= $code_size;
    return $code;
}

sub _br_exhausted {
    my ($r) = @_;
    return $r->{pos} >= length($r->{data}) && $r->{bit_pos} == 0;
}

# ---------------------------------------------------------------------------
# Encoder
# ---------------------------------------------------------------------------

sub encode_codes {
    my ($data) = @_;
    my $original_length = length($data);
    my %enc_dict;
    for my $b (0..255) {
        $enc_dict{ chr($b) } = $b;
    }

    my $next_code   = INITIAL_NEXT_CODE;
    my $max_entries = 2 ** MAX_CODE_SIZE;
    my @codes       = (CLEAR_CODE);
    my $w           = '';

    for my $i (0 .. $original_length - 1) {
        my $byte = substr($data, $i, 1);
        my $wb   = $w . $byte;
        if (exists $enc_dict{$wb}) {
            $w = $wb;
        } else {
            push @codes, $enc_dict{$w};

            if ($next_code < $max_entries) {
                $enc_dict{$wb} = $next_code;
                $next_code++;
            } elsif ($next_code == $max_entries) {
                push @codes, CLEAR_CODE;
                %enc_dict = ();
                for my $b (0..255) { $enc_dict{ chr($b) } = $b; }
                $next_code = INITIAL_NEXT_CODE;
            }

            $w = $byte;
        }
    }

    push @codes, $enc_dict{$w} if length($w) > 0;
    push @codes, STOP_CODE;

    return (\@codes, $original_length);
}

# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------

sub decode_codes {
    my ($codes_ref) = @_;
    my @dec_dict;
    for my $b (0..255) { $dec_dict[$b] = chr($b); }
    $dec_dict[CLEAR_CODE] = '';
    $dec_dict[STOP_CODE]  = '';

    my $next_code   = INITIAL_NEXT_CODE;
    my $max_entries = 2 ** MAX_CODE_SIZE;
    my @output;
    my $prev_code = undef;

    for my $code (@$codes_ref) {
        if ($code == CLEAR_CODE) {
            @dec_dict = ();
            for my $b (0..255) { $dec_dict[$b] = chr($b); }
            $dec_dict[CLEAR_CODE] = '';
            $dec_dict[STOP_CODE]  = '';
            $next_code = INITIAL_NEXT_CODE;
            $prev_code = undef;
            next;
        }

        last if $code == STOP_CODE;

        my $entry;
        if ($code < scalar @dec_dict && defined $dec_dict[$code]) {
            $entry = $dec_dict[$code];
        } elsif ($code == $next_code && defined $prev_code) {
            # Tricky token.
            my $prev_entry = $dec_dict[$prev_code];
            $entry = $prev_entry . substr($prev_entry, 0, 1);
        } else {
            next;  # invalid -- skip
        }

        push @output, $entry;

        if (defined $prev_code && $next_code < $max_entries) {
            my $prev_entry = $dec_dict[$prev_code];
            $dec_dict[$next_code] = $prev_entry . substr($entry, 0, 1);
            $next_code++;
        }

        $prev_code = $code;
    }

    return join('', @output);
}

# ---------------------------------------------------------------------------
# Serialisation
# ---------------------------------------------------------------------------

sub pack_codes {
    my ($codes_ref, $original_length) = @_;
    my $writer    = _bw_new();
    my $code_size = INITIAL_CODE_SIZE;
    my $next_code = INITIAL_NEXT_CODE;
    my $max       = 2 ** MAX_CODE_SIZE;

    for my $code (@$codes_ref) {
        _bw_write($writer, $code, $code_size);

        if ($code == CLEAR_CODE) {
            $code_size = INITIAL_CODE_SIZE;
            $next_code = INITIAL_NEXT_CODE;
        } elsif ($code != STOP_CODE) {
            if ($next_code < $max) {
                $next_code++;
                if ($next_code > (2 ** $code_size) && $code_size < MAX_CODE_SIZE) {
                    $code_size++;
                }
            }
        }
    }
    _bw_flush($writer);

    my $body   = _bw_to_string($writer);
    my $header = pack('N', $original_length);
    return $header . $body;
}

sub unpack_codes {
    my ($data) = @_;
    return ([CLEAR_CODE, STOP_CODE], 0) if length($data) < 4;

    my ($original_length) = unpack('N', substr($data, 0, 4));
    my $reader    = _br_new(substr($data, 4));
    my @codes;
    my $code_size = INITIAL_CODE_SIZE;
    my $next_code = INITIAL_NEXT_CODE;
    my $max       = 2 ** MAX_CODE_SIZE;

    while (!_br_exhausted($reader)) {
        my $code = _br_read($reader, $code_size);
        last unless defined $code;

        push @codes, $code;

        if ($code == STOP_CODE) {
            last;
        } elsif ($code == CLEAR_CODE) {
            $code_size = INITIAL_CODE_SIZE;
            $next_code = INITIAL_NEXT_CODE;
        } elsif ($next_code < $max) {
            $next_code++;
            if ($next_code > (2 ** $code_size) && $code_size < MAX_CODE_SIZE) {
                $code_size++;
            }
        }
    }

    return (\@codes, $original_length);
}

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

sub compress {
    my ($data) = @_;
    my ($codes_ref, $original_length) = encode_codes($data);
    return pack_codes($codes_ref, $original_length);
}

sub decompress {
    my ($data) = @_;
    my ($codes_ref, $original_length) = unpack_codes($data);
    my $result = decode_codes($codes_ref);
    return substr($result, 0, $original_length);
}

1;
