package CodingAdventures::Barcode1D;

use strict;
use warnings;

our $VERSION = '0.01';

require CodingAdventures::Code39;
require CodingAdventures::PixelContainer;

sub current_backend {
  return 'metal' if $^O eq 'darwin' && `uname -m` =~ /arm64|aarch64/;
  return undef;
}

sub _normalize_symbology {
  my ($symbology) = @_;
  $symbology //= 'code39';
  my $normalized = lc($symbology);
  $normalized =~ s/[_-]//g;
  return 'code39' if $normalized eq 'code39';
  die "unsupported symbology: $symbology";
}

sub build_scene {
  my ($class, $data, $symbology, $layout_config) = @_;
  my $normalized = _normalize_symbology($symbology);
  return CodingAdventures::Code39->layout_code39($data, $layout_config)
    if $normalized eq 'code39';
}

sub render_pixels {
  my ($class, $data, $symbology, $layout_config) = @_;
  die "no native Paint VM is available for this host" unless current_backend();

  require CodingAdventures::PaintVmMetalNative;
  my $scene = $class->build_scene($data, $symbology, $layout_config);
  my $rect_blob = join(
    "",
    map {
      join("\t", $_->{x}, $_->{y}, $_->{width}, $_->{height}, $_->{fill}) . "\n"
    } @{$scene->{instructions}}
  );

  my ($width, $height, $bytes) = CodingAdventures::PaintVmMetalNative::render_rect_scene_native(
    $scene->{width},
    $scene->{height},
    $scene->{background},
    $rect_blob,
  );

  my $pixels = CodingAdventures::PixelContainer->new($width, $height);
  ${$pixels->data} = $bytes;
  return $pixels;
}

sub render_png {
  my ($class, $data, $symbology, $layout_config) = @_;
  require CodingAdventures::PaintCodecPngNative;
  my $pixels = $class->render_pixels($data, $symbology, $layout_config);
  return CodingAdventures::PaintCodecPngNative::encode_rgba8_native(
    $pixels->width,
    $pixels->height,
    ${$pixels->data},
  );
}

1;
