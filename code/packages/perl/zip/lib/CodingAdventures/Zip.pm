package CodingAdventures::Zip;

use strict;
use warnings;

our $VERSION = '0.1.0';
use Exporter 'import';
our @EXPORT_OK = qw(
    crc32 dos_epoch dos_datetime
    new_writer add_file add_directory finish
    new_reader reader_entries reader_read read_by_name
    zip unzip
);

use CodingAdventures::LZSS qw(encode);

=head1 NAME

CodingAdventures::Zip - ZIP archive format (PKZIP 1989) from scratch — CMP09

=head1 SYNOPSIS

  use CodingAdventures::Zip qw(zip unzip);

  my $archive = zip([ ["hello.txt", "Hello, ZIP!"], ["data.bin", "\x01\x02\x03"] ]);
  my $files   = unzip($archive);
  print $files->{"hello.txt"};  # Hello, ZIP!

=head1 DESCRIPTION

ZIP bundles one or more files into a single C<.zip> archive, compressing each
entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
The same format underlies Java JARs, Office Open XML (.docx), Android APKs,
Python wheels, and many more.

=head2 Archive Layout

  [ Local File Header + File Data ]  <- entry 1
  [ Local File Header + File Data ]  <- entry 2
  ...
  ===== Central Directory =====
  [ Central Dir Header ]  <- entry 1 (contains local offset)
  [ Central Dir Header ]  <- entry 2
  [ End of Central Directory Record ]

The dual-header design enables two workflows:

=over 4

=item * B<Sequential write>: append Local Headers, write CD at the end.

=item * B<Random-access read>: seek to EOCD at the end, read CD, jump to any entry.

=back

=head2 DEFLATE inside ZIP

ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper. This implementation
uses fixed Huffman blocks (BTYPE=01) with the LZSS module for LZ77 match-finding
(32 KB window, max match 255, min match 3).

=head2 Series

  CMP02 (LZSS,    1982) — LZ77 + flag bits       <- dependency
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman         <- inlined here (raw RFC 1951)
  CMP09 (ZIP,     1989) — DEFLATE container      <- this module

=cut

# ============================================================================
# Utilities: little-endian byte packing
# ============================================================================

# le16 and le32 pack values as little-endian binary strings.
sub _le16 { pack('v', $_[0] & 0xFFFF) }
sub _le32 { pack('V', $_[0] & 0xFFFFFFFF) }

# _read_le16 and _read_le32 unpack from a binary string at a byte offset.
sub _read_le16 {
    my ($s, $pos) = @_;
    return undef if $pos + 1 > length($s);
    return unpack('v', substr($s, $pos, 2));
}

sub _read_le32 {
    my ($s, $pos) = @_;
    return undef if $pos + 3 > length($s);
    return unpack('V', substr($s, $pos, 4));
}

# ============================================================================
# CRC-32
# ============================================================================
#
# CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
# Table-driven: precompute 256 entries, then process one byte at a time.

my @CRC_TABLE = do {
    my @t;
    for my $i (0 .. 255) {
        my $c = $i;
        for (1 .. 8) {
            if ($c & 1) { $c = 0xEDB88320 ^ ($c >> 1) }
            else        { $c >>= 1 }
        }
        push @t, $c;
    }
    @t;
};

=head2 crc32($data, $initial)

Compute the CRC-32 of C<$data>. C<$initial> defaults to 0 and can be used
for chained (incremental) computation.

=cut

sub crc32 {
    my ($data, $initial) = @_;
    $initial //= 0;
    my $crc = $initial ^ 0xFFFFFFFF;
    for my $byte (unpack 'C*', $data) {
        $crc = $CRC_TABLE[($crc ^ $byte) & 0xFF] ^ ($crc >> 8);
    }
    return $crc ^ 0xFFFFFFFF;
}

# ============================================================================
# MS-DOS Date / Time Encoding
# ============================================================================
#
# ZIP stores timestamps in MS-DOS packed format:
#   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
#   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
# Combined 32-bit: (date << 16) | time.

=head2 dos_epoch()

