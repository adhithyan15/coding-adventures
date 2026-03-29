use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CommonmarkParser;

# Convenience alias
sub parse { CodingAdventures::CommonmarkParser::parse(@_) }

# Helper: parse and return the first child block
sub first_child {
    my ($md) = @_;
    return parse($md)->{children}[0];
}

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::CommonmarkParser->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. Empty document
# ---------------------------------------------------------------------------
my $empty = parse('');
is($empty->{type}, 'document', 'empty parse: type is document');
is(scalar @{$empty->{children}}, 0, 'empty parse: no children');

# ---------------------------------------------------------------------------
# 3. ATX heading level 1
# ---------------------------------------------------------------------------
my $h1 = first_child("# Hello\n");
is($h1->{type}, 'heading', 'ATX h1: type is heading');
is($h1->{level}, 1, 'ATX h1: level is 1');
is($h1->{children}[0]{value}, 'Hello', 'ATX h1: text content');

# ---------------------------------------------------------------------------
# 4. ATX heading level 3
# ---------------------------------------------------------------------------
my $h3 = first_child("### World\n");
is($h3->{level}, 3, 'ATX h3: level is 3');

# ---------------------------------------------------------------------------
# 5. ATX heading level 6
# ---------------------------------------------------------------------------
my $h6 = first_child("###### Six\n");
is($h6->{level}, 6, 'ATX h6: level is 6');

# ---------------------------------------------------------------------------
# 6. 7 hashes is not a heading (becomes a paragraph)
# ---------------------------------------------------------------------------
my $not_heading = first_child("####### Not heading\n");
is($not_heading->{type}, 'paragraph', '7 hashes is a paragraph');

# ---------------------------------------------------------------------------
# 7. ATX heading with trailing hashes stripped
# ---------------------------------------------------------------------------
my $h2_trailing = first_child("## Title ##\n");
is($h2_trailing->{children}[0]{value}, 'Title', 'trailing hashes stripped');

# ---------------------------------------------------------------------------
# 8. Paragraph
# ---------------------------------------------------------------------------
my $para = first_child("Hello world\n");
is($para->{type}, 'paragraph', 'paragraph: type correct');
is($para->{children}[0]{value}, 'Hello world', 'paragraph: text content');

# ---------------------------------------------------------------------------
# 9. Thematic break ---
# ---------------------------------------------------------------------------
my $hr = first_child("---\n");
is($hr->{type}, 'thematic_break', 'thematic break --- recognized');

# ---------------------------------------------------------------------------
# 10. Thematic break ***
# ---------------------------------------------------------------------------
my $hr2 = first_child("***\n");
is($hr2->{type}, 'thematic_break', 'thematic break *** recognized');

# ---------------------------------------------------------------------------
# 11. Thematic break ___
# ---------------------------------------------------------------------------
my $hr3 = first_child("___\n");
is($hr3->{type}, 'thematic_break', 'thematic break ___ recognized');

# ---------------------------------------------------------------------------
# 12. Fenced code block
# ---------------------------------------------------------------------------
my $code = first_child("```\nmy \$x = 1;\n```\n");
is($code->{type}, 'code_block', 'fenced code: type correct');
is($code->{value}, "my \$x = 1;\n", 'fenced code: value correct');

# ---------------------------------------------------------------------------
# 13. Fenced code block with language
# ---------------------------------------------------------------------------
my $code_lang = first_child("```perl\nsay 'hi';\n```\n");
is($code_lang->{language}, 'perl', 'fenced code: language tag extracted');

# ---------------------------------------------------------------------------
# 14. Blockquote
# ---------------------------------------------------------------------------
my $bq = first_child("> quoted text\n");
is($bq->{type}, 'blockquote', 'blockquote: type correct');
is($bq->{children}[0]{type}, 'paragraph', 'blockquote: contains paragraph');
is($bq->{children}[0]{children}[0]{value}, 'quoted text', 'blockquote: text content');

# ---------------------------------------------------------------------------
# 15. Bullet list
# ---------------------------------------------------------------------------
my $ul_doc = parse("- item one\n- item two\n");
my $ul = $ul_doc->{children}[0];
is($ul->{type}, 'list', 'bullet list: type correct');
is($ul->{ordered}, 0, 'bullet list: ordered=0');
is(scalar @{$ul->{children}}, 2, 'bullet list: 2 items');
is($ul->{children}[0]{type}, 'list_item', 'bullet list: item type correct');

