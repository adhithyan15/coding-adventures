package CodingAdventures::FontParser;

# Metrics-only OpenType/TrueType font parser.  Zero CPAN dependencies.
#
# OpenType and TrueType files are binary table databases.  The first 12 bytes
# (the "offset table") name the font format and count the tables.  Starting at
# byte 12, an array of 16-byte table records (tag + checksum + offset + length)
# lets us locate any table by its 4-byte ASCII tag.
#
# All multi-byte integers in the file are BIG-ENDIAN.  Perl's unpack() with
# templates n (u16), N (u32), and s> (i16) handle this natively.
#
# Tables parsed:
#   head  - unitsPerEm
#   hhea  - ascender, descender, lineGap, numberOfHMetrics
#   maxp  - numGlyphs
#   cmap  - Format 4, Unicode BMP → glyph index
#   hmtx  - advance width + left-side bearing per glyph
#   kern  - Format 0 sorted pairs (optional)
#   name  - family / subfamily names, UTF-16 BE (optional)
#   OS/2  - xHeight, capHeight (optional, version ≥ 2)

use 5.026;
use strict;
use warnings;
use utf8;
use Encode qw(decode);

our $VERSION = '0.1.0';

# ─────────────────────────────────────────────────────────────────────────────
# Error type
# ─────────────────────────────────────────────────────────────────────────────
#
# We use die() with a hash reference so callers can inspect $err->{kind}.

package CodingAdventures::FontParser::FontError;

sub new {
    my ($class, %args) = @_;
    return bless {
        kind    => $args{kind}    // 'ParseError',
        message => $args{message} // $args{kind} // 'ParseError',
    }, $class;
}