Returns 0x00210000 — the DOS representation of 1980-01-01 00:00:00.

=cut

sub dos_epoch { 0x00210000 }

=head2 dos_datetime($year, $month, $day, $hour, $minute, $second)

Encode a date/time as a 32-bit MS-DOS timestamp.

=cut

sub dos_datetime {
    my ($year, $month, $day, $hour, $minute, $second) = @_;
    $hour   //= 0;
    $minute //= 0;
    $second //= 0;
    my $t = ($hour << 11) | ($minute << 5) | int($second / 2);
    my $d = (($year - 1980) << 9) | ($month << 5) | $day;
    return (($d & 0xFFFF) << 16) | ($t & 0xFFFF);
}

# ============================================================================
# RFC 1951 DEFLATE — Bit I/O
# ============================================================================
#
# RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent MSB-first
# logically, so we bit-reverse them before writing LSB-first.

# BitWriter: integer buffer for LSB-first bit accumulation.
sub _bw_new { { buf => 0, bits => 0, out => [] } }

sub _bw_write_lsb {
    my ($bw, $value, $nbits) = @_;
    $bw->{buf} |= ($value & ((1 << $nbits) - 1)) << $bw->{bits};
    $bw->{bits} += $nbits;
    while ($bw->{bits} >= 8) {
        push @{$bw->{out}}, $bw->{buf} & 0xFF;
        $bw->{buf}  >>= 8;
        $bw->{bits}  -= 8;
    }
}

sub _bw_write_huffman {
    my ($bw, $code, $nbits) = @_;
    # Bit-reverse the code before writing LSB-first.
    my $rev = 0;
    my $c   = $code;
    for (1 .. $nbits) {
        $rev = ($rev << 1) | ($c & 1);
        $c >>= 1;
    }
    _bw_write_lsb($bw, $rev, $nbits);
}

sub _bw_align {
    my ($bw) = @_;
    if ($bw->{bits} > 0) {
        push @{$bw->{out}}, $bw->{buf} & 0xFF;
        $bw->{buf}  = 0;
        $bw->{bits} = 0;
    }
}

sub _bw_finish {
    my ($bw) = @_;
    _bw_align($bw);
    return pack('C*', @{$bw->{out}});
}

# BitReader: read bits LSB-first from a binary string.
sub _br_new {
    my ($data) = @_;
    return { data => $data, pos => 0, buf => 0, bits => 0 };
}

sub _br_fill {
    my ($br, $need) = @_;
    while ($br->{bits} < $need) {
        return 0 if $br->{pos} >= length($br->{data});
        $br->{buf} |= unpack('C', substr($br->{data}, $br->{pos}, 1)) << $br->{bits};
        $br->{pos}++;
        $br->{bits} += 8;
    }
    return 1;
}

sub _br_read_lsb {
    my ($br, $nbits) = @_;
    return 0 if $nbits == 0;
    return undef unless _br_fill($br, $nbits);
    my $mask = (1 << $nbits) - 1;
    my $val  = $br->{buf} & $mask;
    $br->{buf}  >>= $nbits;
    $br->{bits}  -= $nbits;
    return $val;
}

sub _br_read_msb {
    my ($br, $nbits) = @_;
    my $v = _br_read_lsb($br, $nbits);
    return undef unless defined $v;
    my $rev = 0;
    for (1 .. $nbits) {
        $rev = ($rev << 1) | ($v & 1);
        $v >>= 1;
    }
    return $rev;
}

sub _br_align {
    my ($br) = @_;
    my $discard = $br->{bits} % 8;
    if ($discard > 0) {
        $br->{buf}  >>= $discard;
        $br->{bits}  -= $discard;
    }
}

# ============================================================================
# RFC 1951 DEFLATE — Fixed Huffman Tables
# ============================================================================
#
# RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
# Using BTYPE=01 means we never transmit code tables.
#
# Literal/Length code lengths:
#   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
#   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
#   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
#   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
#
# Distance codes: 5-bit codes equal to the symbol number.

