package CodingAdventures::DrawInstructionsSvg;

# ============================================================================
# CodingAdventures::DrawInstructionsSvg — SVG renderer for draw instructions
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# WHAT DOES THIS MODULE DO?
# -------------------------
# It takes a scene hashref (as produced by CodingAdventures::DrawInstructions)
# and serialises it into a complete SVG document string.  That is all it does.
#
# The module is intentionally boring.  It knows how to turn draw instructions
# into SVG markup and nothing more.  No barcode rules, no graph logic, no
# domain knowledge.  That separation is the whole reason this package exists.
#
# HOW SVG MAPS TO DRAW INSTRUCTIONS
# ----------------------------------
# Each draw instruction kind has a direct SVG counterpart:
#
#   rect   -> <rect>          Filled rectangle, optional stroke outline
#   text   -> <text>          Positioned text with font styling
#   line   -> <line>          Straight line segment
#   circle -> <circle>        Filled circle
#   clip   -> <clipPath> + <g>  Rectangular clipping region
#   group  -> <g>             Grouping container
#
# The scene itself becomes the root <svg> element with a background <rect>
# and proper xmlns, viewBox, role="img", and aria-label attributes.
#
# XML ESCAPING
# ------------
# All user-provided text and attribute values are escaped to prevent SVG/XML
# injection.  The five XML special characters are handled:
#
#   &  ->  &amp;     (must be first to avoid double-escaping)
#   <  ->  &lt;
#   >  ->  &gt;
#   "  ->  &quot;
#   '  ->  &apos;
#
# METADATA
# --------
# Metadata hashrefs are serialised as data-* attributes on the corresponding
# SVG element.  This keeps the SVG valid while preserving semantic information
# for downstream tooling (browser inspectors, test harnesses, visualizers).
#
# CLIP IDs
# --------
# Each clip instruction needs a unique ID for its <clipPath> element.  A
# counter is incremented for each clip and reset at the start of every
# render_svg() call so output is deterministic across multiple renders.
#
# Usage:
#
#   use CodingAdventures::DrawInstructions;
#   use CodingAdventures::DrawInstructionsSvg;
#
#   my $scene = CodingAdventures::DrawInstructions::create_scene(
#       800, 600,
#       [ CodingAdventures::DrawInstructions::draw_rect(10, 20, 100, 50, "#ff0000") ],
#       "#ffffff",
#   );
#
#   my $svg = CodingAdventures::DrawInstructionsSvg::render_svg($scene);
#   print $svg;
#
# ============================================================================

use strict;
use warnings;

use CodingAdventures::DrawInstructions;

our $VERSION = '0.01';

# ============================================================================
# Module-level clip ID counter
# ============================================================================
#
# Each clip instruction requires a unique ID for the SVG <clipPath> element.
# We use a simple incrementing counter that resets at the start of each
# render_svg() call.  This ensures deterministic output: calling render_svg()
# twice on the same scene produces identical SVG strings.

my $clip_id_counter = 0;

# ============================================================================
# xml_escape($string)
# ============================================================================
#
# Escapes the five XML special characters in a string so it can be safely
# embedded in an SVG attribute value or text node.
#
# The ampersand MUST be replaced first.  If you replace < with &lt; first
# and then replace & with &amp;, you would turn &lt; into &amp;lt; — a
# double-escape bug.
#
#   Input:    'Tom & "Jerry" <cat>'
#   Output:   'Tom &amp; &quot;Jerry&quot; &lt;cat&gt;'
#
# Truth table for replacement order:
#
#   Step  | Character | Replacement | Why this order?
#   ------|-----------|-------------|--------------------------------
#   1     | &         | &amp;       | Must be first (contains no &)
#   2     | <         | &lt;        | Safe — no & introduced
#   3     | >         | &gt;        | Safe — no & introduced
#   4     | "         | &quot;      | Safe — no & introduced
#   5     | '         | &apos;      | Safe — no & introduced

sub xml_escape {
    my ($str) = @_;
    $str =~ s/&/&amp;/g;
    $str =~ s/</&lt;/g;
    $str =~ s/>/&gt;/g;
    $str =~ s/"/&quot;/g;
    $str =~ s/'/&apos;/g;
    return $str;
}

# ============================================================================
# metadata_to_attributes($metadata)
# ============================================================================
#
# Converts a metadata hashref into a string of HTML5/SVG data-* attributes.
#
# For example:
#
#   { id => "box", class => "red" }
#
# becomes:
#
#   ' data-id="box" data-class="red"'
#
# Note the leading space — this makes it easy to concatenate with other
# attribute strings.  When metadata is empty or undef, returns an empty
# string (no leading space).
#
# Keys are sorted alphabetically to ensure deterministic output for testing.

sub metadata_to_attributes {
    my ($metadata) = @_;
    return '' unless $metadata && %{$metadata};

    my @attrs;
    for my $key (sort keys %{$metadata}) {
        my $escaped_value = xml_escape("$metadata->{$key}");
        push @attrs, qq{ data-$key="$escaped_value"};
    }
    return join('', @attrs);
}

