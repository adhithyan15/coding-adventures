package CodingAdventures::DrawInstructions;

# ============================================================================
# CodingAdventures::DrawInstructions — Backend-neutral drawing instruction set
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# WHAT ARE DRAW INSTRUCTIONS?
# ----------------------------
# When building interactive visualisations you face a choice: target SVG?
# Canvas? WebGL? A terminal?  Each backend has its own API.
#
# The solution is an intermediate representation: a tree of plain Perl
# data structures (hashrefs) that describe *what* to draw without caring
# *how* it is drawn.  A separate renderer converts those hashrefs into
# SVG tags, Canvas calls, or whatever you need.
#
# This is the same pattern used by React ("virtual DOM"), the PDF spec
# ("page description language"), and PostScript.
#
# INSTRUCTION TYPES
# -----------------
# Every instruction is a hashref with at minimum a `kind` key.  The
# currently defined kinds are:
#
#   rect   — filled rectangle
#   text   — positioned text string
#   line   — straight line segment
#   circle — filled circle
#   group  — ordered list of child instructions
#
# All instructions carry a `metadata` field (hashref) for renderer-specific
# hints such as IDs, class names, event handlers, etc.
#
# SCENE
# -----
# A scene wraps a list of top-level instructions together with overall
# dimensions and a background colour.  It is the root of the draw tree.
#
# Usage:
#
#   use CodingAdventures::DrawInstructions;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# draw_rect($x, $y, $width, $height, $fill [, $metadata])
# ============================================================================
#
# Creates a rectangle instruction.
#
# Parameters:
#   $x, $y        — top-left corner coordinates
#   $width        — width of the rectangle
#   $height       — height of the rectangle
#   $fill         — CSS colour string (e.g., "#ff0000" or "red")
#   $metadata     — optional hashref of extra data (default: {})
#
# Returns a hashref:
#   {
#     kind     => "rect",
#     x        => $x,
#     y        => $y,
#     width    => $width,
#     height   => $height,
#     fill     => $fill,
#     metadata => $metadata,
#   }
sub draw_rect {
    my ($x, $y, $width, $height, $fill, $metadata) = @_;
    $metadata //= {};
    return {
        kind     => 'rect',
        x        => $x,
        y        => $y,
        width    => $width,
        height   => $height,
        fill     => $fill,
        metadata => $metadata,
    };
}

# ============================================================================
# draw_text($x, $y, $value [, $fill, $font_family, $font_size, $align, $metadata])
# ============================================================================
#
# Creates a text instruction.
#
# Parameters:
#   $x, $y        — anchor point
#   $value        — the string to render
#   $fill         — text colour (default: "#000000")
#   $font_family  — font (default: "monospace")
#   $font_size    — size in pixels (default: 16)
#   $align        — text alignment: "start", "middle", or "end" (default: "middle")
#   $metadata     — optional hashref (default: {})
#
# In practice the most common usage just passes x, y, and value:
#
#   my $t = draw_text(10, 20, "Hello");
#
# For extra styling pass a hashref as the last argument:
#
#   my $t = draw_text(10, 20, "Hi", "#ff0000", "sans-serif", 12, "start", {});
sub draw_text {
    my ($x, $y, $value, $fill, $font_family, $font_size, $align, $metadata) = @_;
    $fill        //= '#000000';
    $font_family //= 'monospace';
    $font_size   //= 16;
    $align       //= 'middle';
    $metadata    //= {};
    return {
        kind        => 'text',
        x           => $x,
        y           => $y,
        value       => $value,
        fill        => $fill,
        font_family => $font_family,
        font_size   => $font_size,
        align       => $align,
        metadata    => $metadata,
    };
}

# ============================================================================
# draw_group(\@children [, $metadata])
# ============================================================================
#
# Groups a list of child instructions into a single composable node.
# Renderers typically translate this into an SVG <g> element or a Canvas
# save/restore block.
#
# Parameters:
#   \@children    — arrayref of instruction hashrefs
#   $metadata     — optional hashref (default: {})
#
# Returns:
#   {
#     kind     => "group",
#     children => \@children,
#     metadata => $metadata,
#   }
sub draw_group {
    my ($children, $metadata) = @_;
    $metadata //= {};
    return {
        kind     => 'group',
        children => $children,
        metadata => $metadata,
    };
}