sub _fixed_ll_encode {
    my ($sym) = @_;
    if    ($sym <= 143) { return (0x30 + $sym,          8) }
    elsif ($sym <= 255) { return (0x190 + ($sym - 144), 9) }
    elsif ($sym <= 279) { return ($sym - 256,            7) }
    else                { return (0xC0 + ($sym - 280),  8) }
}

# _fixed_ll_decode reads one LL symbol from the BitReader using fixed Huffman.
# Returns undef on truncated input.
sub _fixed_ll_decode {
    my ($br) = @_;
    my $v7 = _br_read_msb($br, 7);
    return undef unless defined $v7;
    if ($v7 <= 23) {
        return $v7 + 256;  # 7-bit codes: symbols 256-279
    }
    my $b1 = _br_read_lsb($br, 1);
    return undef unless defined $b1;
    my $v8 = ($v7 << 1) | $b1;
    if ($v8 >= 48 && $v8 <= 191) {
        return $v8 - 48;         # literals 0-143
    } elsif ($v8 >= 192 && $v8 <= 199) {
        return $v8 + 88;         # symbols 280-287
    } else {
        my $b2 = _br_read_lsb($br, 1);
        return undef unless defined $b2;
        my $v9 = ($v8 << 1) | $b2;
        if ($v9 >= 400 && $v9 <= 511) {
            return $v9 - 256;    # literals 144-255
        }
        return undef;
    }
}

# ============================================================================
# RFC 1951 DEFLATE — Length / Distance Tables
# ============================================================================
#
# Match lengths (3-258) map to LL symbols 257-285 + extra bits.
# Match distances (1-32768) map to distance codes 0-29 + extra bits.
# RFC 1951 §3.2.5: symbol 285 = length 258, 0 extra bits (special case).

# Each entry: [base_length, extra_bits]
my @LENGTH_TABLE = (
    [3,0],[4,0],[5,0],[6,0],[7,0],[8,0],[9,0],[10,0],    # 257-264
    [11,1],[13,1],[15,1],[17,1],                           # 265-268
    [19,2],[23,2],[27,2],[31,2],                           # 269-272
    [35,3],[43,3],[51,3],[59,3],                           # 273-276
    [67,4],[83,4],[99,4],[115,4],                          # 277-280
    [131,5],[163,5],[195,5],[227,5],                       # 281-284
    [258,0],                                               # 285
);

# Each entry: [base_dist, extra_bits]
my @DIST_TABLE = (
    [1,0],[2,0],[3,0],[4,0],
    [5,1],[7,1],[9,2],[13,2],
    [17,3],[25,3],[33,4],[49,4],
    [65,5],[97,5],[129,6],[193,6],
    [257,7],[385,7],[513,8],[769,8],
    [1025,9],[1537,9],[2049,10],[3073,10],
    [4097,11],[6145,11],[8193,12],[12289,12],
    [16385,13],[24577,13],
);

# _encode_length returns (sym, base, extra_bits) for a match length 3-258.
sub _encode_length {
    my ($length) = @_;
    for my $i (reverse 0 .. $#LENGTH_TABLE) {
        if ($length >= $LENGTH_TABLE[$i][0]) {
            return (257 + $i, $LENGTH_TABLE[$i][0], $LENGTH_TABLE[$i][1]);
        }
    }
    die "encode_length: unreachable for length=$length";
}

# _encode_dist returns (code, base, extra_bits) for a match offset 1-32768.
sub _encode_dist {
    my ($offset) = @_;
    for my $i (reverse 0 .. $#DIST_TABLE) {
        if ($offset >= $DIST_TABLE[$i][0]) {
            return ($i, $DIST_TABLE[$i][0], $DIST_TABLE[$i][1]);
        }
    }
    die "encode_dist: unreachable for offset=$offset";
}

# ============================================================================
# RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
# ============================================================================

use constant MAX_OUTPUT => 256 * 1024 * 1024;  # 256 MiB zip-bomb guard

