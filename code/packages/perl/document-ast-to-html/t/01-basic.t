use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DocumentAstToHtml;

# Convenience aliases
sub render   { CodingAdventures::DocumentAstToHtml::render(@_) }
sub to_html  { CodingAdventures::DocumentAstToHtml::to_html(@_) }
sub escape   { CodingAdventures::DocumentAstToHtml::escape(@_) }

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::DocumentAstToHtml->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2-5. escape() function
# ---------------------------------------------------------------------------
is(escape('hello'),          'hello',               'escape: no special chars');
is(escape('<script>'),       '&lt;script&gt;',      'escape: < and >');
is(escape('"quote"'),        '&quot;quote&quot;',   'escape: double quotes');
is(escape('a & b'),          'a &amp; b',           'escape: ampersand');

# ---------------------------------------------------------------------------
# 6. render() document node
# ---------------------------------------------------------------------------
my $doc = {type => 'document', children => []};
is(render($doc), '', 'empty document renders to empty string');

# ---------------------------------------------------------------------------
# 7-12. Heading nodes
# ---------------------------------------------------------------------------
for my $level (1..6) {
    my $d = {type => 'document', children => [{
        type     => 'heading',
        level    => $level,
        children => [{type => 'text', value => "H$level"}],
    }]};
    is(render($d), "<h$level>H$level</h$level>\n", "heading level $level");
}

# ---------------------------------------------------------------------------
# 13. Paragraph
# ---------------------------------------------------------------------------
my $para_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{type => 'text', value => 'Hello world'}],
}]};
is(render($para_doc), "<p>Hello world</p>\n", 'paragraph renders with <p> tags');

# ---------------------------------------------------------------------------
# 14. Code block without language
# ---------------------------------------------------------------------------
my $code_doc = {type => 'document', children => [{
    type     => 'code_block',
    language => '',
    value    => "say 'hi';\n",
}]};
is(render($code_doc), "<pre><code>say 'hi';\n</code></pre>\n", 'code block without language');

# ---------------------------------------------------------------------------
# 15. Code block with language
# ---------------------------------------------------------------------------
my $code_lang_doc = {type => 'document', children => [{
    type     => 'code_block',
    language => 'perl',
    value    => "say 'hi';\n",
}]};
is(render($code_lang_doc), "<pre><code class=\"language-perl\">say 'hi';\n</code></pre>\n", 'code block with language');

# ---------------------------------------------------------------------------
# 16. Thematic break
# ---------------------------------------------------------------------------
my $hr_doc = {type => 'document', children => [{type => 'thematic_break'}]};
is(render($hr_doc), "<hr />\n", 'thematic_break renders as <hr />');

# ---------------------------------------------------------------------------
# 17. Blockquote
# ---------------------------------------------------------------------------
my $bq_doc = {type => 'document', children => [{
    type     => 'blockquote',
    children => [{
        type     => 'paragraph',
        children => [{type => 'text', value => 'quoted'}],
    }],
}]};
is(render($bq_doc), "<blockquote>\n<p>quoted</p>\n</blockquote>\n", 'blockquote renders correctly');

# ---------------------------------------------------------------------------
# 18. Tight unordered list
# ---------------------------------------------------------------------------
my $ul_doc = {type => 'document', children => [{
    type     => 'list',
    ordered  => 0,
    tight    => 1,
    start    => 1,
    children => [
        {type => 'list_item', children => [{type => 'paragraph', children => [{type => 'text', value => 'a'}]}]},
        {type => 'list_item', children => [{type => 'paragraph', children => [{type => 'text', value => 'b'}]}]},
    ],
}]};
is(render($ul_doc), "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n", 'tight unordered list');

# ---------------------------------------------------------------------------
# 19. Ordered list with start
# ---------------------------------------------------------------------------
my $ol_doc = {type => 'document', children => [{
    type     => 'list',
    ordered  => 1,
    tight    => 1,
    start    => 3,
    children => [
        {type => 'list_item', children => [{type => 'paragraph', children => [{type => 'text', value => 'c'}]}]},
    ],
}]};
is(render($ol_doc), "<ol start=\"3\">\n<li>c</li>\n</ol>\n", 'ordered list with start=3');

# ---------------------------------------------------------------------------
# 20. Emphasis and Strong
# ---------------------------------------------------------------------------
my $em_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [
        {type => 'emphasis', children => [{type => 'text', value => 'em'}]},
        {type => 'text', value => ' and '},
        {type => 'strong', children => [{type => 'text', value => 'str'}]},
    ],
}]};
is(render($em_doc), "<p><em>em</em> and <strong>str</strong></p>\n", 'emphasis and strong');

# ---------------------------------------------------------------------------
# 21. Code span
# ---------------------------------------------------------------------------
my $cs_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{type => 'code_span', value => 'my $x = 1'}],
}]};
is(render($cs_doc), "<p><code>my \$x = 1</code></p>\n", 'code_span renders as <code>');

# ---------------------------------------------------------------------------
# 22. Link
# ---------------------------------------------------------------------------
my $link_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{
        type        => 'link',
        destination => 'https://example.com',
        title       => undef,
        children    => [{type => 'text', value => 'click'}],
    }],
}]};
is(render($link_doc), "<p><a href=\"https://example.com\">click</a></p>\n", 'link renders');

