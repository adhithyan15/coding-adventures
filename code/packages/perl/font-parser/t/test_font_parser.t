use strict;
use warnings;
use utf8;
use Test2::V0;
use File::Spec;
use File::Basename qw(dirname);
use Cwd qw(abs_path);

# Resolve paths relative to the test file's own directory, not the CWD,
# so the tests work whether run via `prove` or directly.
my $TEST_DIR = dirname(abs_path(__FILE__));

use lib File::Spec->catdir($TEST_DIR, '..', 'lib');
use CodingAdventures::FontParser qw(load font_metrics glyph_id glyph_metrics kerning);

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

# From t/ → font-parser/ → perl/ → packages/ → code/ → code/fixtures/
my $FONT_PATH = File::Spec->catfile(
    $TEST_DIR, '..', '..', '..', '..', 'fixtures', 'fonts', 'Inter-Regular.ttf'
);

sub inter_bytes {
    open my $fh, '<:raw', $FONT_PATH or die "Cannot open font: $!";
    local $/;
    my $data = <$fh>;
    close $fh;
    return $data;
}

# Build a minimal valid OpenType binary with a kern Format 0 table.
# $pairs is an arrayref of [$left, $right, $value] triples.
sub build_synthetic_font {
    my ($pairs) = @_;
    $pairs //= [];

    # Packing helpers (all big-endian)
    my $w16  = sub { pack('n',  $_[0] & 0xFFFF) };
    my $wi16 = sub { pack('s>', $_[0]) };
    my $w32  = sub { pack('N',  $_[0] & 0xFFFFFFFF) };
    my $tag  = sub { substr($_[0] . "\x00\x00\x00\x00", 0, 4) };

    my $num_tables = 6;
    my $dir_size   = 12 + $num_tables * 16;

    my $head_len = 54;
    my $hhea_len = 36;
    my $maxp_len = 6;
    my $cmap_len = 36;
    my $hmtx_len = 5 * 4;
    my $n_pairs  = scalar @$pairs;
    my $kern_len = 4 + 6 + 8 + $n_pairs * 6;

    my $head_off = $dir_size;
    my $hhea_off = $head_off + $head_len;
    my $maxp_off = $hhea_off + $hhea_len;
    my $cmap_off = $maxp_off + $maxp_len;
    my $hmtx_off = $cmap_off + $cmap_len;
    my $kern_off = $hmtx_off + $hmtx_len;

    my $buf = '';

    # Offset table
    $buf .= $w32->(0x00010000) . $w16->($num_tables) . $w16->(64) . $w16->(2) . $w16->(32);

    # Table records (sorted: cmap < head < hhea < hmtx < kern < maxp)
    for my $r (
        [$tag->('cmap'), $cmap_off, $cmap_len],
        [$tag->('head'), $head_off, $head_len],
        [$tag->('hhea'), $hhea_off, $hhea_len],
        [$tag->('hmtx'), $hmtx_off, $hmtx_len],
        [$tag->('kern'), $kern_off, $kern_len],
        [$tag->('maxp'), $maxp_off, $maxp_len],
    ) {
        $buf .= $r->[0] . $w32->(0) . $w32->($r->[1]) . $w32->($r->[2]);
    }

    # head (54 bytes)
    $buf .= $w32->(0x00010000) . $w32->(0x00010000) . $w32->(0) . $w32->(0x5F0F3CF5);
    $buf .= $w16->(0) . $w16->(1000) . "\x00" x 16;
    $buf .= $wi16->(0) x 4;                  # xMin yMin xMax yMax
    $buf .= $w16->(0) . $w16->(8) . $wi16->(2) . $wi16->(0) . $wi16->(0);

    # hhea (36 bytes)
    $buf .= $w32->(0x00010000) . $wi16->(800) . $wi16->(-200) . $wi16->(0);
    $buf .= $w16->(1000);
    $buf .= $wi16->(0) x 3;                  # minLSB, minRSB, xMaxExtent
    $buf .= $wi16->(1) . $wi16->(0) . $wi16->(0);   # caret slope + offset
    $buf .= $wi16->(0) x 4;                  # reserved
    $buf .= $wi16->(0) . $w16->(5);          # metricDataFormat, numberOfHMetrics

    # maxp (6 bytes)
    $buf .= $w32->(0x00005000) . $w16->(5);

    # cmap
    $buf .= $w16->(0) . $w16->(1);
    $buf .= $w16->(3) . $w16->(1) . $w32->(12);
    $buf .= $w16->(4) . $w16->(24) . $w16->(0);
    $buf .= $w16->(2) . $w16->(2) . $w16->(0) . $w16->(0);
    $buf .= $w16->(0xFFFF);
    $buf .= $w16->(0);
    $buf .= $w16->(0xFFFF);
    $buf .= $wi16->(1);
    $buf .= $w16->(0);

    # hmtx: 5 records {600, 50}
    $buf .= ($w16->(600) . $wi16->(50)) x 5;

    # kern table
    my $sub_len = 6 + 8 + $n_pairs * 6;
    my @sorted = sort { $a->[0] * 65536 + $a->[1] <=> $b->[0] * 65536 + $b->[1] } @$pairs;

    $buf .= $w16->(0) . $w16->(1);
    $buf .= $w16->(0) . $w16->($sub_len) . $w16->(0x0001);
    $buf .= $w16->($n_pairs) . $w16->(0) . $w16->(0) . $w16->(0);

    for my $p (@sorted) {
        $buf .= $w16->($p->[0]) . $w16->($p->[1]) . $wi16->($p->[2]);
    }

    return $buf;
}