sub _deflate_compress {
    my ($data) = @_;
    my $bw = _bw_new();

    if (length($data) == 0) {
        # Empty stored block: BFINAL=1, BTYPE=00, LEN=0, NLEN=0xFFFF
        _bw_write_lsb($bw, 1, 1);      # BFINAL=1
        _bw_write_lsb($bw, 0, 2);      # BTYPE=00
        _bw_align($bw);
        _bw_write_lsb($bw, 0x0000, 16);
        _bw_write_lsb($bw, 0xFFFF, 16);
        return _bw_finish($bw);
    }

    # Tokenize with LZSS (window=32768, max=255, min=3).
    # encode() takes a string (it unpacks internally).
    my @tokens = encode($data, 32768, 255, 3);

    # Block header: BFINAL=1, BTYPE=01 (fixed Huffman).
    _bw_write_lsb($bw, 1, 1);  # BFINAL
    _bw_write_lsb($bw, 1, 1);  # BTYPE bit 0
    _bw_write_lsb($bw, 0, 1);  # BTYPE bit 1

    for my $tok (@tokens) {
        if ($tok->{kind} eq 'literal') {
            my ($code, $nbits) = _fixed_ll_encode($tok->{byte});
            _bw_write_huffman($bw, $code, $nbits);
        } else {
            # length symbol
            my ($sym, $base_len, $extra_len_bits) = _encode_length($tok->{length});
            my ($ll_code, $ll_nbits) = _fixed_ll_encode($sym);
            _bw_write_huffman($bw, $ll_code, $ll_nbits);
            _bw_write_lsb($bw, $tok->{length} - $base_len, $extra_len_bits)
                if $extra_len_bits > 0;
            # distance
            my ($dist_code, $base_dist, $extra_dist_bits) = _encode_dist($tok->{offset});
            _bw_write_huffman($bw, $dist_code, 5);
            _bw_write_lsb($bw, $tok->{offset} - $base_dist, $extra_dist_bits)
                if $extra_dist_bits > 0;
        }
    }

    # End-of-block symbol (256).
    my ($eob_code, $eob_nbits) = _fixed_ll_encode(256);
    _bw_write_huffman($bw, $eob_code, $eob_nbits);

    return _bw_finish($bw);
}

# ============================================================================
# RFC 1951 DEFLATE — Decompress
# ============================================================================
#
# Handles BTYPE=00 (stored) and BTYPE=01 (fixed Huffman).

