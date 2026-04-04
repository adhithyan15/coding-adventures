use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Asciidoc; 1 }, 'module loads');
CodingAdventures::Asciidoc->import(qw(to_html parse));

# ============================================================================
# Basic API and empty/trivial input
# ============================================================================

subtest 'to_html is callable' => sub {
    ok(defined &to_html, 'to_html exported');
};

subtest 'parse is callable' => sub {
    ok(defined &parse, 'parse exported');
};

subtest 'to_html returns string for empty input' => sub {
    my $html = to_html('');
    ok(defined $html, 'returns defined value');
    is($html, '', 'empty input returns empty string');
};

subtest 'to_html returns string for whitespace-only input' => sub {
    my $html = to_html("   \n\n");
    ok(defined $html, 'returns defined value');
};

subtest 'parse returns arrayref' => sub {
    my $ast = parse('');
    ok(ref($ast) eq 'ARRAY', 'returns arrayref');
};

subtest 'parse empty returns empty arrayref' => sub {
    my $ast = parse('');
    is(scalar @$ast, 0, 'empty array');
};

# ============================================================================
# Headings
# ============================================================================

subtest 'h1 heading' => sub {
    my $html = to_html("= Hello\n");
    like($html, qr/<h1>.*Hello.*<\/h1>/s, 'h1 rendered');
};

subtest 'h2 heading' => sub {
    my $html = to_html("== Section\n");
    like($html, qr/<h2>.*Section.*<\/h2>/s, 'h2 rendered');
};

subtest 'h3 heading' => sub {
    my $html = to_html("=== Sub\n");
    like($html, qr/<h3>/, 'h3 rendered');
};

subtest 'h4 heading' => sub {
    my $html = to_html("==== Level4\n");
    like($html, qr/<h4>/, 'h4 rendered');
};

subtest 'h5 heading' => sub {
    my $html = to_html("===== Level5\n");
    like($html, qr/<h5>/, 'h5 rendered');
};

subtest 'h6 heading' => sub {
    my $html = to_html("====== Deep\n");
    like($html, qr/<h6>/, 'h6 rendered');
};

# ============================================================================
# Paragraph
# ============================================================================

subtest 'plain text paragraph' => sub {
    my $html = to_html("This is text.\n");
    like($html, qr/<p>/, 'has p tag');
    like($html, qr/This is text/, 'contains text');
};

subtest 'two paragraphs separated by blank line' => sub {
    my $html = to_html("First\n\nSecond\n");
    my @p = ($html =~ /<p>/g);
    is(scalar @p, 2, 'two paragraphs');
};

# ============================================================================
# Thematic break
# ============================================================================

subtest "''' produces hr" => sub {
    my $html = to_html("'''\n");
    like($html, qr/<hr/, 'has hr');
};

subtest "'''' (four quotes) also produces hr" => sub {
    my $html = to_html("''''\n");
    like($html, qr/<hr/, 'has hr');
};

# ============================================================================
# Comments (skipped)
# ============================================================================

subtest '// comment is not rendered' => sub {
    my $html = to_html("// hidden line\nVisible text\n");
    unlike($html, qr/hidden line/, 'comment not in output');
    like($html,   qr/Visible text/, 'non-comment is rendered');
};

# ============================================================================
# parse() AST node types
# ============================================================================

subtest 'parse heading node type' => sub {
    my $ast  = parse("= Title\n");
    my $node = $ast->[0];
    is($node->{type},  'heading', 'type is heading');
    is($node->{level}, 1,         'level is 1');
    ok(ref($node->{children}) eq 'ARRAY', 'children is array');
};

subtest 'parse paragraph node type' => sub {
    my $ast  = parse("Hello\n");
    my $node = $ast->[0];
    is($node->{type}, 'paragraph', 'type is paragraph');
};

done_testing;