# ============================================================================
# render_rect($instruction)
# ============================================================================
#
# Converts a rect instruction hashref into an SVG <rect> element string.
#
# When stroke is defined, stroke and stroke-width attributes are added.
# When stroke is undef (the default), only the fill is rendered.
#
# Example output:
#
#   <rect x="10" y="20" width="100" height="50" fill="#ff0000" />
#
# With stroke:
#
#   <rect x="10" y="20" width="100" height="50" fill="#ff0000"
#         stroke="#000000" stroke-width="2" />

sub render_rect {
    my ($instr) = @_;
    my $stroke_attrs = '';
    if (defined $instr->{stroke}) {
        my $sw = $instr->{stroke_width} // 1;
        $stroke_attrs = ' stroke="' . xml_escape($instr->{stroke}) . '"'
                      . ' stroke-width="' . $sw . '"';
    }
    my $meta = metadata_to_attributes($instr->{metadata});
    return qq{  <rect x="$instr->{x}" y="$instr->{y}" width="$instr->{width}" height="$instr->{height}" fill="} . xml_escape($instr->{fill}) . qq{"$stroke_attrs$meta />};
}

# ============================================================================
# render_text($instruction)
# ============================================================================
#
# Converts a text instruction hashref into an SVG <text> element.
#
# The text value is XML-escaped and placed as the element's text content.
# Font attributes are mapped to their SVG equivalents:
#
#   Perl field    -> SVG attribute
#   ----------    -> -------------
#   align         -> text-anchor
#   font_family   -> font-family
#   font_size     -> font-size
#   fill          -> fill
#   font_weight   -> font-weight (only when defined and not "normal")
#
# The font_weight attribute is omitted when undef or "normal" because
# SVG's default weight is already normal.  Including it only when it
# differs reduces noise in the output.

sub render_text {
    my ($instr) = @_;
    my $weight_attr = '';
    if (defined $instr->{font_weight} && $instr->{font_weight} ne 'normal') {
        $weight_attr = ' font-weight="' . $instr->{font_weight} . '"';
    }
    my $meta = metadata_to_attributes($instr->{metadata});
    my $escaped_value = xml_escape($instr->{value});
    return qq{  <text x="$instr->{x}" y="$instr->{y}" text-anchor="$instr->{align}" font-family="} . xml_escape($instr->{font_family}) . qq{" font-size="$instr->{font_size}" fill="} . xml_escape($instr->{fill}) . qq{"$weight_attr$meta>$escaped_value</text>};
}

# ============================================================================
# render_line($instruction)
# ============================================================================
#
# Converts a line instruction into an SVG <line> element.
#
# SVG <line> uses x1/y1/x2/y2 attributes — a direct 1:1 mapping from
# the DrawLineInstruction fields.  The stroke colour is always present
# (lines without stroke would be invisible).
#
# Note: unlike rect, line does not have a separate stroke_width field in
# the draw instruction.  Lines use a default stroke-width of 1.

sub render_line {
    my ($instr) = @_;
    my $meta = metadata_to_attributes($instr->{metadata});
    return qq{  <line x1="$instr->{x1}" y1="$instr->{y1}" x2="$instr->{x2}" y2="$instr->{y2}" stroke="} . xml_escape($instr->{stroke}) . qq{" stroke-width="1"$meta />};
}

# ============================================================================
# render_circle($instruction)
# ============================================================================
#
# Converts a circle instruction into an SVG <circle> element.
#
# SVG <circle> uses cx, cy, and r attributes — a direct mapping from the
# draw instruction fields.

sub render_circle {
    my ($instr) = @_;
    my $meta = metadata_to_attributes($instr->{metadata});
    return qq{  <circle cx="$instr->{cx}" cy="$instr->{cy}" r="$instr->{r}" fill="} . xml_escape($instr->{fill}) . qq{"$meta />};
}

# ============================================================================
# render_clip($instruction)
# ============================================================================
#
# Converts a clip instruction into an SVG clipPath + g structure.
#
# SVG clipping requires:
#   1. A <clipPath> element in <defs> containing a <rect> that defines
#      the clipping region
#   2. A <g> element referencing the clipPath via clip-path="url(#id)"
#      that wraps the clipped children
#
# The clip ID counter is incremented for each clip so every clipPath gets
# a unique ID within the document.  The counter is reset in render_svg().
#
# Example output for a clip at (10, 10, 80, 80) with one rect child:
#
#   <defs>
#     <clipPath id="clip-1">
#       <rect x="10" y="10" width="80" height="80" />
#     </clipPath>
#   </defs>
#   <g clip-path="url(#clip-1)">
#     <rect x="0" y="0" width="200" height="200" fill="#ff0000" />
#   </g>

