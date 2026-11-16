use strict;
use warnings;
use Test2::V0;
use JSON::PP;
use Compress::Raw::Zlib qw(adler32);

use CodingAdventures::Zip qw(raw_deflate);
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecPNG qw(:all);
use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Filter;
use CodingAdventures::ImageCodecPNG::Chunk;
use CodingAdventures::ImageCodecPNG::Adler32;

# 1. Codec OO Adapter Interface
subtest 'Codec OO Adapter' => sub {
    my $codec = CodingAdventures::ImageCodecPNG::Codec->new();
    is($codec->mime_type, 'image/png', 'mime_type is image/png');
    is($codec->max_pixels, 33_554_432, 'default max_pixels matches');

    my $container = CodingAdventures::PixelContainer->new(2, 2);
    $container->set_pixel(0, 0, 255, 0, 0, 255);
    $container->set_pixel(1, 1, 0, 255, 0, 255);

    my $encoded = $codec->encode($container);
    ok(length($encoded) > 0, 'encode produces bytes');

    my $decoded = $codec->decode($encoded);
    is($decoded->width, 2, 'decoded width matches');
    is($decoded->height, 2, 'decoded height matches');
    is(unpack('H*', ${ $decoded->data }), unpack('H*', ${ $container->data }), 'decoded RGBA matches');

    # Bounded codec limit enforcement
    my $bounded = CodingAdventures::ImageCodecPNG::Codec->new(max_pixels => 10);
    is($bounded->max_pixels, 10, 'custom max_pixels matches');

    my $err;
    my $res;
    eval { $res = $bounded->decode($encoded); 1; } or $err = $@;
    ok(!defined($err) && defined($res), 'small image within 10 pixels passes'); # 2x2 = 4 <= 10 pixels

    my $large = CodingAdventures::PixelContainer->new(4, 4); # 16 pixels > 10
    my $large_png = encode_png($large);
    $err = undef;
    eval { $bounded->decode($large_png); 1; } or $err = $@;
    ok(defined $err && ref($err) && $err->can('code'), 'exceeding max_pixels throws PngError');
    is($err->code, 'pixel-limit', 'error code is pixel-limit');
};

# 2. Filter Edge Cases and Paeth Predictor Tie-Breaking
subtest 'Filter Paeth Tie-Breaking and Filter Types' => sub {
    # Normative tie-breaking order: a, then b, then c.
    # Case 1: pa == pb == pc -> returns a
    is(CodingAdventures::ImageCodecPNG::Filter::paeth(10, 10, 10), 10, 'pa == pb == pc returns a');

    # Case 2: pa == pb < pc -> returns a
    # a=10, b=10, c=20 => p=0. pa=10, pb=10, pc=20 => returns a (10)
    is(CodingAdventures::ImageCodecPNG::Filter::paeth(10, 10, 20), 10, 'pa == pb < pc returns a');

    # Case 3: pb < pa and pb <= pc -> returns b
    # a=10, b=20, c=0 => p=30. pa=20, pb=10, pc=30 => returns b (20)
    is(CodingAdventures::ImageCodecPNG::Filter::paeth(10, 20, 0), 20, 'pb < pa and pb <= pc returns b');
    is(CodingAdventures::ImageCodecPNG::Filter::paeth(30, 10, 25), 10, 'paeth pb <= pc returns b');

    # Case 4: pc < pa and pc < pb -> returns c
    # a=10, b=30, c=20 => p=20. pa=10, pb=10, pc=0 => returns c (20)
    is(CodingAdventures::ImageCodecPNG::Filter::paeth(10, 30, 20), 20, 'pc < pa and pc < pb returns c');

    # apply_filter across all 5 filter IDs
    my $raw = [10, 20, 30, 40];
    my $prior = [5, 10, 15, 20];
    my $out = [];
    CodingAdventures::ImageCodecPNG::Filter::apply_filter(FILTER_NONE, $raw, $prior, 4, $out);
    is($out, [10, 20, 30, 40], 'apply_filter NONE');
    CodingAdventures::ImageCodecPNG::Filter::apply_filter(FILTER_SUB, $raw, $prior, 4, $out);
    is($out, [10, 20, 30, 40], 'apply_filter SUB');
    CodingAdventures::ImageCodecPNG::Filter::apply_filter(FILTER_UP, $raw, $prior, 4, $out);
    is($out, [5, 10, 15, 20], 'apply_filter UP');
    CodingAdventures::ImageCodecPNG::Filter::apply_filter(FILTER_AVERAGE, $raw, $prior, 4, $out);
    CodingAdventures::ImageCodecPNG::Filter::apply_filter(FILTER_PAETH, $raw, $prior, 4, $out);

    # undo_filter across all 5 filter IDs
    my $r;
    $r = [10, 20, 30, 40]; CodingAdventures::ImageCodecPNG::Filter::undo_filter(FILTER_NONE, $r, $prior, 4);
    $r = [10, 20, 30, 40]; CodingAdventures::ImageCodecPNG::Filter::undo_filter(FILTER_SUB, $r, $prior, 4);
    $r = [10, 20, 30, 40]; CodingAdventures::ImageCodecPNG::Filter::undo_filter(FILTER_UP, $r, $prior, 4);
    $r = [10, 20, 30, 40]; CodingAdventures::ImageCodecPNG::Filter::undo_filter(FILTER_AVERAGE, $r, $prior, 4);
    $r = [10, 20, 30, 40]; CodingAdventures::ImageCodecPNG::Filter::undo_filter(FILTER_PAETH, $r, $prior, 4);

    # Invalid filter ID in undo_filter
    my @row = (1, 2, 3, 4);
    my @prior_row = (0, 0, 0, 0);
    my $err;
    eval { CodingAdventures::ImageCodecPNG::Filter::undo_filter(99, \@row, \@prior_row, 4); 1; } or $err = $@;
    ok(defined $err && ref($err) && $err->can('code'), 'undo_filter with 99 throws');
    is($err->code, 'invalid-filter', 'invalid filter throws invalid-filter');
};