# ---------------------------------------------------------------------------
# 23. Link with title
# ---------------------------------------------------------------------------
my $link_title_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{
        type        => 'link',
        destination => 'https://example.com',
        title       => 'My Site',
        children    => [{type => 'text', value => 'here'}],
    }],
}]};
is(render($link_title_doc),
   "<p><a href=\"https://example.com\" title=\"My Site\">here</a></p>\n",
   'link with title');

# ---------------------------------------------------------------------------
# 24. Image
# ---------------------------------------------------------------------------
my $img_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{
        type        => 'image',
        destination => 'https://example.com/img.png',
        title       => undef,
        alt         => 'A photo',
    }],
}]};
is(render($img_doc),
   "<p><img src=\"https://example.com/img.png\" alt=\"A photo\" /></p>\n",
   'image renders');

# ---------------------------------------------------------------------------
# 25. Hard break and Soft break
# ---------------------------------------------------------------------------
my $break_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [
        {type => 'text', value => 'line1'},
        {type => 'hard_break'},
        {type => 'text', value => 'line2'},
        {type => 'soft_break'},
        {type => 'text', value => 'line3'},
    ],
}]};
is(render($break_doc), "<p>line1<br />\nline2\nline3</p>\n", 'hard_break and soft_break');

# ---------------------------------------------------------------------------
# 26. javascript: URL in link is sanitized
# ---------------------------------------------------------------------------
my $xss_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{
        type        => 'link',
        destination => 'javascript:alert(1)',
        title       => undef,
        children    => [{type => 'text', value => 'evil'}],
    }],
}]};
is(render($xss_doc), "<p><a href=\"\">evil</a></p>\n", 'javascript: URL sanitized');

# ---------------------------------------------------------------------------
# 27. raw_block with format=html passes through
# ---------------------------------------------------------------------------
my $raw_doc = {type => 'document', children => [{
    type   => 'raw_block',
    format => 'html',
    value  => "<div>raw</div>\n",
}]};
is(render($raw_doc), "<div>raw</div>\n", 'raw_block html passes through');

# ---------------------------------------------------------------------------
# 28. raw_block with sanitize=1 is stripped
# ---------------------------------------------------------------------------
is(render($raw_doc, {sanitize => 1}), '', 'raw_block stripped when sanitize=1');

# ---------------------------------------------------------------------------
# 29. to_html() alias works
# ---------------------------------------------------------------------------
is(to_html($para_doc), "<p>Hello world</p>\n", 'to_html() alias works');

# ---------------------------------------------------------------------------
# 30. Strikethrough (GFM extension)
# ---------------------------------------------------------------------------
my $del_doc = {type => 'document', children => [{
    type     => 'paragraph',
    children => [{type => 'strikethrough', children => [{type => 'text', value => 'deleted'}]}],
}]};
is(render($del_doc), "<p><del>deleted</del></p>\n", 'strikethrough renders as <del>');

# ---------------------------------------------------------------------------
# 31. Task item unchecked
# ---------------------------------------------------------------------------
my $task_doc = {type => 'document', children => [{
    type     => 'list',
    ordered  => 0,
    tight    => 1,
    start    => 1,
    children => [{
        type     => 'task_item',
        checked  => 0,
        children => [{type => 'paragraph', children => [{type => 'text', value => 'todo'}]}],
    }],
}]};
my $task_html = render($task_doc);
like($task_html, qr/type="checkbox"/, 'task item has checkbox');
like($task_html, qr/todo/,            'task item text present');

# ---------------------------------------------------------------------------
# 33. Task item checked
# ---------------------------------------------------------------------------
my $checked_doc = {type => 'document', children => [{
    type     => 'list',
    ordered  => 0,
    tight    => 1,
    start    => 1,
    children => [{
        type     => 'task_item',
        checked  => 1,
        children => [{type => 'paragraph', children => [{type => 'text', value => 'done'}]}],
    }],
}]};
my $checked_html = render($checked_doc);
like($checked_html, qr/checked=""/, 'checked task item has checked attribute');

# ---------------------------------------------------------------------------
# 34. Table rendering
# ---------------------------------------------------------------------------
my $table_doc = {type => 'document', children => [{
    type     => 'table',
    children => [
        {
            type      => 'table_row',
            is_header => 1,
            children  => [
                {type => 'table_cell', children => [{type => 'text', value => 'Name'}]},
                {type => 'table_cell', children => [{type => 'text', value => 'Age'}]},
            ],
        },
        {
            type      => 'table_row',
            is_header => 0,
            children  => [
                {type => 'table_cell', children => [{type => 'text', value => 'Alice'}]},
                {type => 'table_cell', children => [{type => 'text', value => '30'}]},
            ],
        },
    ],
}]};
my $table_html = render($table_doc);
like($table_html, qr/<table>/,              'table has <table>');
like($table_html, qr/<thead>/,              'table has <thead>');
like($table_html, qr/<tbody>/,              'table has <tbody>');
like($table_html, qr/<th>Name<\/th>/,       'header cell uses <th>');
like($table_html, qr/<td>Alice<\/td>/,      'body cell uses <td>');

done_testing;
