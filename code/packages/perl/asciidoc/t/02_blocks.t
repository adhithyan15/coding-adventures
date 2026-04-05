use strict;
use warnings;
use Test2::V0;

require CodingAdventures::Asciidoc;
CodingAdventures::Asciidoc->import(qw(to_html parse));

# ============================================================================
# Block-level constructs
# ============================================================================

# ─── Code blocks ──────────────────────────────────────────────────────────────

subtest 'fenced code block ----' => sub {
    my $html = to_html("----\nsome code\n----\n");
    like($html, qr/<pre>/,      'has pre tag');
    like($html, qr/<code>/,     'has code tag');
    like($html, qr/some code/,  'contains code content');
};

subtest 'code block without language has no class' => sub {
    my $html = to_html("----\ncode\n----\n");
    unlike($html, qr/class=/, 'no class attribute');
};

subtest '[source,python] sets language class' => sub {
    my $html = to_html("[source,python]\n----\nx = 1\n----\n");
    like($html, qr/language-python/, 'language class present');
};

subtest '[source,ruby] sets language class' => sub {
    my $html = to_html("[source,ruby]\n----\nputs 'hi'\n----\n");
    like($html, qr/language-ruby/, 'ruby language class');
};

subtest 'code block preserves newlines' => sub {
    my $html = to_html("----\nline1\nline2\nline3\n----\n");
    like($html, qr/line1/, 'has line1');
    like($html, qr/line2/, 'has line2');
    like($html, qr/line3/, 'has line3');
};

subtest 'code block AST node' => sub {
    my $ast  = parse("----\nfoo\n----\n");
    my $node = $ast->[0];
    is($node->{type}, 'code_block', 'type is code_block');
    ok(defined $node->{value}, 'has value');
    like($node->{value}, qr/foo/, 'value contains content');
};

subtest 'literal block ....' => sub {
    my $html = to_html("....\nliteral text\n....\n");
    like($html, qr/<pre>/, 'has pre');
    like($html, qr/literal text/, 'has content');
};

# ─── Passthrough block ────────────────────────────────────────────────────────

subtest 'passthrough block ++++ passes raw content' => sub {
    my $html = to_html("++++\n<video src='x.mp4'/>\n++++\n");
    like($html, qr/video/, 'raw content passed through');
};

subtest 'passthrough block AST node type is raw_block' => sub {
    my $ast  = parse("++++\nraw\n++++\n");
    my $node = $ast->[0];
    is($node->{type}, 'raw_block', 'type is raw_block');
};

# ─── Quote block ──────────────────────────────────────────────────────────────

subtest '____ block produces blockquote' => sub {
    my $html = to_html("____\nA quote here\n____\n");
    like($html, qr/<blockquote>/, 'has blockquote');
    like($html, qr/A quote here/, 'has content');
};

subtest 'blockquote AST node type' => sub {
    my $ast  = parse("____\nquoted\n____\n");
    my $node = $ast->[0];
    is($node->{type}, 'blockquote', 'type is blockquote');
    ok(ref($node->{children}) eq 'ARRAY', 'children is array');
};

subtest 'blockquote children parsed recursively' => sub {
    my $ast  = parse("____\nSome text\n____\n");
    my $node = $ast->[0];
    is(scalar @{$node->{children}}, 1, 'one child block');
    is($node->{children}[0]{type}, 'paragraph', 'child is paragraph');
};

# ─── Unordered list ───────────────────────────────────────────────────────────

subtest '* items produce ul' => sub {
    my $html = to_html("* Alpha\n* Beta\n");
    like($html, qr/<ul>/, 'has ul');
    like($html, qr/<li>/, 'has li');
    like($html, qr/Alpha/, 'has Alpha');
    like($html, qr/Beta/,  'has Beta');
};

subtest 'unordered list AST node' => sub {
    my $ast  = parse("* One\n* Two\n");
    my $node = $ast->[0];
    is($node->{type},    'list', 'type is list');
    is($node->{ordered}, 0,      'ordered is false');
    is(scalar @{$node->{items}}, 2, 'two items');
};

subtest 'nested ** treated as same-level list item' => sub {
    my $html = to_html("* Top\n** Nested\n");
    my @li = ($html =~ /<li>/g);
    ok(scalar @li >= 2, 'at least two list items');
};

# ─── Ordered list ─────────────────────────────────────────────────────────────

subtest '. items produce ol' => sub {
    my $html = to_html(". First\n. Second\n");
    like($html, qr/<ol>/, 'has ol');
    like($html, qr/First/, 'has First');
};

subtest 'ordered list AST node' => sub {
    my $ast  = parse(". A\n. B\n");
    my $node = $ast->[0];
    is($node->{type},    'list', 'type is list');
    is($node->{ordered}, 1,      'ordered is true');
};

# ─── Mixed blocks ─────────────────────────────────────────────────────────────

subtest 'heading + paragraph + code block' => sub {
    my $src = "= Title\n\nIntro\n\n----\ncode\n----\n";
    my $ast = parse($src);
    is(scalar @$ast, 3, 'three blocks');
    is($ast->[0]{type}, 'heading',    'first is heading');
    is($ast->[1]{type}, 'paragraph',  'second is paragraph');
    is($ast->[2]{type}, 'code_block', 'third is code_block');
};

subtest 'HTML escaping in code block value' => sub {
    my $html = to_html("----\n<tag> & stuff\n----\n");
    like($html, qr/&lt;tag&gt;/, 'angle brackets escaped');
    like($html, qr/&amp;/,       'ampersand escaped');
};

done_testing;
