use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Commonmark; 1 }, 'module loads');

CodingAdventures::Commonmark->import(qw(render parse to_html render_safe));

# ============================================================================
# render() — high-level
# ============================================================================

subtest 'render returns string' => sub {
    my $html = render("Hello\n");
    ok(defined $html, 'returns something');
    like($html, qr/Hello/, 'contains text');
};

subtest 'render empty string' => sub {
    my $html = render('');
    ok(defined $html, 'empty input returns defined');
    is($html, '', 'empty input returns empty string');
};

# ============================================================================
# ATX headings
# ============================================================================

subtest 'h1 heading' => sub {
    my $html = render("# Hello\n");
    like($html, qr/<h1>Hello<\/h1>/, 'h1 heading');
};

subtest 'h2 heading' => sub {
    my $html = render("## World\n");
    like($html, qr/<h2>World<\/h2>/, 'h2 heading');
};

subtest 'h6 heading' => sub {
    my $html = render("###### Deep\n");
    like($html, qr/<h6>Deep<\/h6>/, 'h6 heading');
};

# ============================================================================
# Paragraphs
# ============================================================================

subtest 'paragraph' => sub {
    my $html = render("This is a paragraph.\n");
    like($html, qr/<p>This is a paragraph\.<\/p>/, 'paragraph');
};

subtest 'multiple paragraphs' => sub {
    my $html = render("First\n\nSecond\n");
    like($html, qr/<p>First<\/p>/, 'first para');
    like($html, qr/<p>Second<\/p>/, 'second para');
};

# ============================================================================
# Inline formatting
# ============================================================================

subtest 'bold' => sub {
    my $html = render("**bold text**\n");
    like($html, qr/<strong>bold text<\/strong>/, 'bold');
};

subtest 'italic' => sub {
    my $html = render("*italic text*\n");
    like($html, qr/<em>italic text<\/em>/, 'italic');
};

subtest 'inline code' => sub {
    my $html = render("`foo()`\n");
    like($html, qr/<code>foo\(\)<\/code>/, 'inline code');
};

# ============================================================================
# Links and images
# ============================================================================

subtest 'link' => sub {
    my $html = render("[click me](https://example.com)\n");
    like($html, qr/<a href="https:\/\/example\.com">click me<\/a>/, 'link');
};

subtest 'image' => sub {
    my $html = render("![alt text](pic.png)\n");
    like($html, qr/<img src="pic\.png" alt="alt text"/, 'image');
};

# ============================================================================
# Code blocks
# ============================================================================

subtest 'fenced code block' => sub {
    my $html = render("```\ncode here\n```\n");
    like($html, qr/<pre><code>/, 'has pre/code');
    like($html, qr/code here/,   'contains code');
};

subtest 'fenced code block with language' => sub {
    my $html = render("```perl\nmy \$x = 1;\n```\n");
    like($html, qr/language-perl/, 'has language class');
};

subtest 'indented code block' => sub {
    my $html = render("    indented\n");
    like($html, qr/<pre><code>/, 'has pre/code');
    like($html, qr/indented/,    'contains text');
};

# ============================================================================
# Lists
# ============================================================================

subtest 'unordered list' => sub {
    my $html = render("- item 1\n- item 2\n");
    like($html, qr/<ul>/, 'has ul');
    like($html, qr/<li>/, 'has li');
    like($html, qr/item 1/, 'first item');
    like($html, qr/item 2/, 'second item');
};

subtest 'ordered list' => sub {
    my $html = render("1. first\n2. second\n");
    like($html, qr/<ol>/, 'has ol');
    like($html, qr/first/, 'first item');
};

# ============================================================================
# Blockquotes
# ============================================================================

subtest 'blockquote' => sub {
    my $html = render("> quoted text\n");
    like($html, qr/<blockquote>/, 'has blockquote');
    like($html, qr/quoted text/,  'has text');
};

# ============================================================================
# Thematic breaks
# ============================================================================

subtest 'thematic break ---' => sub {
    my $html = render("---\n");
    like($html, qr/<hr/, 'has hr');
};

# ============================================================================
# HTML escaping
# ============================================================================

subtest 'HTML entities in text' => sub {
    my $html = render("a < b & c > d\n");
    like($html, qr/&lt;/, 'less-than escaped');
    like($html, qr/&amp;/, 'ampersand escaped');
    like($html, qr/&gt;/, 'greater-than escaped');
};

# ============================================================================
# parse() and to_html()
# ============================================================================

subtest 'parse returns AST' => sub {
    my $ast = parse("# Hello\n");
    is($ast->{type}, 'document', 'root is document');
    ok(ref($ast->{children}) eq 'ARRAY', 'children is array');
};

subtest 'parse heading AST node' => sub {
    my $ast  = parse("# Title\n");
    my $node = $ast->{children}[0];
    is($node->{type},  'heading', 'type is heading');
    is($node->{level}, 1,         'level is 1');
};

subtest 'to_html from AST' => sub {
    my $ast  = parse("## Sub\n");
    my $html = to_html($ast);
    like($html, qr/<h2>Sub<\/h2>/, 'renders heading');
};

# ============================================================================
# render_safe() — sanitization
# ============================================================================

subtest 'render_safe strips script tags' => sub {
    my $html = render_safe("<script>alert('xss')</script>\n");
    unlike($html, qr/<script>/, 'script tag removed');
};

subtest 'render_safe strips onclick' => sub {
    my $html = render_safe("[click](http://x.com) <div onclick=\"evil()\">x</div>\n");
    unlike($html, qr/onclick/, 'onclick removed');
};

# ============================================================================
# Setext headings
# ============================================================================

subtest 'setext h1' => sub {
    my $html = render("Title\n=====\n");
    like($html, qr/<h1>Title<\/h1>/, 'setext h1');
};

subtest 'setext h2' => sub {
    my $html = render("Sub\n---\n");
    like($html, qr/<h2>Sub<\/h2>/, 'setext h2');
};

done_testing;