# _deflate_decompress decompresses raw RFC 1951 DEFLATE data.
# Returns (data, undef) on success, or (undef, $errmsg) on failure.
sub _deflate_decompress {
    my ($data) = @_;
    my $br  = _br_new($data);
    my @out;  # byte array for O(1) back-reference indexing

    while (1) {
        my $bfinal = _br_read_lsb($br, 1);
        return (undef, "deflate: unexpected EOF reading BFINAL") unless defined $bfinal;
        my $btype = _br_read_lsb($br, 2);
        return (undef, "deflate: unexpected EOF reading BTYPE") unless defined $btype;

        if ($btype == 0) {
            # Stored block
            _br_align($br);
            my $len16  = _br_read_lsb($br, 16);
            my $nlen16 = _br_read_lsb($br, 16);
            return (undef, "deflate: EOF reading stored LEN/NLEN")
                unless defined $len16 && defined $nlen16;
            return (undef, "deflate: stored block LEN/NLEN mismatch")
                if ($nlen16 ^ 0xFFFF) != $len16;
            return (undef, "deflate: output size limit exceeded")
                if scalar(@out) + $len16 > MAX_OUTPUT;
            for (1 .. $len16) {
                my $b = _br_read_lsb($br, 8);
                return (undef, "deflate: EOF inside stored block") unless defined $b;
                push @out, $b;
            }

        } elsif ($btype == 1) {
            # Fixed Huffman block
            while (1) {
                my $sym = _fixed_ll_decode($br);
                return (undef, "deflate: EOF decoding symbol") unless defined $sym;

                if ($sym < 256) {
                    return (undef, "deflate: output size limit exceeded")
                        if scalar(@out) >= MAX_OUTPUT;
                    push @out, $sym;
                } elsif ($sym == 256) {
                    last;  # end-of-block
                } elsif ($sym >= 257 && $sym <= 285) {
                    my $idx = $sym - 257;
                    return (undef, "deflate: invalid length sym $sym")
                        if $idx > $#LENGTH_TABLE;
                    my ($base_len, $extra_len_bits) = @{$LENGTH_TABLE[$idx]};
                    my $extra_len = _br_read_lsb($br, $extra_len_bits);
                    return (undef, "deflate: EOF reading length extra bits")
                        unless defined $extra_len;
                    my $length = $base_len + $extra_len;

                    my $dist_code = _br_read_msb($br, 5);
                    return (undef, "deflate: EOF reading distance code")
                        unless defined $dist_code;
                    return (undef, "deflate: invalid distance code $dist_code")
                        if $dist_code >= scalar(@DIST_TABLE);
                    my ($base_dist, $extra_dist_bits) = @{$DIST_TABLE[$dist_code]};
                    my $extra_dist = _br_read_lsb($br, $extra_dist_bits);
                    return (undef, "deflate: EOF reading distance extra bits")
                        unless defined $extra_dist;
                    my $offset = $base_dist + $extra_dist;

                    my $n = scalar(@out);
                    return (undef, "deflate: output size limit exceeded")
                        if $n + $length >= MAX_OUTPUT;
                    return (undef, "deflate: back-reference offset $offset > output len $n")
                        if $offset > $n;
                    # Copy byte-by-byte using integer array for O(1) indexing.
                    for (1 .. $length) {
                        push @out, $out[$n - $offset];
                        $n++;
                    }
                } else {
                    return (undef, "deflate: invalid LL symbol $sym");
                }
            }

        } elsif ($btype == 2) {
            return (undef, "deflate: dynamic Huffman blocks (BTYPE=10) not supported");
        } else {
            return (undef, "deflate: reserved BTYPE=11");
        }

        last if $bfinal;
    }

    return (pack('C*', @out), undef);
}

# ============================================================================
# Entry name validation
# ============================================================================

sub _validate_entry_name {
    my ($name) = @_;
    return (undef, "zip: entry name contains null byte")
        if index($name, "\0") >= 0;
    return (undef, "zip: entry name contains backslash")
        if index($name, "\\") >= 0;
    return (undef, "zip: entry name is an absolute path: $name")
        if substr($name, 0, 1) eq '/';
    for my $seg (split m{/}, $name) {
        return (undef, "zip: entry name contains path traversal (..): $name")
            if $seg eq '..';
        # Three dots resolve like ".." on some Windows versions.
        return (undef, "zip: entry name contains three-dot segment (...): $name")
            if $seg eq '...';
        # Windows drive prefix (e.g. "C:") can redirect extraction outside target dir.
        return (undef, "zip: entry name contains Windows drive prefix: $name")
            if $seg =~ /\A[A-Za-z]:/;
    }
    return (1, undef);
}

# ============================================================================
# ZIP Write — ZipWriter
# ============================================================================
#
# ZipWriter accumulates entries: for each file it writes a Local File Header
# + data, records CD metadata, and assembles the full archive on finish().
#
# Auto-compression policy:
#   - Try DEFLATE. Use method=8 only if compressed < original.
#   - Otherwise use method=0 (Stored).

=head2 new_writer()

Creates a new ZipWriter hashref.

=cut

sub new_writer {
    return { buf => [], entries => [] };
}

=head2 add_file($writer, $name, $data, $compress)

Add a file entry. C<$compress> defaults to 1 (true).
Dies on invalid entry names (path traversal, etc.).

=cut

sub add_file {
    my ($writer, $name, $data, $compress) = @_;
    $compress //= 1;
    _add_entry($writer, $name, $data, $compress, 0x81A4);  # 0o100644
}

=head2 add_directory($writer, $name)

Add a directory entry. C<$name> should end with '/'.
Dies on invalid entry names.

=cut

sub add_directory {
    my ($writer, $name) = @_;
    _add_entry($writer, $name, '', 0, 0x41ED);  # 0o040755
}