sub kind    { $_[0]->{kind}    }
sub message { $_[0]->{message} }
sub throw   { die $_[0]->new(@_[1..$#_]) }

package CodingAdventures::FontParser;

# Shorthand for throwing a FontError.
sub _err {
    my ($kind, $msg) = @_;
    die CodingAdventures::FontParser::FontError->new(kind => $kind, message => $msg // $kind);
}

# ─────────────────────────────────────────────────────────────────────────────
# Binary read helpers — all big-endian, 0-based offsets
# ─────────────────────────────────────────────────────────────────────────────

# unpack() uses 1-based positions? No — the offset parameter is byte-based,
# and the string is 0-indexed when we use substr().  We define helpers that
# accept 0-based offsets for consistency with all other implementations.

sub _ru8  { unpack 'C',  substr($_[0], $_[1], 1) }
sub _ru16 { unpack 'n',  substr($_[0], $_[1], 2) }
sub _ri16 { unpack 's>', substr($_[0], $_[1], 2) }
sub _ru32 { unpack 'N',  substr($_[0], $_[1], 4) }

# ─────────────────────────────────────────────────────────────────────────────
# Offset table + table records
# ─────────────────────────────────────────────────────────────────────────────

sub _parse_offset_table {
    my ($data) = @_;

    _err('BufferTooShort') if length($data) < 12;

    my $sfnt_ver = _ru32($data, 0);
    _err('InvalidMagic') unless $sfnt_ver == 0x00010000 || $sfnt_ver == 0x4F54544F;

    my $num_tables = _ru16($data, 4);
    my %tables;

    for my $i (0 .. $num_tables - 1) {
        my $base = 12 + $i * 16;
        my $tag  = substr($data, $base, 4);
        my $off  = _ru32($data, $base + 8);
        my $len  = _ru32($data, $base + 12);
        $tables{$tag} = { offset => $off, length => $len };
    }

    return %tables;
}

sub _require_table {
    my ($tables, $tag) = @_;
    _err('TableNotFound', "required table '$tag' not found")
        unless exists $tables->{$tag};
    return $tables->{$tag}{offset}, $tables->{$tag}{length};
}

# ─────────────────────────────────────────────────────────────────────────────
# head table — unitsPerEm at offset 18
# ─────────────────────────────────────────────────────────────────────────────

sub _parse_head {
    my ($data, $tables) = @_;
    my ($off) = _require_table($tables, 'head');
    return { units_per_em => _ru16($data, $off + 18) };
}

# ─────────────────────────────────────────────────────────────────────────────
# hhea table
# ─────────────────────────────────────────────────────────────────────────────
#
# Fixed(4)  version
# i16       ascender      offset 4
# i16       descender     offset 6
# i16       lineGap       offset 8
# ...
# u16       numberOfHMetrics  offset 34

sub _parse_hhea {
    my ($data, $tables) = @_;
    my ($off) = _require_table($tables, 'hhea');
    return {
        ascender      => _ri16($data, $off + 4),
        descender     => _ri16($data, $off + 6),
        line_gap      => _ri16($data, $off + 8),
        num_h_metrics => _ru16($data, $off + 34),
    };
}

# ─────────────────────────────────────────────────────────────────────────────
# maxp table — numGlyphs at offset 4
# ─────────────────────────────────────────────────────────────────────────────

sub _parse_maxp {
    my ($data, $tables) = @_;
    my ($off) = _require_table($tables, 'maxp');
    return _ru16($data, $off + 4);
}

# ─────────────────────────────────────────────────────────────────────────────
# cmap table — Format 4 BMP subtable
# ─────────────────────────────────────────────────────────────────────────────
#
# cmap header: version(2) + numSubtables(2)
# Encoding record (8 bytes each):
#   platformID  u16    3 = Windows
#   encodingID  u16    1 = Unicode BMP
#   offset      u32    relative to start of cmap table
#
# Format 4 layout:
#   0    format          u16  = 4
#   6    segCountX2      u16
#   14   endCode[n]           2n bytes
#   16+2n startCode[n]        2n bytes
#   16+4n idDelta[n]          2n bytes  (signed)
#   16+6n idRangeOffset[n]    2n bytes

sub _parse_cmap {
    my ($data, $tables) = @_;
    my ($cmap_off) = _require_table($tables, 'cmap');

    my $num_subtables = _ru16($data, $cmap_off + 2);
    my $sub_off;

    for my $i (0 .. $num_subtables - 1) {
        my $rec  = $cmap_off + 4 + $i * 8;
        my $plat = _ru16($data, $rec);
        my $enc  = _ru16($data, $rec + 2);
        my $rel  = _ru32($data, $rec + 4);
        if ($plat == 3 && $enc == 1) {
            $sub_off = $cmap_off + $rel;
            last;
        }
    }

    _err('TableNotFound', 'no cmap Format 4 subtable') unless defined $sub_off;
    _err('ParseError', 'expected cmap Format 4')       unless _ru16($data, $sub_off) == 4;

    my $seg_count            = _ru16($data, $sub_off + 6) >> 1;
    my $end_codes_base       = $sub_off + 14;
    my $start_codes_base     = $sub_off + 16 + $seg_count * 2;
    my $id_delta_base        = $sub_off + 16 + $seg_count * 4;
    my $id_range_offset_base = $sub_off + 16 + $seg_count * 6;

    my @segments;
    for my $i (0 .. $seg_count - 1) {
        push @segments, {
            end_code        => _ru16($data, $end_codes_base       + $i * 2),
            start_code      => _ru16($data, $start_codes_base     + $i * 2),
            id_delta        => _ri16($data, $id_delta_base        + $i * 2),
            id_range_offset => _ru16($data, $id_range_offset_base + $i * 2),
            iro_abs         => $id_range_offset_base + $i * 2,
        };
    }

    return \@segments;
}

# cmap_lookup: scan segments for a codepoint.
#
# The idRangeOffset self-relative pointer:
#   If id_range_offset == 0: glyph = (cp + id_delta) & 0xFFFF
#   Otherwise:  abs_off = iro_abs + id_range_offset + (cp - start_code) * 2
#               glyph   = ru16(data, abs_off)

sub _cmap_lookup {
    my ($segments, $data, $cp) = @_;

    for my $seg (@$segments) {
        next if $cp > $seg->{end_code};
        return undef if $cp < $seg->{start_code};

        my $gid;
        if ($seg->{id_range_offset} == 0) {
            $gid = ($cp + $seg->{id_delta}) & 0xFFFF;
        } else {
            my $abs_off = $seg->{iro_abs} + $seg->{id_range_offset}
                        + ($cp - $seg->{start_code}) * 2;
            $gid = _ru16($data, $abs_off);
        }

        return undef if $gid == 0;
        return $gid;
    }

    return undef;
}

# ─────────────────────────────────────────────────────────────────────────────
# hmtx table
# ─────────────────────────────────────────────────────────────────────────────
#
# numberOfHMetrics full records: advanceWidth(u16) + lsb(i16)
# Glyphs ≥ numberOfHMetrics share the last advanceWidth.

sub _hmtx_offset {
    my ($data, $tables) = @_;
    my ($off) = _require_table($tables, 'hmtx');
    _ru8($data, $off);  # sanity probe
    return $off;
}

sub _lookup_glyph_metrics {
    my ($font, $gid) = @_;
    return undef if $gid < 0 || $gid >= $font->{num_glyphs};

    my $nhm  = $font->{num_h_metrics};
    my $off  = $font->{hmtx_off};
    my $data = $font->{raw};

    my $metric_idx = $gid < $nhm ? $gid : $nhm - 1;
    my $advance    = _ru16($data, $off + $metric_idx * 4);

    my $lsb;
    if ($gid < $nhm) {
        $lsb = _ri16($data, $off + $gid * 4 + 2);
    } else {
        $lsb = _ri16($data, $off + $nhm * 4 + ($gid - $nhm) * 2);
    }

    return { advance_width => $advance, left_side_bearing => $lsb };
}

# ─────────────────────────────────────────────────────────────────────────────
# kern table — Format 0
# ─────────────────────────────────────────────────────────────────────────────
#
# kern header: version(u16) + nTables(u16)
# Subtable header (6 bytes): version(u16) + length(u16) + coverage(u16)
#   coverage HIGH byte = subtable format (0 = sorted pairs)
# Format 0 data (+6): nPairs(u16) + 3×u16 + nPairs×{left,right,value}
#
# Precompute a hash of (left * 65536 + right) => value.

sub _parse_kern {
    my ($data, $tables) = @_;
    return {} unless exists $tables->{'kern'};

    my $off      = $tables->{'kern'}{offset};
    my $n_tables = _ru16($data, $off + 2);
    my %kern_map;
    my $cur = $off + 4;

    for my $i (1 .. $n_tables) {
        my $sub_len  = _ru16($data, $cur + 2);
        my $coverage = _ru16($data, $cur + 4);
        my $fmt      = $coverage >> 8;   # format in HIGH byte

        if ($fmt == 0) {
            my $n_pairs    = _ru16($data, $cur + 6);
            my $pairs_base = $cur + 14;   # 6 (header) + 8 (Format 0 header)

            for my $j (0 .. $n_pairs - 1) {
                my $poff  = $pairs_base + $j * 6;
                my $left  = _ru16($data, $poff);
                my $right = _ru16($data, $poff + 2);
                my $value = _ri16($data, $poff + 4);
                $kern_map{$left * 65536 + $right} = $value;
            }
        }

        $cur += $sub_len;
    }

    return \%kern_map;
}

# ─────────────────────────────────────────────────────────────────────────────
# name table
# ─────────────────────────────────────────────────────────────────────────────
#
# name header: format(u16) + count(u16) + stringOffset(u16)
# Name record (12 bytes): platformID(u16) + encodingID(u16) + languageID(u16)
#   + nameID(u16) + length(u16) + offset(u16, relative to stringOffset)

sub _parse_name {
    my ($data, $tables) = @_;
    return ('(unknown)', '(unknown)') unless exists $tables->{'name'};

    my $tbl_off  = $tables->{'name'}{offset};
    my $count    = _ru16($data, $tbl_off + 2);
    my $str_base = $tbl_off + _ru16($data, $tbl_off + 4);

    my ($family, $subfamily);

    for my $i (0 .. $count - 1) {
        my $rec  = $tbl_off + 6 + $i * 12;
        my $plat = _ru16($data, $rec);
        my $enc  = _ru16($data, $rec + 2);
        my $nid  = _ru16($data, $rec + 6);
        my $nlen = _ru16($data, $rec + 8);
        my $noff = _ru16($data, $rec + 10);

        if ($plat == 3 && $enc == 1) {
            my $raw = substr($data, $str_base + $noff, $nlen);
            my $str = eval { decode('UTF-16BE', $raw, Encode::FB_CROAK()) };
            $str //= $raw;  # fallback to raw on decode error
            $family    = $str if $nid == 1 && !defined $family;
            $subfamily = $str if $nid == 2 && !defined $subfamily;
        }

        last if defined $family && defined $subfamily;
    }

    return ($family // '(unknown)', $subfamily // '(unknown)');
}

# ─────────────────────────────────────────────────────────────────────────────
# OS/2 table
# ─────────────────────────────────────────────────────────────────────────────
#
# sxHeight  at offset 86  (version ≥ 2)
# sCapHeight at offset 88 (version ≥ 2)

sub _parse_os2 {
    my ($data, $tables) = @_;
    return (undef, undef) unless exists $tables->{'OS/2'};

    my $off = $tables->{'OS/2'}{offset};
    my $len = $tables->{'OS/2'}{length};
    my $ver = _ru16($data, $off);

    return (undef, undef) unless $ver >= 2 && $len >= 90;
    return (_ri16($data, $off + 86), _ri16($data, $off + 88));
}

# ─────────────────────────────────────────────────────────────────────────────
# Public API
# ─────────────────────────────────────────────────────────────────────────────

=head1 NAME

CodingAdventures::FontParser - Metrics-only OpenType/TrueType font parser

=head1 SYNOPSIS

    use CodingAdventures::FontParser qw(load font_metrics glyph_id glyph_metrics kerning);

    open my $fh, '<:raw', 'Inter-Regular.ttf' or die $!;
    local $/;
    my $data = <$fh>;
    close $fh;

    my $font = load($data);
    my $m    = font_metrics($font);
    print $m->{units_per_em};   # 2048

=cut

use Exporter 'import';
our @EXPORT_OK = qw(load font_metrics glyph_id glyph_metrics kerning);

=head2 load($data)

Parse a binary font string. Dies with a C<CodingAdventures::FontParser::FontError>
object on failure. Check C<< $err->{kind} >> for the category:
C<BufferTooShort>, C<InvalidMagic>, C<TableNotFound>, C<ParseError>.

=cut

sub load {
    my ($data) = @_;

    my %tables = eval { _parse_offset_table($data) };
    if ($@) {
        die $@ if ref($@) && $@->isa('CodingAdventures::FontParser::FontError');
        _err('ParseError', "$@");
    }

    my $head_d = _parse_head($data, \%tables);
    my $hhea_d = _parse_hhea($data, \%tables);
    my $num_glyphs = _parse_maxp($data, \%tables);
    my $cmap_segs  = _parse_cmap($data, \%tables);
    my $hmtx_off   = _hmtx_offset($data, \%tables);

    my $kern_map = _parse_kern($data, \%tables);
    my ($family, $subfamily) = _parse_name($data, \%tables);
    my ($x_height, $cap_height) = _parse_os2($data, \%tables);

    my $metrics = {
        units_per_em   => $head_d->{units_per_em},
        ascender       => $hhea_d->{ascender},
        descender      => $hhea_d->{descender},
        line_gap       => $hhea_d->{line_gap},
        x_height       => $x_height,
        cap_height     => $cap_height,
        num_glyphs     => $num_glyphs,
        family_name    => $family,
        subfamily_name => $subfamily,
    };

    return {
        _type         => 'FontFile',
        raw           => $data,
        metrics       => $metrics,
        cmap_segments => $cmap_segs,
        num_h_metrics => $hhea_d->{num_h_metrics},
        num_glyphs    => $num_glyphs,
        hmtx_off      => $hmtx_off,
        kern_map      => $kern_map,
    };
}

=head2 font_metrics($font)

Return a hashref with C<units_per_em>, C<ascender>, C<descender>, C<line_gap>,
C<x_height> (undef if absent), C<cap_height> (undef if absent), C<num_glyphs>,
C<family_name>, C<subfamily_name>.

=cut

sub font_metrics { $_[0]->{metrics} }

=head2 glyph_id($font, $codepoint)

Map a Unicode codepoint to a glyph index. Returns C<undef> for out-of-BMP
codepoints, negative values, or unmapped codepoints.

=cut

sub glyph_id {
    my ($font, $cp) = @_;
    return undef unless defined $cp && $cp >= 0 && $cp <= 0xFFFF;
    return _cmap_lookup($font->{cmap_segments}, $font->{raw}, int($cp));
}

=head2 glyph_metrics($font, $glyph_id)

Return a hashref with C<advance_width> and C<left_side_bearing>, or C<undef>
for out-of-range glyph IDs.

=cut

sub glyph_metrics {
    my ($font, $gid) = @_;
    return undef unless defined $gid && $gid >= 0;
    return _lookup_glyph_metrics($font, int($gid));
}

=head2 kerning($font, $left, $right)

Return the kern value (integer, signed font units) for the ordered glyph pair,
or C<0> if the kern table is absent or the pair is not listed.

=cut

sub kerning {
    my ($font, $left, $right) = @_;
    return 0 unless defined $left && defined $right;
    return $font->{kern_map}{$left * 65536 + $right} // 0;
}

1;