# ─────────────────────────────────────────────────────────────────────────────
# Tests: load
# ─────────────────────────────────────────────────────────────────────────────

subtest 'load — empty string raises BufferTooShort' => sub {
    my $err = dies { load('') };
    ok $err, 'load("") dies';
    is $err->{kind}, 'BufferTooShort', 'kind is BufferTooShort';
};

subtest 'load — wrong magic raises InvalidMagic' => sub {
    my $buf = pack('N', 0xDEADBEEF) . "\x00" x 252;
    my $err = dies { load($buf) };
    is $err->{kind}, 'InvalidMagic', 'kind is InvalidMagic';
};

subtest 'load — numTables=0 raises TableNotFound' => sub {
    my $buf = pack('N', 0x00010000) . pack('n', 0) . "\x00" x 6;
    my $err = dies { load($buf) };
    is $err->{kind}, 'TableNotFound', 'kind is TableNotFound';
};

subtest 'load — Inter Regular loads without error' => sub {
    my $font = load(inter_bytes());
    ok defined $font, 'font defined';
    is $font->{_type}, 'FontFile', 'type is FontFile';
};

subtest 'load — synthetic font loads without error' => sub {
    my $font = load(build_synthetic_font([[1, 2, -140]]));
    ok defined $font, 'font defined';
};

# ─────────────────────────────────────────────────────────────────────────────
# Tests: font_metrics
# ─────────────────────────────────────────────────────────────────────────────

my $inter_font = load(inter_bytes());

subtest 'font_metrics — units_per_em is 2048' => sub {
    is font_metrics($inter_font)->{units_per_em}, 2048;
};

subtest 'font_metrics — family_name is Inter' => sub {
    is font_metrics($inter_font)->{family_name}, 'Inter';
};

subtest 'font_metrics — subfamily_name is Regular' => sub {
    is font_metrics($inter_font)->{subfamily_name}, 'Regular';
};

subtest 'font_metrics — ascender is positive' => sub {
    ok font_metrics($inter_font)->{ascender} > 0, 'ascender > 0';
};

subtest 'font_metrics — descender is non-positive' => sub {
    ok font_metrics($inter_font)->{descender} <= 0, 'descender <= 0';
};

subtest 'font_metrics — num_glyphs is large' => sub {
    ok font_metrics($inter_font)->{num_glyphs} > 100, 'num_glyphs > 100';
};

