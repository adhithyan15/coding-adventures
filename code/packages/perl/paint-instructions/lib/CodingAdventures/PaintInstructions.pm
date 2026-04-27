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

# paint_path — create a PaintPath instruction from an array of PathCommands.
#
# Each command is a hashref with a 'kind' key:
#   { kind => 'move_to', x => $x, y => $y }
#   { kind => 'line_to', x => $x, y => $y }
#   { kind => 'close' }
#
# This is used for drawing arbitrary vector shapes, such as flat-top hexagons
# in MaxiCode grids.
#
# Arguments:
#   $commands  - arrayref of PathCommand hashrefs
#   $fill      - fill color string (default '#000000')
#   $metadata  - optional hashref of metadata
sub paint_path {
    my ($class, $commands, $fill, $metadata) = @_;
    $commands //= [];
    $fill     //= '#000000';
    return {
        kind     => 'path',
        commands => $commands,
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
