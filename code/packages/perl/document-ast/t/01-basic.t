use strict;
use warnings;
use Test2::V0;

# Load the module
ok( eval { require CodingAdventures::DocumentAst; 1 }, 'DocumentAst module loads' )
    or diag($@);

CodingAdventures::DocumentAst->import(qw(
    make_document make_heading make_paragraph make_code_block
    make_blockquote make_list make_list_item make_task_item
    make_thematic_break make_raw_block
    make_text make_emphasis make_strong make_strikethrough
    make_code_span make_link make_image make_autolink
    make_raw_inline make_hard_break make_soft_break
    node_type is_block is_inline walk
));

# ===== Test 1: make_document =====
{
    my $doc = make_document([]);
    is( $doc->{type},     'document', 'make_document sets type' );
    is( ref($doc->{children}), 'ARRAY', 'make_document has children array' );
    is( scalar(@{ $doc->{children} }), 0, 'empty document has no children' );
}

# ===== Test 2: make_heading =====
{
    my $h = make_heading(2, [make_text('Hello')]);
    is( $h->{type},  'heading', 'make_heading sets type' );
    is( $h->{level}, 2,         'make_heading sets level' );
    is( scalar(@{ $h->{children} }), 1, 'make_heading has one child' );
    is( $h->{children}[0]{type}, 'text', 'heading child is text node' );
}

# ===== Test 3: make_paragraph =====
{
    my $p = make_paragraph([make_text('Hello world')]);
    is( $p->{type}, 'paragraph', 'make_paragraph sets type' );
    is( scalar(@{ $p->{children} }), 1, 'paragraph has one child' );
}

# ===== Test 4: make_code_block =====
{
    my $cb = make_code_block('perl', "print 1;\n");
    is( $cb->{type},     'code_block',  'make_code_block sets type' );
    is( $cb->{language}, 'perl',        'make_code_block sets language' );
    is( $cb->{value},    "print 1;\n",  'make_code_block sets value' );
}

# ===== Test 5: make_code_block with undef language =====
{
    my $cb = make_code_block(undef, 'raw code');
    is( $cb->{language}, undef, 'code_block allows undef language' );
}

# ===== Test 6: make_blockquote =====
{
    my $bq = make_blockquote([make_paragraph([make_text('quote')])]);
    is( $bq->{type}, 'blockquote', 'make_blockquote sets type' );
    is( $bq->{children}[0]{type}, 'paragraph', 'blockquote contains paragraph' );
}

# ===== Test 7: make_list (unordered) =====
{
    my $list = make_list(0, undef, 1, [make_list_item([make_text('item')])]);
    is( $list->{type},    'list',   'make_list sets type' );
    is( $list->{ordered}, 0,        'unordered list has ordered=0' );
    is( $list->{tight},   1,        'tight list has tight=1' );
    is( $list->{start},   undef,    'unordered list has start=undef' );
}

# ===== Test 8: make_list (ordered) =====
{
    my $list = make_list(1, 1, 0, []);
    is( $list->{ordered}, 1, 'ordered list has ordered=1' );
    is( $list->{start},   1, 'ordered list has start=1' );
}

# ===== Test 9: make_list_item =====
{
    my $li = make_list_item([make_text('item')]);
    is( $li->{type}, 'list_item', 'make_list_item sets type' );
    is( $li->{children}[0]{type}, 'text', 'list item has text child' );
}

# ===== Test 10: make_task_item =====
{
    my $ti = make_task_item(1, [make_text('done')]);
    is( $ti->{type},    'task_item', 'make_task_item sets type' );
    is( $ti->{checked}, 1,           'checked task item has checked=1' );

    my $ti2 = make_task_item(0, []);
    is( $ti2->{checked}, 0, 'unchecked task item has checked=0' );
}

# ===== Test 11: make_thematic_break =====
{
    my $hr = make_thematic_break();
    is( $hr->{type}, 'thematic_break', 'make_thematic_break sets type' );
    ok( !exists $hr->{children} || !defined $hr->{children},
        'thematic_break has no children field' );
}

# ===== Test 12: make_raw_block =====
{
    my $rb = make_raw_block('html', '<div>raw</div>');
    is( $rb->{type},   'raw_block',       'make_raw_block sets type' );
    is( $rb->{format}, 'html',            'raw_block has format' );
    is( $rb->{value},  '<div>raw</div>',  'raw_block has value' );
}

# ===== Test 13: make_text =====
{
    my $t = make_text('Hello');
    is( $t->{type},  'text',  'make_text sets type' );
    is( $t->{value}, 'Hello', 'make_text sets value' );
}

