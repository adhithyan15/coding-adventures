use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Gfm; 1 }, 'module loads');

CodingAdventures::Gfm->import(qw(render parse to_html render_safe));

# ============================================================================
# GFM: Tables
# ============================================================================

subtest 'simple table' => sub {
    my $md   = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |\n";
    my $html = render($md);
    like($html, qr/<table>/,    'has table');
    like($html, qr/<thead>/,    'has thead');
    like($html, qr/<tbody>/,    'has tbody');
    like($html, qr/<th>/,       'has th');
    like($html, qr/<td>/,       'has td');
    like($html, qr/Alice/,      'has Alice');
    like($html, qr/Bob/,        'has Bob');
};

subtest 'table header row' => sub {
    my $md   = "| A | B |\n|---|---|\n| 1 | 2 |\n";
    my $html = render($md);
    like($html, qr/<th>.*A.*<\/th>/s, 'header A');
    like($html, qr/<th>.*B.*<\/th>/s, 'header B');
};

subtest 'table with column alignment' => sub {
    my $md   = "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |\n";
    my $html = render($md);
    like($html, qr/text-align: left/,   'left align');
    like($html, qr/text-align: center/, 'center align');
    like($html, qr/text-align: right/,  'right align');
};

# ============================================================================
# GFM: Task lists
# ============================================================================

subtest 'checked task item' => sub {
    my $html = render("- [x] Done\n");
    like($html, qr/type="checkbox"/, 'has checkbox');
    like($html, qr/checked/,         'is checked');
    like($html, qr/Done/,            'has text');
};

subtest 'unchecked task item' => sub {
    my $html = render("- [ ] Todo\n");
    like($html, qr/type="checkbox"/, 'has checkbox');
    unlike($html, qr/checked disabled.*checked|(?<!un)checked\b/,
           'not checked (or disabled without checked)');
    like($html, qr/Todo/, 'has text');
};

subtest 'mixed task list' => sub {
    my $html = render("- [x] Done\n- [ ] Not done\n");
    like($html, qr/Done/,     'has Done');
    like($html, qr/Not done/, 'has Not done');
    like($html, qr/<ul>/,     'is unordered list');
};

# ============================================================================
# GFM: Strikethrough
# ============================================================================

subtest 'strikethrough' => sub {
    my $html = render("~~struck text~~\n");
    like($html, qr/<del>struck text<\/del>/, 'has del tag');
};

subtest 'strikethrough inline' => sub {
    my $html = render("normal ~~struck~~ normal\n");
    like($html, qr/<del>struck<\/del>/, 'del inline');
};

# ============================================================================
# GFM: Autolinks
# ============================================================================

subtest 'autolink http' => sub {
    my $html = render("Visit https://example.com for more.\n");
    like($html, qr/<a href="https:\/\/example\.com"/, 'autolinked URL');
};

# ============================================================================
# CommonMark compatibility (inherited features)
# ============================================================================

subtest 'headings work' => sub {
    my $html = render("# H1\n## H2\n");
    like($html, qr/<h1>H1<\/h1>/, 'h1');
    like($html, qr/<h2>H2<\/h2>/, 'h2');
};

subtest 'paragraphs work' => sub {
    my $html = render("Hello world.\n\nSecond para.\n");
    like($html, qr/<p>Hello world\.<\/p>/, 'first para');
    like($html, qr/<p>Second para\.<\/p>/, 'second para');
};

subtest 'bold and italic work' => sub {
    my $html = render("**bold** and *italic*\n");
    like($html, qr/<strong>bold<\/strong>/, 'bold');
    like($html, qr/<em>italic<\/em>/,       'italic');
};

subtest 'inline code works' => sub {
    my $html = render("`code`\n");
    like($html, qr/<code>code<\/code>/, 'inline code');
};

subtest 'fenced code block works' => sub {
    my $html = render("```\nfoo\n```\n");
    like($html, qr/<pre><code>/, 'fenced code block');
};

subtest 'unordered list' => sub {
    my $html = render("- a\n- b\n");
    like($html, qr/<ul>/, 'ul');
    like($html, qr/<li>/, 'li');
};

subtest 'ordered list' => sub {
    my $html = render("1. first\n2. second\n");
    like($html, qr/<ol>/, 'ol');
};

subtest 'thematic break' => sub {
    my $html = render("---\n");
    like($html, qr/<hr/, 'hr');
};

subtest 'blockquote' => sub {
    my $html = render("> quoted\n");
    like($html, qr/<blockquote>/, 'blockquote');
};

# ============================================================================
# parse() and to_html() API
# ============================================================================

subtest 'parse returns AST document' => sub {
    my $ast = parse("# Hello\n");
    is($ast->{type}, 'document', 'root is document');
};

subtest 'to_html from AST' => sub {
    my $ast  = parse("## Sub\n");
    my $html = to_html($ast);
    like($html, qr/<h2>Sub<\/h2>/, 'renders correctly');
};

# ============================================================================
# render_safe()
# ============================================================================

subtest 'render_safe strips script' => sub {
    my $html = render_safe("<script>bad</script>\n");
    unlike($html, qr/<script>/, 'script removed');
};

subtest 'render_safe strips onclick' => sub {
    my $html = render_safe("[x](y) <div onclick=\"evil()\">x</div>\n");
    unlike($html, qr/onclick/, 'onclick removed');
};

# ============================================================================
# Table AST
# ============================================================================

subtest 'parse table node' => sub {
    my $ast  = parse("| A |\n|---|\n| 1 |\n");
    my $node = $ast->{children}[0];
    is($node->{type}, 'table', 'type is table');
    ok(defined $node->{rows},   'has rows');
    ok(defined $node->{aligns}, 'has aligns');
};

done_testing;
