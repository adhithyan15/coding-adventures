use strict;
use warnings;
use Test2::V0;
use JSON::PP;
use File::Basename qw(dirname);
use File::Spec;
use Compress::Raw::Zlib;

use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecPNG qw(:all);

my $cases_file = File::Spec->catfile(
    dirname(__FILE__), '..', '..', '..', '..',
    'specs', 'fixtures', 'image-codec-png-v1', 'cases.json'
);

open my $fh, '<', $cases_file or die "Cannot open $cases_file: $!";
my $json_text = do { local $/; <$fh> };
close $fh;

my $data = decode_json($json_text);
my $cases = $data->{cases};
is(scalar(@$cases), 85, 'Found exactly 85 conformance cases');

for my $case (@$cases) {
    my $id = $case->{id};
    my $op = $case->{operation};

    if ($op eq 'decode') {
        my $png_bytes = pack('H*', $case->{png_hex});
        my %opts = $case->{options} ? %{$case->{options}} : ();
        my $decoded = decode_png($png_bytes, %opts);

        is($decoded->width, $case->{expected}{width}, "$id: width matches");
        is($decoded->height, $case->{expected}{height}, "$id: height matches");
        is(unpack('H*', ${ $decoded->data }), lc($case->{expected}{rgba_hex}), "$id: RGBA payload matches");
    } elsif ($op eq 'encode') {
        my $w = $case->{input}{width};
        my $h = $case->{input}{height};
        my $rgba_bytes = pack('H*', $case->{input}{rgba_hex});

        my $container = CodingAdventures::PixelContainer->new($w, $h);
        ${ $container->data } = $rgba_bytes;

        my $png_bytes = encode_png($container);
        ok(length($png_bytes) > 8, "$id: generated non-empty PNG bytes");

        # Roundtrip through decoder
        my $roundtrip = decode_png($png_bytes);
        is($roundtrip->width, $w, "$id: roundtrip width matches");
        is($roundtrip->height, $h, "$id: roundtrip height matches");
        is(unpack('H*', ${ $roundtrip->data }), lc($case->{input}{rgba_hex}), "$id: roundtrip RGBA matches");

        # Independent foreign oracle verification with Compress::Raw::Zlib
        # Extract IDAT chunk payload (slice out zlib stream)
        my $idat_pos = index($png_bytes, 'IDAT');
        ok($idat_pos >= 4, "$id: found IDAT chunk");
        my $idat_len = unpack('N', substr($png_bytes, $idat_pos - 4, 4));
        my $zlib_data = substr($png_bytes, $idat_pos + 4, $idat_len);

        my ($d, $status) = Compress::Raw::Zlib::Inflate->new();
        my $uncompressed = '';
        $status = $d->inflate($zlib_data, $uncompressed);
        ok($status == Z_STREAM_END, "$id: foreign Compress::Raw::Zlib inflates encoder output cleanly");
        is(length($uncompressed), $h * ($w * 4 + 1), "$id: uncompressed length matches expected scanlines");
    } elsif ($op eq 'adler32') {
        my $raw_data = pack('H*', $case->{input_hex});
        my $actual = adler32($raw_data);
        my $expected = hex($case->{expected}{adler32_hex});
        is($actual, $expected, "$id: adler32 matches fixture");
    } elsif ($op eq 'decode-error') {
        my $png_bytes = pack('H*', $case->{png_hex});
        my %opts = $case->{options} ? %{$case->{options}} : ();
        my $err;
        eval {
            decode_png($png_bytes, %opts);
            1;
        } or $err = $@;

        ok(defined $err && ref($err) && $err->can('code'), "$id: threw typed PngError");
        is($err->code, $case->{expected}{error_id}, "$id: error code matches expected taxonomy");
    } elsif ($op eq 'encode-error') {
        my $w = $case->{input}{width};
        my $h = $case->{input}{height};
        my $rgba_hex = $case->{input}{rgba_hex} // '';
        my $rgba_bytes = pack('H*', $rgba_hex);

        my $fake_container = bless {
            width  => $w,
            height => $h,
            data   => \$rgba_bytes,
        }, 'CodingAdventures::PixelContainer';

        my $err;
        eval {
            encode_png($fake_container);
            1;
        } or $err = $@;

        ok(defined $err && ref($err) && $err->can('code'), "$id: threw typed PngError");
        is($err->code, $case->{expected}{error_id}, "$id: error code matches expected taxonomy");
    } else {
        die "Unknown operation: $op in case $id";
    }
}

done_testing;
