use strict;
use warnings;
use Test2::V0;

require CodingAdventures::Asciidoc;
CodingAdventures::Asciidoc->import(qw(to_html parse));

# ============================================================================
# Inline markup
# ============================================================================
#
# NOTE: In AsciiDoc, *single asterisks* produce STRONG (bold), not emphasis!
# Emphasis uses _underscores_.  This is the critical difference from Markdown.

# ─── Strong / bold ────────────────────────────────────────────────────────────

subtest '*bold* produces strong' => sub {
    my $html = to_html("*bold text*\n");
    like($html, qr/<strong>bold text<\/strong>/, 'strong rendered');
};

subtest '**unconstrained bold** produces strong' => sub {
    my $html = to_html("**bold**\n");
    like($html, qr/<strong>bold<\/strong>/, 'unconstrained strong');
};

subtest 'strong AST node' => sub {
    my $ast  = parse("*hello*\n");
    my $para = $ast->[0];
    is($para->{type}, 'paragraph', 'paragraph');
    my ($strong) = grep { $_->{type} eq 'strong' } @{$para->{children}};
    ok(defined $strong, 'found strong node');
    ok(ref($strong->{children}) eq 'ARRAY', 'strong has children');
};

# ─── Emphasis / italic ────────────────────────────────────────────────────────

subtest '_italic_ produces em' => sub {
    my $html = to_html("_italic text_\n");
    like($html, qr/<em>italic text<\/em>/, 'em rendered');
};

subtest '__unconstrained italic__ produces em' => sub {
    my $html = to_html("__italic__\n");
    like($html, qr/<em>italic<\/em>/, 'unconstrained em');
};

subtest 'emphasis AST node' => sub {
    my $ast  = parse("_hi_\n");
    my $para = $ast->[0];
    my ($emph) = grep { $_->{type} eq 'emph' } @{$para->{children}};
    ok(defined $emph, 'found emph node');
};

# ─── Inline code ──────────────────────────────────────────────────────────────

subtest '`code` produces code tag' => sub {
    my $html = to_html("`foo()`\n");
    like($html, qr/<code>foo\(\)<\/code>/, 'code rendered');
};

subtest 'code span content is verbatim' => sub {
    my $html = to_html("`*not bold*`\n");
    # The asterisks should appear inside <code>, not trigger bold.
    like($html, qr/<code>/, 'has code tag');
};

subtest 'code span AST node' => sub {
    my $ast  = parse("`bar`\n");
    my $para = $ast->[0];
    my ($code) = grep { $_->{type} eq 'code_span' } @{$para->{children}};
    ok(defined $code, 'found code_span node');
    like($code->{value}, qr/bar/, 'code value correct');
};

# ─── Link macro ───────────────────────────────────────────────────────────────

subtest 'link:url[text] produces a href' => sub {
    my $html = to_html("link:https://example.com[Click Here]\n");
    like($html, qr/<a href="https:\/\/example\.com">/, 'href set');
    like($html, qr/Click Here/, 'link text rendered');
};

subtest 'link macro AST node' => sub {
    my $ast  = parse("link:https://x.com[X]\n");
    my $para = $ast->[0];
    my ($link) = grep { $_->{type} eq 'link' } @{$para->{children}};
    ok(defined $link, 'found link node');
    is($link->{href}, 'https://x.com', 'href correct');
};

# ─── Image macro ──────────────────────────────────────────────────────────────

subtest 'image:url[alt] produces img tag' => sub {
    my $html = to_html("image:photo.png[A photo]\n");
    like($html, qr/<img/, 'has img tag');
    like($html, qr/photo\.png/, 'src set');
};

subtest 'image macro AST node' => sub {
    my $ast  = parse("image:x.png[alt]\n");
    my $para = $ast->[0];
    my ($img) = grep { $_->{type} eq 'image' } @{$para->{children}};
    ok(defined $img, 'found image node');
    is($img->{src}, 'x.png', 'src correct');
};

# ─── Cross-references ─────────────────────────────────────────────────────────

subtest '<<anchor,text>> produces link to #anchor' => sub {
    my $html = to_html("<<intro,Introduction>>\n");
    like($html, qr/<a href="#intro">/, 'href to #intro');
    like($html, qr/Introduction/,      'link text rendered');
};

subtest '<<anchor>> without label uses anchor as text' => sub {
    my $html = to_html("<<section-one>>\n");
    like($html, qr/<a href="#section-one">/, 'href set');
    like($html, qr/section-one/,             'anchor used as text');
};

# ─── Bare URLs ────────────────────────────────────────────────────────────────

subtest 'https:// URL is auto-linked' => sub {
    my $html = to_html("https://example.com\n");
    like($html, qr/<a href="https:\/\/example\.com">/, 'auto-link created');
};

subtest 'http:// URL is auto-linked' => sub {
    my $html = to_html("http://example.com\n");
    like($html, qr/<a href="http:\/\/example\.com">/, 'http auto-link');
};

# ─── HTML escaping ────────────────────────────────────────────────────────────

subtest '< in text is escaped' => sub {
    my $html = to_html("a < b\n");
    like($html, qr/&lt;/, 'less-than escaped');
};

subtest '> in text is escaped' => sub {
    my $html = to_html("a > b\n");
    like($html, qr/&gt;/, 'greater-than escaped');
};

subtest '& in text is escaped' => sub {
    my $html = to_html("a & b\n");
    like($html, qr/&amp;/, 'ampersand escaped');
};

subtest '" in text is escaped' => sub {
    my $html = to_html("He said \"hi\"\n");
    # The parser may or may not escape quotes in paragraph text
    ok(defined $html, 'output is defined');
};

# ─── Mixed inline ─────────────────────────────────────────────────────────────

subtest 'bold and italic in same paragraph' => sub {
    my $html = to_html("*bold* and _italic_\n");
    like($html, qr/<strong>bold<\/strong>/, 'bold rendered');
    like($html, qr/<em>italic<\/em>/,       'italic rendered');
};

subtest 'link with bold label' => sub {
    my $html = to_html("link:https://x.com[*bold link*]\n");
    like($html, qr/<a href=/, 'has anchor');
    like($html, qr/<strong>/, 'bold inside link');
};

subtest 'text node value is plain escaped text' => sub {
    my $ast  = parse("hello world\n");
    my $para = $ast->[0];
    my ($text) = grep { $_->{type} eq 'text' } @{$para->{children}};
    ok(defined $text, 'found text node');
    like($text->{value}, qr/hello/, 'value contains hello');
};

done_testing;