sub _add_entry {
    my ($writer, $name, $data, $compress, $unix_mode) = @_;

    # Validate on the write path — refuse to produce archives with
    # path-traversal names that other tools might extract unsafely.
    my ($ok, $err) = _validate_entry_name($name);
    die $err unless $ok;

    # Also refuse if the archive already has 65535 entries (ZIP16 limit).
    die "zip: too many entries (max 65535)"
        if scalar(@{$writer->{entries}}) >= 65535;

    my $checksum         = crc32($data);
    my $uncompressed_size = length($data);

    my ($method, $file_data);
    if ($compress && $uncompressed_size > 0) {
        my $compressed = _deflate_compress($data);
        if (length($compressed) < $uncompressed_size) {
            $method    = 8;
            $file_data = $compressed;
        } else {
            $method    = 0;
            $file_data = $data;
        }
    } else {
        $method    = 0;
        $file_data = $data;
    }

    my $compressed_size = length($file_data);
    my $local_offset = 0;
    for my $s (@{$writer->{buf}}) { $local_offset += length($s) }
    my $version_needed = ($method == 8) ? 20 : 10;

    # Local File Header
    push @{$writer->{buf}},
        _le32(0x04034B50),          # signature
        _le16($version_needed),
        _le16(0x0800),              # flags (UTF-8)
        _le16($method),
        _le16(dos_epoch() & 0xFFFF),          # mod_time
        _le16((dos_epoch() >> 16) & 0xFFFF),  # mod_date
        _le32($checksum),
        _le32($compressed_size),
        _le32($uncompressed_size),
        _le16(length($name)),
        _le16(0),                   # extra_len = 0
        $name,
        $file_data;

    push @{$writer->{entries}}, {
        name              => $name,
        method            => $method,
        crc               => $checksum,
        compressed_size   => $compressed_size,
        uncompressed_size => $uncompressed_size,
        local_offset      => $local_offset,
        external_attrs    => ($unix_mode << 16),
    };
}

=head2 finish($writer)

Return the completed archive as a binary string.

=cut

sub finish {
    my ($writer) = @_;

    # Current buffer length = cd_offset.
    my $cd_offset = 0;
    for my $s (@{$writer->{buf}}) { $cd_offset += length($s) }

    # Central Directory
    my @cd_parts;
    for my $e (@{$writer->{entries}}) {
        my $version_needed = ($e->{method} == 8) ? 20 : 10;
        push @cd_parts,
            _le32(0x02014B50),          # CD signature
            _le16(0x031E),              # version made by
            _le16($version_needed),
            _le16(0x0800),              # flags (UTF-8)
            _le16($e->{method}),
            _le16(dos_epoch() & 0xFFFF),
            _le16((dos_epoch() >> 16) & 0xFFFF),
            _le32($e->{crc}),
            _le32($e->{compressed_size}),
            _le32($e->{uncompressed_size}),
            _le16(length($e->{name})),
            _le16(0),                   # extra_len
            _le16(0),                   # comment_len
            _le16(0),                   # disk_start
            _le16(0),                   # internal_attrs
            _le32($e->{external_attrs}),
            _le32($e->{local_offset}),
            $e->{name};
    }
    my $cd_str  = join('', @cd_parts);
    my $cd_size = length($cd_str);
    my $num     = scalar @{$writer->{entries}};

    my $eocd = join('',
        _le32(0x06054B50),  # EOCD signature
        _le16(0),           # disk_number
        _le16(0),           # cd_disk
        _le16($num),        # entries this disk
        _le16($num),        # entries total
        _le32($cd_size),
        _le32($cd_offset),
        _le16(0),           # comment_len
    );

    return join('', @{$writer->{buf}}, $cd_str, $eocd);
}

# ============================================================================
# ZIP Read — ZipReader
# ============================================================================
#
# ZipReader uses the "EOCD-first" strategy:
#   1. Scan backwards for EOCD signature.
#   2. Read CD offset and size.
#   3. Parse all CD headers.
#   4. On reader_read(entry): skip local header, decompress, verify CRC-32.