# ===== Test 14: make_emphasis =====
{
    my $em = make_emphasis([make_text('italic')]);
    is( $em->{type}, 'emphasis', 'make_emphasis sets type' );
    is( $em->{children}[0]{value}, 'italic', 'emphasis contains text child' );
}

# ===== Test 15: make_strong =====
{
    my $s = make_strong([make_text('bold')]);
    is( $s->{type}, 'strong', 'make_strong sets type' );
}

# ===== Test 16: make_strikethrough =====
{
    my $st = make_strikethrough([make_text('struck')]);
    is( $st->{type}, 'strikethrough', 'make_strikethrough sets type' );
}

# ===== Test 17: make_code_span =====
{
    my $cs = make_code_span('const x = 1');
    is( $cs->{type},  'code_span',   'make_code_span sets type' );
    is( $cs->{value}, 'const x = 1', 'make_code_span sets value' );
}

# ===== Test 18: make_link =====
{
    my $link = make_link('https://example.com', 'Example', [make_text('click')]);
    is( $link->{type},        'link',                 'make_link sets type' );
    is( $link->{destination}, 'https://example.com',  'make_link sets destination' );
    is( $link->{title},       'Example',              'make_link sets title' );
    is( $link->{children}[0]{value}, 'click',         'make_link has text child' );
}

# ===== Test 19: make_link with undef title =====
{
    my $link = make_link('https://example.com', undef, []);
    is( $link->{title}, undef, 'make_link allows undef title' );
}

# ===== Test 20: make_image =====
{
    my $img = make_image('cat.png', undef, 'a cat');
    is( $img->{type},        'image', 'make_image sets type' );
    is( $img->{destination}, 'cat.png', 'make_image sets destination' );
    is( $img->{alt},         'a cat',   'make_image sets alt' );
    is( $img->{title},       undef,     'make_image allows undef title' );
}

# ===== Test 21: make_autolink =====
{
    my $al = make_autolink('user@example.com', 1);
    is( $al->{type},        'autolink',         'make_autolink sets type' );
    is( $al->{destination}, 'user@example.com', 'make_autolink sets destination' );
    is( $al->{is_email},    1,                  'make_autolink sets is_email' );
}

# ===== Test 22: make_hard_break and make_soft_break =====
{
    my $hb = make_hard_break();
    is( $hb->{type}, 'hard_break', 'make_hard_break sets type' );

    my $sb = make_soft_break();
    is( $sb->{type}, 'soft_break', 'make_soft_break sets type' );
}

# ===== Test 23: is_block and is_inline =====
{
    is( is_block(make_document([])),'1', 'document is a block node' );
    is( is_block(make_heading(1, [])), '1', 'heading is a block node' );
    is( is_block(make_paragraph([])),  '1', 'paragraph is a block node' );
    is( is_block(make_text('x')),      '0', 'text is not a block node' );
    is( is_inline(make_text('x')),     '1', 'text is an inline node' );
    is( is_inline(make_emphasis([])),  '1', 'emphasis is an inline node' );
    is( is_inline(make_paragraph([])), '0', 'paragraph is not an inline node' );
}

# ===== Test 24: node_type =====
{
    is( node_type(make_heading(2, [])), 'heading',   'node_type returns heading' );
    is( node_type(make_text('hi')),     'text',      'node_type returns text' );
    is( node_type(undef),               undef,       'node_type(undef) returns undef' );
}

# ===== Test 25: walk visits all nodes =====
{
    my $doc = make_document([
        make_heading(1, [make_text('Title')]),
        make_paragraph([make_text('Hello '), make_emphasis([make_text('world')])]),
    ]);

    my @visited_types;
    walk($doc, sub {
        my ($n) = @_;
        push @visited_types, $n->{type};
    });

    # Expected: document, heading, text, paragraph, text, emphasis, text
    is( scalar(@visited_types), 7, 'walk visits all 7 nodes' );
    is( $visited_types[0], 'document', 'walk starts with document' );
    is( $visited_types[1], 'heading',  'walk visits heading' );
    is( $visited_types[2], 'text',     'walk visits heading text child' );
}

# ===== Test 26: walk on leaf node =====
{
    my @types;
    walk(make_text('leaf'), sub { push @types, $_[0]->{type} });
    is( scalar(@types), 1, 'walk on leaf node visits exactly one node' );
    is( $types[0], 'text', 'visited node is the leaf' );
}

# ===== Test 27: make_raw_inline =====
{
    my $ri = make_raw_inline('html', '<em>raw</em>');
    is( $ri->{type},   'raw_inline',    'make_raw_inline sets type' );
    is( $ri->{format}, 'html',          'raw_inline has format' );
    is( $ri->{value},  '<em>raw</em>',  'raw_inline has value' );
}

done_testing;
