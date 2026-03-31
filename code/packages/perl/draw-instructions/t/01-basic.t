use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DrawInstructions;

# Convenience alias
my $pkg = 'CodingAdventures::DrawInstructions';

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is($pkg->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. draw_rect — basic
# ---------------------------------------------------------------------------
my $rect = $pkg->can('draw_rect')->(10, 20, 100, 50, '#ff0000');
is($rect->{kind},   'rect',    'rect kind');
is($rect->{x},      10,        'rect x');
is($rect->{y},      20,        'rect y');
is($rect->{width},  100,       'rect width');
is($rect->{height}, 50,        'rect height');
is($rect->{fill},   '#ff0000', 'rect fill');
is(ref $rect->{metadata}, 'HASH', 'rect metadata is hashref');
is(scalar keys %{$rect->{metadata}}, 0, 'rect metadata is empty by default');

# draw_rect with custom metadata
my $rect2 = $pkg->can('draw_rect')->(0, 0, 10, 10, 'blue', { id => 'box1' });
is($rect2->{metadata}{id}, 'box1', 'rect metadata is passed through');

# ---------------------------------------------------------------------------
# 3. draw_text — basic
# ---------------------------------------------------------------------------
my $text = $pkg->can('draw_text')->(10, 20, 'Hello');
is($text->{kind},        'text',       'text kind');
is($text->{x},           10,           'text x');
is($text->{y},           20,           'text y');
is($text->{value},       'Hello',      'text value');
is($text->{fill},        '#000000',    'text default fill');
is($text->{font_family}, 'monospace',  'text default font_family');
is($text->{font_size},   16,           'text default font_size');
is($text->{align},       'middle',     'text default align');
is(ref $text->{metadata}, 'HASH',      'text metadata is hashref');

# draw_text with all params (including font_weight and metadata)
my $text2 = $pkg->can('draw_text')->(
    5, 10, 'Hi', '#ff0000', 'sans-serif', 12, 'start', undef, { class => 'label' }
);
is($text2->{fill},        '#ff0000',   'text custom fill');
is($text2->{font_family}, 'sans-serif','text custom font_family');
is($text2->{font_size},   12,          'text custom font_size');
is($text2->{align},       'start',     'text custom align');
ok(!defined $text2->{font_weight},     'text font_weight undef when not set');
is($text2->{metadata}{class}, 'label', 'text custom metadata');

# ---------------------------------------------------------------------------
# 4. draw_line — basic
# ---------------------------------------------------------------------------
my $line = $pkg->can('draw_line')->(0, 0, 100, 100, '#000000');
is($line->{kind},   'line',    'line kind');
is($line->{x1},     0,         'line x1');
is($line->{y1},     0,         'line y1');
is($line->{x2},     100,       'line x2');
is($line->{y2},     100,       'line y2');
is($line->{stroke}, '#000000', 'line stroke');
is(ref $line->{metadata}, 'HASH', 'line metadata is hashref');

# draw_line with metadata
my $line2 = $pkg->can('draw_line')->(1, 2, 3, 4, 'red', { opacity => 0.5 });
is($line2->{metadata}{opacity}, 0.5, 'line metadata passed through');

# ---------------------------------------------------------------------------
# 5. draw_circle — basic
# ---------------------------------------------------------------------------
my $circle = $pkg->can('draw_circle')->(50, 50, 25, '#0000ff');
is($circle->{kind}, 'circle',   'circle kind');
is($circle->{cx},   50,         'circle cx');
is($circle->{cy},   50,         'circle cy');
is($circle->{r},    25,         'circle r');
is($circle->{fill}, '#0000ff',  'circle fill');
is(ref $circle->{metadata}, 'HASH', 'circle metadata is hashref');

# draw_circle with metadata
my $circle2 = $pkg->can('draw_circle')->(0, 0, 10, 'green', { id => 'dot' });
is($circle2->{metadata}{id}, 'dot', 'circle metadata passed through');

# ---------------------------------------------------------------------------
# 6. draw_group — basic
# ---------------------------------------------------------------------------
my $group = $pkg->can('draw_group')->([$rect, $text]);
is($group->{kind},     'group',    'group kind');
is(ref $group->{children}, 'ARRAY','group children is arrayref');
is(scalar @{$group->{children}}, 2, 'group has 2 children');
is($group->{children}[0], $rect,    'group child 0 is rect');
is($group->{children}[1], $text,    'group child 1 is text');
is(ref $group->{metadata}, 'HASH',  'group metadata is hashref');

# Empty group
my $empty_group = $pkg->can('draw_group')->([]);
is(scalar @{$empty_group->{children}}, 0, 'empty group has no children');

# Group with metadata
my $group2 = $pkg->can('draw_group')->([$line], { transform => 'translate(10,10)' });
is($group2->{metadata}{transform}, 'translate(10,10)', 'group metadata passed through');

# ---------------------------------------------------------------------------
# 7. create_scene — basic
# ---------------------------------------------------------------------------
my $scene = $pkg->can('create_scene')->(800, 600, [$rect], '#ffffff');
is($scene->{width},       800,       'scene width');
is($scene->{height},      600,       'scene height');
is($scene->{background},  '#ffffff', 'scene background');
is(ref $scene->{instructions}, 'ARRAY', 'scene instructions is arrayref');
is(scalar @{$scene->{instructions}}, 1, 'scene has 1 instruction');
is($scene->{instructions}[0], $rect,    'scene instruction is rect');
is(ref $scene->{metadata}, 'HASH',      'scene metadata is hashref');

# Scene with metadata
my $scene2 = $pkg->can('create_scene')->(
    1920, 1080, [$rect, $text, $line], '#000000',
    { title => 'My Scene' }
);
is($scene2->{width},        1920,        'scene2 width');
is($scene2->{height},       1080,        'scene2 height');
is($scene2->{background},   '#000000',   'scene2 background');
is(scalar @{$scene2->{instructions}}, 3, 'scene2 has 3 instructions');
is($scene2->{metadata}{title}, 'My Scene','scene2 metadata title');

# ---------------------------------------------------------------------------
# 8. Nested groups
# ---------------------------------------------------------------------------
my $inner = $pkg->can('draw_group')->([$rect]);
my $outer = $pkg->can('draw_group')->([$inner, $circle]);
is($outer->{children}[0]{kind}, 'group',  'nested group outer child 0 is group');
is($outer->{children}[1]{kind}, 'circle', 'nested group outer child 1 is circle');
is($outer->{children}[0]{children}[0]{kind}, 'rect', 'inner group contains rect');

# ---------------------------------------------------------------------------
# 9. draw_clip — basic
# ---------------------------------------------------------------------------
my $clip_child = $pkg->can('draw_rect')->(0, 0, 200, 200, '#ff0000');
my $clip = $pkg->can('draw_clip')->(10, 10, 80, 80, [$clip_child]);
is($clip->{kind},   'clip', 'clip kind');
is($clip->{x},      10,     'clip x');
is($clip->{y},      10,     'clip y');
is($clip->{width},  80,     'clip width');
is($clip->{height}, 80,     'clip height');
is(ref $clip->{children}, 'ARRAY', 'clip children is arrayref');
is(scalar @{$clip->{children}}, 1,  'clip has 1 child');
is($clip->{children}[0]{kind}, 'rect', 'clip child is rect');
is(ref $clip->{metadata}, 'HASH', 'clip metadata is hashref');
is(scalar keys %{$clip->{metadata}}, 0, 'clip metadata is empty by default');

# draw_clip with metadata
my $clip2 = $pkg->can('draw_clip')->(0, 0, 50, 50, [], { id => 'mask' });
is($clip2->{metadata}{id}, 'mask', 'clip metadata passed through');
is(scalar @{$clip2->{children}}, 0, 'clip with empty children');

# ---------------------------------------------------------------------------
# 10. draw_rect with stroke
# ---------------------------------------------------------------------------
my $stroked = $pkg->can('draw_rect')->(
    10, 20, 100, 50, '#ff0000',
    stroke => '#000000', stroke_width => 2
);
is($stroked->{kind},         'rect',    'stroked rect kind');
is($stroked->{fill},         '#ff0000', 'stroked rect fill');
is($stroked->{stroke},       '#000000', 'stroked rect stroke');
is($stroked->{stroke_width}, 2,         'stroked rect stroke_width');
is(ref $stroked->{metadata}, 'HASH',    'stroked rect metadata is hashref');

# draw_rect with stroke and metadata
my $stroked2 = $pkg->can('draw_rect')->(
    0, 0, 10, 10, 'blue',
    stroke => 'red', stroke_width => 1, metadata => { class => 'bordered' }
);
is($stroked2->{stroke},              'red',      'stroked2 stroke');
is($stroked2->{stroke_width},        1,          'stroked2 stroke_width');
is($stroked2->{metadata}{class},     'bordered', 'stroked2 metadata');

# draw_rect without stroke has undef stroke fields
my $no_stroke = $pkg->can('draw_rect')->(0, 0, 10, 10, 'green');
ok(!defined $no_stroke->{stroke},       'no stroke by default');
ok(!defined $no_stroke->{stroke_width}, 'no stroke_width by default');

# draw_rect legacy metadata-as-hashref call still works
my $legacy = $pkg->can('draw_rect')->(0, 0, 10, 10, 'white', { id => 'old' });
is($legacy->{metadata}{id}, 'old', 'legacy hashref metadata still works');
ok(!defined $legacy->{stroke},     'legacy call has no stroke');

# ---------------------------------------------------------------------------
# 11. draw_text with font_weight
# ---------------------------------------------------------------------------
my $bold_text = $pkg->can('draw_text')->(
    10, 20, 'Bold', '#000000', 'sans-serif', 14, 'start', 'bold'
);
is($bold_text->{kind},        'text',       'bold text kind');
is($bold_text->{font_weight}, 'bold',       'bold text font_weight');
is($bold_text->{value},       'Bold',       'bold text value');
is($bold_text->{font_family}, 'sans-serif', 'bold text font_family');

# draw_text with font_weight and metadata
my $bold_text2 = $pkg->can('draw_text')->(
    0, 0, 'X', '#fff', 'monospace', 12, 'end', 'normal', { id => 'lbl' }
);
is($bold_text2->{font_weight},    'normal', 'normal font_weight');
is($bold_text2->{metadata}{id},   'lbl',    'bold text metadata');

# draw_text without font_weight (default behaviour unchanged)
my $plain_text = $pkg->can('draw_text')->(5, 5, 'Hi');
ok(!defined $plain_text->{font_weight}, 'font_weight is undef by default');

# ---------------------------------------------------------------------------
# 12. Instructions are independent hashrefs (not shared mutable state)
# ---------------------------------------------------------------------------
my $r1 = $pkg->can('draw_rect')->(1, 2, 3, 4, 'red');
my $r2 = $pkg->can('draw_rect')->(5, 6, 7, 8, 'blue');
$r1->{metadata}{foo} = 'bar';
ok(!exists $r2->{metadata}{foo}, 'modifying one rect metadata does not affect another');

done_testing;
