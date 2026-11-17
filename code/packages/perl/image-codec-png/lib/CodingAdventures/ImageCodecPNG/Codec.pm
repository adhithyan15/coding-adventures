package CodingAdventures::ImageCodecPNG::Codec;

use strict;
use warnings;
use CodingAdventures::ImageCodec;
use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Decoder;
use CodingAdventures::ImageCodecPNG::Encoder;

# Constructs a stateful PNG codec adapter instance.
sub new {
    my ($class, %opts) = @_;
    my $max_pixels = CodingAdventures::ImageCodecPNG::Decoder::validate_max_pixels($opts{max_pixels});

    return bless {
        max_pixels => $max_pixels,
    }, ref($class) || $class;
}

# Returns MIME type string for PNG images.
sub mime_type {
    return MIME_TYPE_PNG();
}

# Returns configured max_pixels limit.
sub max_pixels {
    return $_[0]->{max_pixels};
}

# Encodes an RGBA8 PixelContainer to PNG bytes.
sub encode {
    my ($self, $pixels) = @_;
    return CodingAdventures::ImageCodecPNG::Encoder::encode($pixels);
}

# Decodes PNG bytes to an RGBA8 PixelContainer.
sub decode {
    my ($self, $data) = @_;
    return CodingAdventures::ImageCodecPNG::Decoder::decode($data, max_pixels => $self->{max_pixels});
}

1;