subtest 'font_metrics — x_height is positive' => sub {
    my $m = font_metrics($inter_font);
    ok defined $m->{x_height}, 'x_height defined';
    ok $m->{x_height} > 0, 'x_height > 0';
};

subtest 'font_metrics — cap_height is positive' => sub {
    my $m = font_metrics($inter_font);
    ok defined $m->{cap_height}, 'cap_height defined';
    ok $m->{cap_height} > 0, 'cap_height > 0';
};

subtest 'font_metrics — synthetic font units_per_em is 1000' => sub {
    my $f = load(build_synthetic_font([]));
    is font_metrics($f)->{units_per_em}, 1000;
};

subtest 'font_metrics — synthetic font family_name is (unknown)' => sub {
    my $f = load(build_synthetic_font([]));
    is font_metrics($f)->{family_name}, '(unknown)';
};

# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_id
# ─────────────────────────────────────────────────────────────────────────────

subtest "glyph_id — 'A' (0x0041) is non-nil" => sub {
    ok defined glyph_id($inter_font, 0x0041), 'glyph_id(A) defined';
};

subtest "glyph_id — 'V' (0x0056) is non-nil" => sub {
    ok defined glyph_id($inter_font, 0x0056), 'glyph_id(V) defined';
};

subtest "glyph_id — space (0x0020) is non-nil" => sub {
    ok defined glyph_id($inter_font, 0x0020), 'glyph_id(space) defined';
};

subtest 'glyph_id — A and V differ' => sub {
    my $gid_a = glyph_id($inter_font, 0x0041);
    my $gid_v = glyph_id($inter_font, 0x0056);
    isnt $gid_a, $gid_v, 'A and V have different glyph IDs';
};

subtest 'glyph_id — codepoint above 0xFFFF returns undef' => sub {
    is glyph_id($inter_font, 0x10000), undef;
};

subtest 'glyph_id — negative codepoint returns undef' => sub {
    is glyph_id($inter_font, -1), undef;
};

# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_metrics
# ─────────────────────────────────────────────────────────────────────────────

subtest "glyph_metrics — advance_width for 'A' is positive" => sub {
    my $gid = glyph_id($inter_font, 0x0041);
    my $gm  = glyph_metrics($inter_font, $gid);
    ok defined $gm, 'gm defined';
    ok $gm->{advance_width} > 0, 'advance_width > 0';
};

subtest 'glyph_metrics — advance_width in reasonable range' => sub {
    my $gid = glyph_id($inter_font, 0x0041);
    my $gm  = glyph_metrics($inter_font, $gid);
    ok $gm->{advance_width} >= 100 && $gm->{advance_width} <= 2400, 'in range';
};

subtest 'glyph_metrics — out-of-range glyph returns undef' => sub {
    my $ng = font_metrics($inter_font)->{num_glyphs};
    is glyph_metrics($inter_font, $ng), undef;
};

subtest 'glyph_metrics — negative glyph_id returns undef' => sub {
    is glyph_metrics($inter_font, -1), undef;
};

# ─────────────────────────────────────────────────────────────────────────────
# Tests: kerning
# ─────────────────────────────────────────────────────────────────────────────

subtest 'kerning — Inter A+V returns 0 (Inter uses GPOS not kern)' => sub {
    my $gid_a = glyph_id($inter_font, 0x0041);
    my $gid_v = glyph_id($inter_font, 0x0056);
    is kerning($inter_font, $gid_a, $gid_v), 0;
};

subtest 'kerning — synthetic pair (1,2) returns -140' => sub {
    my $f = load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]));
    is kerning($f, 1, 2), -140;
};

subtest 'kerning — synthetic pair (3,4) returns 80' => sub {
    my $f = load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]));
    is kerning($f, 3, 4), 80;
};

subtest 'kerning — absent pair returns 0' => sub {
    my $f = load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]));
    is kerning($f, 1, 4), 0;
};

subtest 'kerning — reversed pair returns 0' => sub {
    my $f = load(build_synthetic_font([[1, 2, -140]]));
    is kerning($f, 2, 1), 0;
};

done_testing;