# ---------------------------------------------------------------------------
# 16. Ordered list
# ---------------------------------------------------------------------------
my $ol_doc = parse("1. first\n2. second\n");
my $ol = $ol_doc->{children}[0];
is($ol->{type}, 'list', 'ordered list: type correct');
is($ol->{ordered}, 1, 'ordered list: ordered=1');
is($ol->{start}, 1, 'ordered list: start=1');
is(scalar @{$ol->{children}}, 2, 'ordered list: 2 items');

# ---------------------------------------------------------------------------
# 17. Multiple blocks separated by blank lines
# ---------------------------------------------------------------------------
my $multi = parse("# Heading\n\nParagraph text.\n");
is(scalar @{$multi->{children}}, 2, 'heading and paragraph both parsed');
is($multi->{children}[0]{type}, 'heading', 'first block is heading');
is($multi->{children}[1]{type}, 'paragraph', 'second block is paragraph');

# ---------------------------------------------------------------------------
# 18. Bold inline **text**
# ---------------------------------------------------------------------------
my $bold_para = first_child("**bold text**\n");
my $strong = $bold_para->{children}[0];
is($strong->{type}, 'strong', 'bold: type is strong');
is($strong->{children}[0]{value}, 'bold text', 'bold: text content correct');

# ---------------------------------------------------------------------------
# 19. Italic inline *text*
# ---------------------------------------------------------------------------
my $em_para = first_child("*italic text*\n");
my $em = $em_para->{children}[0];
is($em->{type}, 'emphasis', 'italic: type is emphasis');
is($em->{children}[0]{value}, 'italic text', 'italic: text content correct');

# ---------------------------------------------------------------------------
# 20. Inline code span `code`
# ---------------------------------------------------------------------------
my $code_para = first_child("Use \`printf\` please\n");
my $has_code_span = 0;
for my $child (@{$code_para->{children}}) {
    $has_code_span = 1 if $child->{type} eq 'code_span' && $child->{value} eq 'printf';
}
ok($has_code_span, 'inline code span recognized');

# ---------------------------------------------------------------------------
# 21. Link [text](url)
# ---------------------------------------------------------------------------
my $link_para = first_child("[click here](https://example.com)\n");
my $link = $link_para->{children}[0];
is($link->{type}, 'link', 'link: type correct');
is($link->{destination}, 'https://example.com', 'link: destination correct');
is($link->{children}[0]{value}, 'click here', 'link: text content correct');

# ---------------------------------------------------------------------------
# 22. Image ![alt](url)
# ---------------------------------------------------------------------------
my $img_para = first_child("![alt text](https://example.com/img.png)\n");
my $img = $img_para->{children}[0];
is($img->{type}, 'image', 'image: type correct');
is($img->{destination}, 'https://example.com/img.png', 'image: destination correct');
is($img->{alt}, 'alt text', 'image: alt text correct');

# ---------------------------------------------------------------------------
# 23. Hard line break (two trailing spaces)
# ---------------------------------------------------------------------------
my $hb_para = first_child("line one  \nline two\n");
my @hb_children = @{$hb_para->{children}};
my $has_hard_break = grep { $_->{type} eq 'hard_break' } @hb_children;
ok($has_hard_break, 'two trailing spaces produce hard_break');

# ---------------------------------------------------------------------------
# 24. Setext heading level 1 (underlined with ===)
# ---------------------------------------------------------------------------
my $setext1 = first_child("Title\n=====\n");
is($setext1->{type}, 'heading', 'setext h1: type is heading');
is($setext1->{level}, 1, 'setext h1: level is 1');

# ---------------------------------------------------------------------------
# 25. Setext heading level 2 (underlined with ---)
# ---------------------------------------------------------------------------
my $setext2 = first_child("Subtitle\n--------\n");
is($setext2->{type}, 'heading', 'setext h2: type is heading');
is($setext2->{level}, 2, 'setext h2: level is 2');

# ---------------------------------------------------------------------------
# 26. Link with title [text](url "title")
# ---------------------------------------------------------------------------
my $link_title = first_child("[text](https://example.com \"My Title\")\n");
my $lt = $link_title->{children}[0];
is($lt->{title}, 'My Title', 'link title parsed correctly');

# ---------------------------------------------------------------------------
# 27. Soft break between paragraph lines
# ---------------------------------------------------------------------------
my $sb_para = first_child("line one\nline two\n");
my @sb_children = @{$sb_para->{children}};
my $has_soft_break = grep { $_->{type} eq 'soft_break' } @sb_children;
ok($has_soft_break, 'newline within paragraph produces soft_break');

# ---------------------------------------------------------------------------
# 28. Backslash escape \* produces literal *
# ---------------------------------------------------------------------------
my $esc_para = first_child("\\*not italic\\*\n");
is($esc_para->{children}[0]{value}, '*not italic*', 'backslash escape works');

done_testing;