# 3. Untrusted Type Guards and Validation
subtest 'Type Guards & Validations' => sub {
    # decode_png with undef / bad types
    my $err;
    eval { decode_png(undef); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(undef) throws file-too-short');

    $err = undef;
    eval { decode_png({}); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(hashref) throws file-too-short');

    # decode_png with wide character that cannot be downgraded
    $err = undef;
    eval { decode_png("\x{100}"); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(wide utf8 string) throws file-too-short');

    # options as hashref
    my $custom_codec = codec(max_pixels => 1000);
    is($custom_codec->max_pixels, 1000, 'codec(%opts) returns configured instance');

    $err = undef;
    eval { decode_png([256, 0]); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(out-of-range byte array) throws file-too-short');

    # Pure strings and non-integers in byte array rejected
    $err = undef;
    eval { decode_png(['250', 0]); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(string in byte array) throws file-too-short');

    $err = undef;
    eval { decode_png([JSON::PP::true, 0]); 1; } or $err = $@;
    is($err->code, 'file-too-short', 'decode_png(boolean in byte array) throws file-too-short');

    # Byte array decode
    my $sample = CodingAdventures::PixelContainer->new(1, 1);
    $sample->set_pixel(0, 0, 10, 20, 30, 255);
    my $sample_png = encode_png($sample);

    # decode with options as hashref
    my $from_hashref = decode_png($sample_png, { max_pixels => 100 });
    is($from_hashref->width, 1, 'decode_png with options hashref width');

    my @byte_array = unpack('C*', $sample_png);
    my $from_array = decode_png(\@byte_array);
    is($from_array->width, 1, 'byte array decode width');
    is(unpack('H*', ${ $from_array->data }), unpack('H*', ${ $sample->data }), 'byte array decode data matches');

    # encode_png guards
    $err = undef;
    eval { encode_png(undef); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'encode_png(undef) throws invalid-image-dimensions');

    $err = undef;
    eval { encode_png("not-a-blessed-container"); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'encode_png(string) throws invalid-image-dimensions');

    # Invalid dimensions
    my $fake = bless { width => -1, height => 10, data => \('x' x 40) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($fake); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'negative width throws');

    my $fake_float = bless { width => 1.5, height => 10, data => \('x' x 60) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($fake_float); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'float width throws');

    my $fake_str = bless { width => '10', height => 10, data => \('x' x 400) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($fake_str); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'string width throws');

    my $fake_edge = bless { width => 16_385, height => 1, data => \('x' x 65540) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($fake_edge); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'dimension > 16384 throws');

    my $fake_product = bless { width => 10_000, height => 4_000, data => \('x' x 160000000) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($fake_product); 1; } or $err = $@;
    is($err->code, 'invalid-image-dimensions', 'product > 33554432 throws');

    # Buffer length mismatch
    my $bad_buf = bless { width => 2, height => 2, data => \('x' x 15) }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($bad_buf); 1; } or $err = $@;
    is($err->code, 'invalid-pixel-data-length', 'buffer length mismatch throws');

    # Non-scalar ref data
    my $bad_ref = bless { width => 2, height => 2, data => 'not-a-ref' }, 'CodingAdventures::PixelContainer';
    $err = undef;
    eval { encode_png($bad_ref); 1; } or $err = $@;
    is($err->code, 'invalid-pixel-data-length', 'non-ref data throws');
};

# 4. Multi-Chunk Adler-32 Cross-Chunk Boundary (> 5552 bytes)
subtest 'Adler-32 Large Buffer' => sub {
    is(adler32('Wikipedia'), 0x11E60398, 'RFC vector Wikipedia matches');
    is(adler32(''), 1, 'Empty string Adler-32 is 1');

    my $large = '';
    for (my $i = 0; $i < 12_000; $i++) {
        $large .= chr(($i * 31) & 0xFF);
    }
    my $expected = Compress::Raw::Zlib::adler32($large);
    is(adler32($large), $expected, 'Multi-chunk Adler-32 matches foreign zlib oracle');

    my $utf8_str = 'Wikipedia';
    utf8::upgrade($utf8_str);
    is(adler32($utf8_str), 0x11E60398, 'Adler-32 safely handles upgraded UTF-8 string');
};

# 5. Deterministic Property Round-Trips
subtest 'Property Round-Trips' => sub {
    for my $dim ([1, 1], [3, 7], [8, 8], [16, 5]) {
        my ($w, $h) = @$dim;
        my $c = CodingAdventures::PixelContainer->new($w, $h);
        for (my $y = 0; $y < $h; $y++) {
            for (my $x = 0; $x < $w; $x++) {
                $c->set_pixel($x, $y, ($x * 17) & 0xFF, ($y * 23) & 0xFF, (($x + $y) * 29) & 0xFF, 255);
            }
        }

        my $png = encode_png($c);
        my $out = decode_png($png);
        is($out->width, $w, "Roundtrip width $w");
        is($out->height, $h, "Roundtrip height $h");
        is(unpack('H*', ${ $out->data }), unpack('H*', ${ $c->data }), "Roundtrip payload matches for ${w}x${h}");
    }
};

sub make_custom_png {
    my ($w, $h, $col_type, $scanlines, @extra_chunks) = @_;
    my $ihdr = CodingAdventures::ImageCodecPNG::Chunk::format('IHDR', pack('NNCCCCC', $w, $h, 8, $col_type, 0, 0, 0));
    my $deflated = CodingAdventures::Zip::raw_deflate($scanlines);
    my $adler = CodingAdventures::ImageCodecPNG::Adler32::compute($scanlines);
    my $zlib = pack('CC', 0x78, 0x01) . $deflated . pack('N', $adler);
    my $idat = CodingAdventures::ImageCodecPNG::Chunk::format('IDAT', $zlib);
    my $iend = CodingAdventures::ImageCodecPNG::Chunk::format('IEND', '');

    my $png = PNG_SIGNATURE() . $ihdr;
    for my $c (@extra_chunks) {
        $png .= $c;
    }
    $png .= $idat . $iend;
    return $png;
}

# 6. Decoder Format Matrix & Chunk Branches
subtest 'Decoder Format Matrix & Chunk Branches' => sub {
    # Grey + Alpha (colour type 4): 2 channels per pixel
    my $grey_alpha_png = make_custom_png(1, 1, 4, "\x00\x80\xFF");
    my $dec_ga = decode_png($grey_alpha_png);
    is($dec_ga->width, 1, 'Grey+Alpha width');
    is([unpack('CCCC', ${ $dec_ga->data })], [128, 128, 128, 255], 'Grey+Alpha pixel RGBA matches');

    # Greyscale (colour type 0) with tRNS chunk
    my $trns_grey = CodingAdventures::ImageCodecPNG::Chunk::format('tRNS', pack('n', 64));
    my $grey_trns_png = make_custom_png(1, 1, 0, "\x00\x40", $trns_grey);
    my $dec_gt = decode_png($grey_trns_png);
    is([unpack('CCCC', ${ $dec_gt->data })], [64, 64, 64, 0], 'Greyscale transparent tRNS pixel matches');

    # RGB (colour type 2) with tRNS chunk
    my $trns_rgb = CodingAdventures::ImageCodecPNG::Chunk::format('tRNS', pack('nnn', 10, 20, 30));
    my $rgb_trns_png = make_custom_png(1, 1, 2, "\x00\x0A\x14\x1E", $trns_rgb);
    my $dec_rt = decode_png($rgb_trns_png);
    is([unpack('CCCC', ${ $dec_rt->data })], [10, 20, 30, 0], 'RGB transparent tRNS pixel matches');

    # RGB (colour type 2) with valid PLTE chunk
    my $plte = CodingAdventures::ImageCodecPNG::Chunk::format('PLTE', "\xFF\x00\x00");
    my $rgb_plte_png = make_custom_png(1, 1, 2, "\x00\xFF\x00\x00", $plte);
    my $dec_plte = decode_png($rgb_plte_png);
    is([unpack('CCCC', ${ $dec_plte->data })], [255, 0, 0, 255], 'RGB with PLTE decodes cleanly');

    # Animation chunks (acTL, fcTL, fdAT) throw unsupported-feature
    for my $type ('acTL', 'fcTL', 'fdAT') {
        my $chunk = CodingAdventures::ImageCodecPNG::Chunk::format($type, 'dummy');
        my $apng = make_custom_png(1, 1, 6, "\x00\x00\x00\x00\xFF", $chunk);
        my $err;
        eval { decode_png($apng); 1; } or $err = $@;
        ok(defined $err && ref($err) && $err->can('code'), "$type throws");
        is($err->code, 'unsupported-feature', "$type throws unsupported-feature");
    }

    # Unknown critical chunk throws unknown-critical-chunk
    my $bad_crit = CodingAdventures::ImageCodecPNG::Chunk::format('QXYZ', 'payload');
    my $png_crit = make_custom_png(1, 1, 6, "\x00\x00\x00\x00\xFF", $bad_crit);
    my $err;
    eval { decode_png($png_crit); 1; } or $err = $@;
    ok(defined $err && ref($err) && $err->can('code'), 'Unknown critical chunk throws');
    is($err->code, 'unknown-critical-chunk', 'Unknown critical chunk throws unknown-critical-chunk');
};

done_testing;
