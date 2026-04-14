package CodingAdventures::Barcode1D;

use strict;
use warnings;

our $VERSION = '0.01';

require CodingAdventures::Code39;
require CodingAdventures::Codabar;
require CodingAdventures::Code128;
require CodingAdventures::Ean13;
require CodingAdventures::Itf;
require CodingAdventures::PixelContainer;
require CodingAdventures::UpcA;

sub current_backend {
  return 'metal' if $^O eq 'darwin' && `uname -m` =~ /arm64|aarch64/;
  return undef;
}

sub _normalize_symbology {
  my ($symbology) = @_;
  $symbology //= 'code39';
  my $normalized = lc($symbology);
  $normalized =~ s/[_-]//g;
  return 'codabar' if $normalized eq 'codabar';
  return 'code128' if $normalized eq 'code128';
  return 'code39' if $normalized eq 'code39';
  return 'ean13' if $normalized eq 'ean13';
  return 'itf' if $normalized eq 'itf';
  return 'upca' if $normalized eq 'upca';
  die "unsupported symbology: $symbology";
}

sub build_scene {
  my ($class, $data, $symbology, $layout_config) = @_;
  my $normalized = _normalize_symbology($symbology);
  return CodingAdventures::Codabar->layout_codabar($data, $layout_config)
    if $normalized eq 'codabar';
  return CodingAdventures::Code128->layout_code128($data, $layout_config)
    if $normalized eq 'code128';
  return CodingAdventures::Code39->layout_code39($data, $layout_config)
    if $normalized eq 'code39';
  return CodingAdventures::Ean13->layout_ean_13($data, $layout_config)
    if $normalized eq 'ean13';
  return CodingAdventures::Itf->layout_itf($data, $layout_config)
    if $normalized eq 'itf';
  return CodingAdventures::UpcA->layout_upc_a($data, $layout_config)
    if $normalized eq 'upca';
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
