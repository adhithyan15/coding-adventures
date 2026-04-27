package CodingAdventures::PaintInstructions;

use strict;
use warnings;

our $VERSION = '0.01';

sub _copy_metadata {
    my ($metadata) = @_;
    return {} unless defined $metadata;
    return { %{$metadata} };
}

sub paint_rect {
    my ($class, $x, $y, $width, $height, $fill, $metadata) = @_;
    $fill //= '#000000';
    return {
        kind     => 'rect',
        x        => $x,
        y        => $y,
        width    => $width,
        height   => $height,
        fill     => $fill,
        metadata => _copy_metadata($metadata),
    };
}

sub paint_scene {
    my ($class, $width, $height, $instructions, $background, $metadata) = @_;
    $instructions //= [];
    $background   //= '#ffffff';
    return {
        width        => $width,
        height       => $height,
        instructions => $instructions,
        background   => $background,
        metadata     => _copy_metadata($metadata),
    };
}

sub create_scene {
    my ($class, @args) = @_;
    return $class->paint_scene(@args);
}

1;