# ============================================================================
# draw_line($x1, $y1, $x2, $y2, $stroke [, $metadata])
# ============================================================================
#
# Creates a straight line segment from ($x1,$y1) to ($x2,$y2).
#
# Parameters:
#   $x1, $y1      — start point
#   $x2, $y2      — end point
#   $stroke       — stroke colour (e.g., "#000000")
#   $metadata     — optional hashref (default: {})
sub draw_line {
    my ($x1, $y1, $x2, $y2, $stroke, $metadata) = @_;
    $metadata //= {};
    return {
        kind     => 'line',
        x1       => $x1,
        y1       => $y1,
        x2       => $x2,
        y2       => $y2,
        stroke   => $stroke,
        metadata => $metadata,
    };
}

# ============================================================================
# draw_circle($cx, $cy, $r, $fill [, $metadata])
# ============================================================================
#
# Creates a filled circle.
#
# Parameters:
#   $cx, $cy      — centre of the circle
#   $r            — radius
#   $fill         — fill colour
#   $metadata     — optional hashref (default: {})
sub draw_circle {
    my ($cx, $cy, $r, $fill, $metadata) = @_;
    $metadata //= {};
    return {
        kind     => 'circle',
        cx       => $cx,
        cy       => $cy,
        r        => $r,
        fill     => $fill,
        metadata => $metadata,
    };
}

# ============================================================================
# create_scene($width, $height, \@instructions, $background [, $metadata])
# ============================================================================
#
# Wraps a set of top-level draw instructions into a scene.
#
# Parameters:
#   $width, $height   — pixel dimensions of the output surface
#   \@instructions    — arrayref of draw instruction hashrefs
#   $background       — background colour (e.g., "#ffffff")
#   $metadata         — optional hashref (default: {})
#
# Returns:
#   {
#     width        => $width,
#     height       => $height,
#     background   => $background,
#     instructions => \@instructions,
#     metadata     => $metadata,
#   }
sub create_scene {
    my ($width, $height, $instructions, $background, $metadata) = @_;
    $metadata //= {};
    return {
        width        => $width,
        height       => $height,
        background   => $background,
        instructions => $instructions,
        metadata     => $metadata,
    };
}

1;

__END__

=head1 NAME

CodingAdventures::DrawInstructions - Backend-neutral drawing instruction set for visualizations

=head1 SYNOPSIS

    use CodingAdventures::DrawInstructions;

    my $rect = CodingAdventures::DrawInstructions::draw_rect(10, 20, 100, 50, "#ff0000");
    my $text = CodingAdventures::DrawInstructions::draw_text(10, 20, "Hello");
    my $line = CodingAdventures::DrawInstructions::draw_line(0, 0, 100, 100, "#000000");
    my $circ = CodingAdventures::DrawInstructions::draw_circle(50, 50, 25, "#0000ff");
    my $grp  = CodingAdventures::DrawInstructions::draw_group([$rect, $text]);
    my $scene = CodingAdventures::DrawInstructions::create_scene(800, 600, [$grp], "#ffffff");

=head1 DESCRIPTION

Provides a set of plain-data constructor functions for describing 2-D drawing
operations in a backend-neutral way.  Each function returns a hashref; no
objects or classes are used.  A downstream renderer converts the tree into
SVG, Canvas 2D, or any other target.

=head2 Functions

=over 4

=item draw_rect($x, $y, $width, $height, $fill [, $metadata])

=item draw_text($x, $y, $value [, $fill, $font_family, $font_size, $align, $metadata])

=item draw_group(\@children [, $metadata])

=item draw_line($x1, $y1, $x2, $y2, $stroke [, $metadata])

=item draw_circle($cx, $cy, $r, $fill [, $metadata])

=item create_scene($width, $height, \@instructions, $background [, $metadata])

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
