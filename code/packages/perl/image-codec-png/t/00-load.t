use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../lib";
use lib "$FindBin::Bin/../../pixel-container/lib";
use lib "$FindBin::Bin/../../zip/lib";
use lib "$FindBin::Bin/../../lzss/lib";
use Test2::V0;

# Load all package modules at runtime under profiler
ok(eval { require CodingAdventures::ImageCodecPNG; 1 }, 'CodingAdventures::ImageCodecPNG loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Constants; 1 }, 'Constants loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Messages; 1 }, 'Messages loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Error; 1 }, 'Error loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Adler32; 1 }, 'Adler32 loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Chunk; 1 }, 'Chunk loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Filter; 1 }, 'Filter loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Decoder; 1 }, 'Decoder loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Encoder; 1 }, 'Encoder loads');
ok(eval { require CodingAdventures::ImageCodecPNG::Codec; 1 }, 'Codec loads');

# Import symbols into main namespace for testing
CodingAdventures::ImageCodecPNG->import(':all');
CodingAdventures::ImageCodecPNG::Constants->import(':all');
CodingAdventures::ImageCodecPNG::Messages->import(':all');

# 1. Module loading and exports
ok(1, 'All modules loaded cleanly');

is(PNG_MAX_DIMENSION(), 16_384, 'PNG_MAX_DIMENSION is 16384');
is(PNG_MAX_PIXELS(), 33_554_432, 'PNG_MAX_PIXELS is 33554432');

# 2. Closed set of 29 error codes in exact fixture taxonomy order
my @expected_codes = (
    'invalid-max-pixels',
    'invalid-image-dimensions',
    'invalid-pixel-data-length',
    'file-too-short',
    'invalid-signature',
    'truncated-chunk',
    'invalid-chunk-type',
    'chunk-crc-mismatch',
    'chunk-before-ihdr',
    'duplicate-ihdr',
    'invalid-ihdr-length',
    'invalid-dimensions',
    'dimension-limit',
    'pixel-limit',
    'unsupported-feature',
    'invalid-plte',
    'invalid-trns',
    'nonconsecutive-idat',
    'invalid-iend',
    'trailing-data',
    'unknown-critical-chunk',
    'missing-required-chunk',
    'invalid-zlib-header',
    'preset-dictionary',
    'inflate-failed',
    'inflated-length-mismatch',
    'idat-cavity',
    'adler-mismatch',
    'invalid-filter',
);

my @actual_codes = png_error_codes();
is(\@actual_codes, \@expected_codes, 'png_error_codes matches 29-code taxonomy in exact order');

# 3. Payload-blind error messages and standing rule #20
for my $code (@actual_codes) {
    my $err = CodingAdventures::ImageCodecPNG::Error->new($code);
    is($err->code, $code, "Error code for $code matches");
    my $msg = $err->message;
    ok(defined $msg && length($msg) > 0, "Message for $code is non-empty");
    like($msg, qr/^image-codec-png:/, "Message for $code has standard prefix");
    is("$err", $msg, "Stringification overload matches message");
}

# 4. Unknown code fallback
my $err_unknown = CodingAdventures::ImageCodecPNG::Error->new('completely-arbitrary-error');
is($err_unknown->code, 'unknown-error', 'Unknown code falls back to unknown-error');
is($err_unknown->message, 'image-codec-png: unknown error', 'Unknown code message matches catalog');

# 5. Placeholders
is(REDACTED_PLACEHOLDER(), '<redacted>', 'Redacted placeholder matches');
is(INVALID_PLACEHOLDER(), '<invalid>', 'Invalid placeholder matches');

# 6. Singleton codec
my $c1 = CodingAdventures::ImageCodecPNG::codec();
my $c2 = CodingAdventures::ImageCodecPNG::codec();
is($c1, $c2, 'codec() returns singleton');
is($c1->mime_type, 'image/png', 'mime_type is image/png');

done_testing;