sub _find_eocd {
    my ($data) = @_;
    my $min_size    = 22;
    my $max_comment = 65535;
    return undef if length($data) < $min_size;

    my $scan_start = length($data) - $min_size - $max_comment;
    $scan_start = 0 if $scan_start < 0;

    for my $i (reverse $scan_start .. length($data) - $min_size) {
        my $sig = _read_le32($data, $i);
        next unless defined $sig && $sig == 0x06054B50;
        my $comment_len = _read_le16($data, $i + 20);
        next unless defined $comment_len;
        return $i if $i + $min_size + $comment_len == length($data);
    }
    return undef;
}

=head2 new_reader($data)

Parse a ZIP archive binary string. Returns a reader hashref on success,
or dies with an error message on failure.

=cut

sub new_reader {
    my ($data) = @_;

    my $eocd_pos = _find_eocd($data);
    die "zip: no End of Central Directory record found\n"
        unless defined $eocd_pos;

    my $cd_size   = _read_le32($data, $eocd_pos + 12);
    my $cd_offset = _read_le32($data, $eocd_pos + 16);
    die "zip: EOCD too short\n"
        unless defined $cd_size && defined $cd_offset;
    # Guard against sign-extended values (ZIP64 not supported).
    die "zip: EOCD cd_size or cd_offset has high bit set (>2 GiB); ZIP64 not supported\n"
        if $cd_size > 0x7FFFFFFF || $cd_offset > 0x7FFFFFFF;
    die "zip: Central Directory out of bounds\n"
        if $cd_offset + $cd_size > length($data);

    my @entries;
    my $pos = $cd_offset;

    while ($pos + 3 < $cd_offset + $cd_size) {
        my $sig = _read_le32($data, $pos);
        last unless defined $sig && $sig == 0x02014B50;

        die "zip: CD entry header out of bounds\n"
            if $pos + 46 > length($data);

        my $method        = _read_le16($data, $pos + 10);
        my $crc           = _read_le32($data, $pos + 16);
        my $compressed_sz = _read_le32($data, $pos + 20);
        my $size          = _read_le32($data, $pos + 24);
        my $name_len      = _read_le16($data, $pos + 28);
        my $extra_len     = _read_le16($data, $pos + 30);
        my $comment_len   = _read_le16($data, $pos + 32);
        my $local_offset  = _read_le32($data, $pos + 42);

        die "zip: CD entry fields truncated\n"
            unless defined($method) && defined($crc) && defined($compressed_sz)
                && defined($size) && defined($name_len) && defined($extra_len)
                && defined($comment_len) && defined($local_offset);
        die "zip: CD entry has >2 GiB field; ZIP64 not supported\n"
            if $compressed_sz > 0x7FFFFFFF || $size > 0x7FFFFFFF
            || $local_offset > 0x7FFFFFFF;

        my $name_start = $pos + 46;
        my $name_end   = $name_start + $name_len;
        die "zip: CD entry name out of bounds\n"
            if $name_end > length($data);

        my $name = substr($data, $name_start, $name_len);
        my ($ok, $err) = _validate_entry_name($name);
        die "$err\n" unless $ok;

        my $next_pos = $name_end + $extra_len + $comment_len;
        die "zip: CD entry advance out of bounds\n"
            if $next_pos > $cd_offset + $cd_size;

        push @entries, {
            name            => $name,
            size            => $size,
            compressed_size => $compressed_sz,
            method          => $method,
            crc32           => $crc,
            is_directory    => (substr($name, -1) eq '/') ? 1 : 0,
            local_offset    => $local_offset,
        };
        $pos = $next_pos;
    }

    return { data => $data, entries => \@entries, cd_offset => $cd_offset };
}

=head2 reader_entries($reader)

Returns an arrayref of all entry hashrefs in the archive.

=cut

sub reader_entries {
    my ($reader) = @_;
    return $reader->{entries};
}

=head2 reader_read($reader, $entry)

Decompress and CRC-validate one entry. Returns the data string on success,
or dies with an error message on failure.

=cut

