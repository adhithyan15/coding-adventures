use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DrawInstructions;
use CodingAdventures::DrawInstructionsSvg;

# Convenience aliases
my $di  = 'CodingAdventures::DrawInstructions';
my $svg = 'CodingAdventures::DrawInstructionsSvg';

# ---------------------------------------------------------------------------
# 1. Renders complete SVG document
# ---------------------------------------------------------------------------
# A scene with a single rectangle should produce a well-formed SVG document
# with the correct root attributes: xmlns, width, height, viewBox, role,
# and aria-label.

subtest 'complete SVG document' => sub {
    my $scene = $di->can('create_scene')->(800, 600,
        [ $di->can('draw_rect')->(10, 20, 100, 50, '#ff0000') ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);

    like($result, qr/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg"/, 'starts with <svg xmlns>');
    like($result, qr/width="800"/, 'has width');
    like($result, qr/height="600"/, 'has height');
    like($result, qr/viewBox="0 0 800 600"/, 'has viewBox');
    like($result, qr/role="img"/, 'has role="img"');
    like($result, qr/aria-label="draw instructions scene"/, 'has default aria-label');
    like($result, qr/<\/svg>$/, 'ends with </svg>');

    # Background rect
    like($result, qr/<rect x="0" y="0" width="800" height="600" fill="#ffffff" \/>/, 'has background rect');
};

# ---------------------------------------------------------------------------
# 2. Renders rect
# ---------------------------------------------------------------------------
subtest 'renders rect' => sub {
    my $scene = $di->can('create_scene')->(100, 100,
        [ $di->can('draw_rect')->(10, 20, 30, 40, '#aabbcc') ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/<rect x="10" y="20" width="30" height="40" fill="#aabbcc" \/>/, 'rect element present');
};

# ---------------------------------------------------------------------------
# 3. Renders text with XML escaping
# ---------------------------------------------------------------------------
# Text containing XML special characters must be properly escaped in both
# the element content and any attribute values.

subtest 'renders text with escaping' => sub {
    my $scene = $di->can('create_scene')->(200, 100,
        [ $di->can('draw_text')->(10, 20, 'A & B <C>') ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/>A &amp; B &lt;C&gt;<\/text>/, 'text content is XML-escaped');
    like($result, qr/text-anchor="middle"/, 'has text-anchor from align');
    like($result, qr/font-family="monospace"/, 'has font-family');
    like($result, qr/font-size="16"/, 'has font-size');
    like($result, qr/fill="#000000"/, 'has fill');
};

# ---------------------------------------------------------------------------
# 4. Renders group recursively
# ---------------------------------------------------------------------------
# A group containing a rect and a text should produce a <g> with both
# children rendered inside it.

subtest 'renders group recursively' => sub {
    my $rect = $di->can('draw_rect')->(0, 0, 10, 10, 'red');
    my $text = $di->can('draw_text')->(5, 5, 'Hi');
    my $group = $di->can('draw_group')->([$rect, $text]);
    my $scene = $di->can('create_scene')->(100, 100, [$group], '#ffffff');
    my $result = $svg->can('render_svg')->($scene);

    like($result, qr/<g>/, 'has opening <g>');
    like($result, qr/<\/g>/, 'has closing </g>');
    like($result, qr/<rect x="0" y="0" width="10" height="10" fill="red" \/>/, 'group child rect');
    like($result, qr/<text.*>Hi<\/text>/, 'group child text');
};

# ---------------------------------------------------------------------------
# 5. Renders line
# ---------------------------------------------------------------------------
subtest 'renders line' => sub {
    my $scene = $di->can('create_scene')->(100, 100,
        [ $di->can('draw_line')->(0, 0, 50, 75, '#333333') ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/<line x1="0" y1="0" x2="50" y2="75" stroke="#333333" stroke-width="1" \/>/, 'line element present');
};

# ---------------------------------------------------------------------------
# 6. Renders clip
# ---------------------------------------------------------------------------
# A clip should produce a <defs> block with a <clipPath> containing a <rect>,
# followed by a <g> referencing that clipPath and containing the children.

subtest 'renders clip' => sub {
    my $child = $di->can('draw_rect')->(0, 0, 200, 200, '#ff0000');
    my $clip  = $di->can('draw_clip')->(10, 10, 80, 80, [$child]);
    my $scene = $di->can('create_scene')->(100, 100, [$clip], '#ffffff');
    my $result = $svg->can('render_svg')->($scene);

    like($result, qr/<defs>/, 'has <defs>');
    like($result, qr/<clipPath id="clip-1">/, 'has clipPath with id');
    like($result, qr/<rect x="10" y="10" width="80" height="80" \/>/, 'clipPath rect');
    like($result, qr/<\/clipPath>/, 'closes clipPath');
    like($result, qr/<\/defs>/, 'closes defs');
    like($result, qr/<g clip-path="url\(#clip-1\)">/, 'g references clipPath');
    like($result, qr/<rect x="0" y="0" width="200" height="200" fill="#ff0000" \/>/, 'clip child rect');
};

# ---------------------------------------------------------------------------
# 7. Renders circle
# ---------------------------------------------------------------------------
subtest 'renders circle' => sub {
    my $scene = $di->can('create_scene')->(100, 100,
        [ $di->can('draw_circle')->(50, 50, 25, '#0000ff') ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/<circle cx="50" cy="50" r="25" fill="#0000ff" \/>/, 'circle element present');
};

# ---------------------------------------------------------------------------
# 8. Handles stroke on rect
# ---------------------------------------------------------------------------
subtest 'handles stroke on rect' => sub {
    my $scene = $di->can('create_scene')->(100, 100,
        [ $di->can('draw_rect')->(5, 5, 90, 90, '#fff',
            stroke => '#000', stroke_width => 3) ],
        '#ffffff',
    );
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/stroke="#000"/, 'has stroke attribute');
    like($result, qr/stroke-width="3"/, 'has stroke-width attribute');
};

# ---------------------------------------------------------------------------
# 9. Handles font_weight on text
# ---------------------------------------------------------------------------
subtest 'handles font_weight on text' => sub {
    # Bold text should include font-weight attribute
    my $bold = $di->can('draw_text')->(10, 20, 'Bold', '#000', 'sans-serif', 14, 'start', 'bold');
    my $scene = $di->can('create_scene')->(100, 100, [$bold], '#ffffff');
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/font-weight="bold"/, 'bold text has font-weight');

    # "normal" weight should NOT produce a font-weight attribute (it is the default)
    my $normal = $di->can('draw_text')->(10, 20, 'Normal', '#000', 'sans-serif', 14, 'start', 'normal');
    my $scene2 = $di->can('create_scene')->(100, 100, [$normal], '#ffffff');
    my $result2 = $svg->can('render_svg')->($scene2);
    unlike($result2, qr/font-weight/, 'normal weight omits font-weight attribute');

    # undef weight should NOT produce a font-weight attribute
    my $undef = $di->can('draw_text')->(10, 20, 'Default');
    my $scene3 = $di->can('create_scene')->(100, 100, [$undef], '#ffffff');
    my $result3 = $svg->can('render_svg')->($scene3);
    unlike($result3, qr/font-weight/, 'undef weight omits font-weight attribute');
};

# ---------------------------------------------------------------------------
# 10. Clip counter resets between renders
# ---------------------------------------------------------------------------
# Calling render_svg() twice should produce identical clip IDs because
# the counter resets at the start of each call.

subtest 'clip counter resets between renders' => sub {
    my $clip  = $di->can('draw_clip')->(0, 0, 50, 50,
        [ $di->can('draw_rect')->(0, 0, 100, 100, 'red') ]);
    my $scene = $di->can('create_scene')->(100, 100, [$clip], '#fff');

    my $first  = $svg->can('render_svg')->($scene);
    my $second = $svg->can('render_svg')->($scene);
    is($first, $second, 'two renders produce identical output');
    like($first, qr/clip-1/, 'clip ID starts at 1');
};

# ---------------------------------------------------------------------------
# 11. Custom aria-label from metadata
# ---------------------------------------------------------------------------
subtest 'custom aria-label from metadata' => sub {
    my $scene = $di->can('create_scene')->(100, 100, [], '#fff',
        { label => 'My Chart' });
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/aria-label="My Chart"/, 'uses label from metadata');
};

# ---------------------------------------------------------------------------
# 12. Metadata becomes data-* attributes
# ---------------------------------------------------------------------------
subtest 'metadata to data-* attributes' => sub {
    my $rect = $di->can('draw_rect')->(0, 0, 10, 10, 'red', metadata => { id => 'box' });
    my $scene = $di->can('create_scene')->(100, 100, [$rect], '#fff');
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/data-id="box"/, 'rect has data-id attribute');
};

# ---------------------------------------------------------------------------
# 13. Nested groups
# ---------------------------------------------------------------------------
subtest 'nested groups' => sub {
    my $inner = $di->can('draw_group')->([ $di->can('draw_rect')->(0, 0, 5, 5, 'blue') ]);
    my $outer = $di->can('draw_group')->([$inner]);
    my $scene = $di->can('create_scene')->(100, 100, [$outer], '#fff');
    my $result = $svg->can('render_svg')->($scene);

    # Count <g> tags — should have at least 2 (outer + inner)
    my @g_opens = ($result =~ /<g[> ]/g);
    ok(scalar @g_opens >= 2, 'has at least two <g> elements for nesting');
};

# ---------------------------------------------------------------------------
# 14. Multiple clips get unique IDs
# ---------------------------------------------------------------------------
subtest 'multiple clips get unique IDs' => sub {
    my $clip1 = $di->can('draw_clip')->(0, 0, 10, 10, []);
    my $clip2 = $di->can('draw_clip')->(20, 20, 10, 10, []);
    my $scene = $di->can('create_scene')->(100, 100, [$clip1, $clip2], '#fff');
    my $result = $svg->can('render_svg')->($scene);

    like($result, qr/clip-1/, 'first clip gets clip-1');
    like($result, qr/clip-2/, 'second clip gets clip-2');
};

# ---------------------------------------------------------------------------
# 15. XML escaping in attribute values
# ---------------------------------------------------------------------------
subtest 'XML escaping in attributes' => sub {
    my $rect = $di->can('draw_rect')->(0, 0, 10, 10, 'a&b<c>"d');
    my $scene = $di->can('create_scene')->(100, 100, [$rect], '#fff');
    my $result = $svg->can('render_svg')->($scene);
    like($result, qr/fill="a&amp;b&lt;c&gt;&quot;d"/, 'fill attribute is XML-escaped');
};

done_testing;