sub render_clip {
    my ($instr) = @_;
    $clip_id_counter++;
    my $id = "clip-$clip_id_counter";
    my $meta = metadata_to_attributes($instr->{metadata});

    my @children_svg;
    for my $child (@{$instr->{children}}) {
        push @children_svg, render_instruction($child);
    }
    my $children_str = join("\n", @children_svg);

    my @lines = (
        "  <defs>",
        qq{    <clipPath id="$id">},
        qq{      <rect x="$instr->{x}" y="$instr->{y}" width="$instr->{width}" height="$instr->{height}" />},
        "    </clipPath>",
        "  </defs>",
        qq{  <g clip-path="url(#$id)"$meta>},
        $children_str,
        "  </g>",
    );
    return join("\n", @lines);
}

# ============================================================================
# render_group($instruction)
# ============================================================================
#
# Converts a group instruction into an SVG <g> element containing its
# children rendered recursively.
#
# Groups are the composition primitive: they let you bundle instructions
# together under a single SVG <g> node.  This is useful for transforms,
# opacity, and logical grouping.

sub render_group {
    my ($instr) = @_;
    my $meta = metadata_to_attributes($instr->{metadata});

    my @children_svg;
    for my $child (@{$instr->{children}}) {
        push @children_svg, render_instruction($child);
    }
    my $children_str = join("\n", @children_svg);

    return join("\n", "  <g$meta>", $children_str, "  </g>");
}

# ============================================================================
# render_instruction($instruction)
# ============================================================================
#
# Dispatches a single instruction hashref to the correct renderer based
# on the 'kind' field.
#
# This is the central routing function.  Every instruction flows through
# here, including children of groups and clips (which call this
# recursively).
#
# Dispatch table:
#
#   Kind     | Function
#   ---------|-------------------
#   rect     | render_rect()
#   text     | render_text()
#   line     | render_line()
#   circle   | render_circle()
#   clip     | render_clip()
#   group    | render_group()

sub render_instruction {
    my ($instr) = @_;
    my $kind = $instr->{kind};

    if ($kind eq 'rect') {
        return render_rect($instr);
    } elsif ($kind eq 'text') {
        return render_text($instr);
    } elsif ($kind eq 'line') {
        return render_line($instr);
    } elsif ($kind eq 'circle') {
        return render_circle($instr);
    } elsif ($kind eq 'clip') {
        return render_clip($instr);
    } elsif ($kind eq 'group') {
        return render_group($instr);
    } else {
        die "Unknown draw instruction kind: $kind";
    }
}

# ============================================================================
# render_svg($scene)
# ============================================================================
#
# The main entry point.  Takes a scene hashref and returns a complete SVG
# document as a string.
#
# The SVG document includes:
#   - xmlns declaration for SVG namespace
#   - width/height from the scene dimensions
#   - viewBox matching the scene dimensions (0 0 width height)
#   - role="img" for accessibility
#   - aria-label from scene metadata (defaults to "draw instructions scene")
#   - A full-size background <rect>
#   - All scene instructions rendered in order
#
# The clip ID counter is reset at the start of each call to ensure
# deterministic output.  This means calling render_svg() twice on the
# same scene produces identical strings.
#
# Parameters:
#   $scene — a scene hashref as returned by create_scene()
#
# Returns:
#   A string containing a complete SVG document.

sub render_svg {
    my ($scene) = @_;

    # Reset clip counter for deterministic output across renders.
    $clip_id_counter = 0;

    # Determine the accessibility label from scene metadata.
    my $label = 'draw instructions scene';
    if ($scene->{metadata} && defined $scene->{metadata}{label}) {
        $label = xml_escape("$scene->{metadata}{label}");
    }

    # Render all top-level instructions.
    my @instruction_svgs;
    for my $instr (@{$scene->{instructions}}) {
        push @instruction_svgs, render_instruction($instr);
    }
    my $instructions_str = join("\n", @instruction_svgs);

    my $bg = xml_escape($scene->{background});

    return join("\n",
        qq{<svg xmlns="http://www.w3.org/2000/svg" width="$scene->{width}" height="$scene->{height}" viewBox="0 0 $scene->{width} $scene->{height}" role="img" aria-label="$label">},
        qq{  <rect x="0" y="0" width="$scene->{width}" height="$scene->{height}" fill="$bg" />},
        $instructions_str,
        "</svg>",
    );
}

1;

__END__

=head1 NAME

CodingAdventures::DrawInstructionsSvg - SVG renderer for draw instructions

=head1 SYNOPSIS

    use CodingAdventures::DrawInstructions;
    use CodingAdventures::DrawInstructionsSvg;

    my $scene = CodingAdventures::DrawInstructions::create_scene(
        800, 600,
        [ CodingAdventures::DrawInstructions::draw_rect(10, 20, 100, 50, "#ff0000") ],
        "#ffffff",
    );

    my $svg = CodingAdventures::DrawInstructionsSvg::render_svg($scene);

=head1 DESCRIPTION

Serialises a draw instruction scene into a complete SVG document string.

Handles all instruction kinds: rect, text, line, circle, clip, and group.
Text and attribute values are XML-escaped.  Metadata hashrefs become data-*
attributes on the SVG elements.

=head1 FUNCTIONS

=over 4

=item render_svg($scene)

Takes a scene hashref and returns a complete SVG document string.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