sub reader_read {
    my ($reader, $entry) = @_;
    return '' if $entry->{is_directory};

    my $data   = $reader->{data};
    my $lh_off = $entry->{local_offset};

    # Validate that local_offset points into the local-file region, not the CD.
    die "zip: local header offset for '$entry->{name}' points into Central Directory\n"
        if defined $reader->{cd_offset} && $lh_off >= $reader->{cd_offset};

    # Check local flags (encryption bit).
    my $local_flags = _read_le16($data, $lh_off + 6);
    die "zip: local header out of bounds for '$entry->{name}'\n"
        unless defined $local_flags;
    die "zip: entry '$entry->{name}' is encrypted\n"
        if $local_flags & 1;

    my $lh_name_len  = _read_le16($data, $lh_off + 26);
    my $lh_extra_len = _read_le16($data, $lh_off + 28);
    die "zip: local header fields out of bounds for '$entry->{name}'\n"
        unless defined($lh_name_len) && defined($lh_extra_len);

    my $data_start = $lh_off + 30 + $lh_name_len + $lh_extra_len;
    # Ensure header fields don't advance into the Central Directory.
    die "zip: local header fields advance into Central Directory for '$entry->{name}'\n"
        if defined $reader->{cd_offset} && $data_start > $reader->{cd_offset};

    my $data_end = $data_start + $entry->{compressed_size};
    die "zip: entry '$entry->{name}' data out of bounds\n"
        if $data_end > length($data);

    my $compressed = substr($data, $data_start, $entry->{compressed_size});

    my $decompressed;
    if ($entry->{method} == 0) {
        $decompressed = $compressed;
    } elsif ($entry->{method} == 8) {
        my ($result, $err) = _deflate_decompress($compressed);
        die "zip: entry '$entry->{name}': $err\n" unless defined $result;
        $decompressed = $result;
    } else {
        die "zip: unsupported compression method $entry->{method} for '$entry->{name}'\n";
    }

    # Trim to declared uncompressed size.
    if (length($decompressed) > $entry->{size}) {
        $decompressed = substr($decompressed, 0, $entry->{size});
    }

    # Verify CRC-32.
    my $actual_crc = crc32($decompressed);
    die sprintf(
        "zip: CRC-32 mismatch for '%s': expected %08X, got %08X\n",
        $entry->{name}, $entry->{crc32}, $actual_crc
    ) if $actual_crc != $entry->{crc32};

    return $decompressed;
}

=head2 read_by_name($reader, $name)

Convenience wrapper: find an entry by name and return its data.
Dies if the entry is not found or fails CRC validation.

=cut

sub read_by_name {
    my ($reader, $name) = @_;
    for my $entry (@{$reader->{entries}}) {
        if ($entry->{name} eq $name) {
            return reader_read($reader, $entry);
        }
    }
    die "zip: entry '$name' not found\n";
}

# ============================================================================
# Convenience Functions
# ============================================================================

=head2 zip($entries, $compress)

One-shot compress. C<$entries> is an arrayref of C<[$name, $data]> pairs.
C<$compress> defaults to 1.

Returns a binary ZIP archive string.

=cut

sub zip {
    my ($entries, $compress) = @_;
    $compress //= 1;
    my $w = new_writer();
    for my $e (@$entries) {
        add_file($w, $e->[0], $e->[1], $compress);
    }
    return finish($w);
}

=head2 unzip($data)

One-shot decompress. Returns a hashref mapping name to data.
Dies on CRC mismatch, unsupported method, corrupt data, or duplicate entry names.

=cut

sub unzip {
    my ($data) = @_;
    my $reader = new_reader($data);
    my %result;
    for my $entry (@{$reader->{entries}}) {
        next if $entry->{is_directory};
        die "zip: duplicate entry name '$entry->{name}'\n"
            if exists $result{$entry->{name}};
        $result{$entry->{name}} = reader_read($reader, $entry);
    }
    return \%result;
}

1;

__END__

=head1 SEE ALSO

L<CodingAdventures::LZSS>, L<CodingAdventures::Deflate>

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
