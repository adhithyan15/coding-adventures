use strict;
use warnings;
use Test2::V0;

use CodingAdventures::GfmParser;

# Convenience alias
sub parse { CodingAdventures::GfmParser::parse(@_) }

# Helper: parse and return the first child block
sub first_child {
    my ($md) = @_;
    return parse($md)->{children}[0];
}

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::GfmParser->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. Empty document
# ---------------------------------------------------------------------------
my $empty = parse('');
is($empty->{type}, 'document', 'empty: type is document');
is(scalar @{$empty->{children}}, 0, 'empty: no children');

# ---------------------------------------------------------------------------
# 3. ATX heading (inherited from CommonMark)
# ---------------------------------------------------------------------------
my $h2 = first_child("## Hello GFM\n");
is($h2->{type}, 'heading', 'ATX h2: type correct');
is($h2->{level}, 2, 'ATX h2: level correct');
is($h2->{children}[0]{value}, 'Hello GFM', 'ATX h2: content correct');

# ---------------------------------------------------------------------------
# 4. Paragraph
# ---------------------------------------------------------------------------
my $para = first_child("Simple paragraph.\n");
is($para->{type}, 'paragraph', 'paragraph type correct');

# ---------------------------------------------------------------------------
# 5. Thematic break
# ---------------------------------------------------------------------------
my $hr = first_child("---\n");
is($hr->{type}, 'thematic_break', 'thematic_break recognized');

# ---------------------------------------------------------------------------
# 6. Fenced code block
# ---------------------------------------------------------------------------
my $cb = first_child("```python\nprint('hi')\n```\n");
is($cb->{type}, 'code_block', 'fenced code: type correct');
is($cb->{language}, 'python', 'fenced code: language correct');
like($cb->{value}, qr/print/, 'fenced code: value contains code');

# ---------------------------------------------------------------------------
# 7. Blockquote
# ---------------------------------------------------------------------------
my $bq = first_child("> blockquoted\n");
is($bq->{type}, 'blockquote', 'blockquote type correct');

# ---------------------------------------------------------------------------
# 8. Bullet list
# ---------------------------------------------------------------------------
my $ul = first_child("- alpha\n- beta\n");
is($ul->{type}, 'list', 'bullet list type correct');
is($ul->{ordered}, 0, 'bullet list ordered=0');
is(scalar @{$ul->{children}}, 2, 'bullet list has 2 items');

# ---------------------------------------------------------------------------
# 9. Ordered list
# ---------------------------------------------------------------------------
my $ol = first_child("1. first\n2. second\n3. third\n");
is($ol->{type}, 'list', 'ordered list type correct');
is($ol->{ordered}, 1, 'ordered list ordered=1');
is(scalar @{$ol->{children}}, 3, 'ordered list has 3 items');

# ---------------------------------------------------------------------------
# 10. GFM Task list — unchecked
# ---------------------------------------------------------------------------
my $task_doc = parse("- [ ] unchecked task\n");
my $task_list = $task_doc->{children}[0];
is($task_list->{type}, 'list', 'task list: type is list');
my $task_item = $task_list->{children}[0];
is($task_item->{type}, 'task_item', 'task item: type is task_item');
is($task_item->{checked}, 0, 'unchecked task: checked=0');

# ---------------------------------------------------------------------------
# 11. GFM Task list — checked (lowercase x)
# ---------------------------------------------------------------------------
my $checked_doc = parse("- [x] checked task\n");
my $checked_item = $checked_doc->{children}[0]{children}[0];
is($checked_item->{type}, 'task_item', 'checked task: type is task_item');
is($checked_item->{checked}, 1, 'checked task: checked=1');

# ---------------------------------------------------------------------------
# 12. GFM Task list — checked (uppercase X)
# ---------------------------------------------------------------------------
my $checked_upper = parse("- [X] also checked\n");
my $upper_item = $checked_upper->{children}[0]{children}[0];
is($upper_item->{checked}, 1, 'uppercase X: checked=1');

# ---------------------------------------------------------------------------
# 13. GFM Task list — mixed items in one list
# ---------------------------------------------------------------------------
my $mixed_doc = parse("- [ ] todo\n- [x] done\n- regular\n");
my $mixed_list = $mixed_doc->{children}[0];
is($mixed_list->{children}[0]{type}, 'task_item',  'mixed list item 0 is task_item');
is($mixed_list->{children}[1]{type}, 'task_item',  'mixed list item 1 is task_item');
is($mixed_list->{children}[2]{type}, 'list_item',  'mixed list item 2 is regular list_item');

# ---------------------------------------------------------------------------
# 14. GFM Strikethrough ~~text~~
# ---------------------------------------------------------------------------
my $strike_para = first_child("~~deleted text~~\n");
my $strike = $strike_para->{children}[0];
is($strike->{type}, 'strikethrough', 'strikethrough: type correct');
is($strike->{children}[0]{value}, 'deleted text', 'strikethrough: content correct');

# ---------------------------------------------------------------------------
# 15. GFM Strikethrough in mixed text
# ---------------------------------------------------------------------------
my $mix_para = first_child("before ~~gone~~ after\n");
my @mix_children = @{$mix_para->{children}};
my $has_strike = grep { $_->{type} eq 'strikethrough' } @mix_children;
ok($has_strike, 'strikethrough found in mixed text');

# ---------------------------------------------------------------------------
# 16. GFM Table — basic structure
# ---------------------------------------------------------------------------
my $table_md = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |\n";
my $table = first_child($table_md);
is($table->{type}, 'table', 'table: type correct');
is(scalar @{$table->{children}}, 3, 'table: 1 header + 2 body rows');

# ---------------------------------------------------------------------------
# 17. Table — header row
# ---------------------------------------------------------------------------
my $header_row = $table->{children}[0];
is($header_row->{type}, 'table_row', 'table header: type is table_row');
is($header_row->{is_header}, 1, 'table header: is_header=1');
is(scalar @{$header_row->{children}}, 2, 'table header: 2 cells');

# ---------------------------------------------------------------------------
# 18. Table — body rows
# ---------------------------------------------------------------------------
my $body_row = $table->{children}[1];
is($body_row->{is_header}, 0, 'table body row: is_header=0');
is($body_row->{children}[0]{type}, 'table_cell', 'table body: cell type correct');

# ---------------------------------------------------------------------------
# 19. Table — cell content
# ---------------------------------------------------------------------------
my $name_cell = $header_row->{children}[0];
is($name_cell->{children}[0]{value}, 'Name', 'table header cell: content correct');

# ---------------------------------------------------------------------------
# 20. GFM Autolink — URL
# ---------------------------------------------------------------------------
my $url_para = first_child("<https://example.com>\n");
my $autolink = $url_para->{children}[0];
is($autolink->{type}, 'autolink', 'URL autolink: type correct');
is($autolink->{destination}, 'https://example.com', 'URL autolink: destination correct');
is($autolink->{is_email}, 0, 'URL autolink: is_email=0');

# ---------------------------------------------------------------------------
# 21. GFM Autolink — email
# ---------------------------------------------------------------------------
my $email_para = first_child('<user@example.com>' . "\n");
my $email_link = $email_para->{children}[0];
is($email_link->{type}, 'autolink', 'email autolink: type correct');
is($email_link->{destination}, 'user@example.com', 'email autolink: destination correct');
is($email_link->{is_email}, 1, 'email autolink: is_email=1');

# ---------------------------------------------------------------------------
# 22. Bold (inherited from CommonMark)
# ---------------------------------------------------------------------------
my $bold = first_child("**bold**\n")->{children}[0];
is($bold->{type}, 'strong', 'bold: type is strong');

# ---------------------------------------------------------------------------
# 23. Italic (inherited from CommonMark)
# ---------------------------------------------------------------------------
my $italic = first_child("*italic*\n")->{children}[0];
is($italic->{type}, 'emphasis', 'italic: type is emphasis');

# ---------------------------------------------------------------------------
# 24. Inline code (inherited)
# ---------------------------------------------------------------------------
my $code_para = first_child("`code`\n");
is($code_para->{children}[0]{type}, 'code_span', 'inline code: type correct');

# ---------------------------------------------------------------------------
# 25. Link (inherited)
# ---------------------------------------------------------------------------
my $link_para = first_child("[text](https://example.com)\n");
is($link_para->{children}[0]{type}, 'link', 'link: type correct');
is($link_para->{children}[0]{destination}, 'https://example.com', 'link: destination correct');

# ---------------------------------------------------------------------------
# 26. Image (inherited)
# ---------------------------------------------------------------------------
my $img_para = first_child("![alt](https://example.com/img.png)\n");
is($img_para->{children}[0]{type}, 'image', 'image: type correct');
is($img_para->{children}[0]{alt}, 'alt', 'image: alt correct');

# ---------------------------------------------------------------------------
# 27. Multiple GFM extensions in one document
# ---------------------------------------------------------------------------
my $complex = parse(
    "# GFM Test\n\n" .
    "~~strikethrough~~ and **bold**\n\n" .
    "- [x] task done\n" .
    "- [ ] task pending\n\n" .
    "| Col1 | Col2 |\n|------|------|\n| a | b |\n"
);
is(scalar @{$complex->{children}}, 4, 'complex doc: 4 top-level blocks');
is($complex->{children}[0]{type}, 'heading',   'complex: first block is heading');
is($complex->{children}[1]{type}, 'paragraph', 'complex: second block is paragraph');
is($complex->{children}[2]{type}, 'list',      'complex: third block is list');
is($complex->{children}[3]{type}, 'table',     'complex: fourth block is table');

# ---------------------------------------------------------------------------
# 28. Task item text content preserved
# ---------------------------------------------------------------------------
my $task_text_doc = parse("- [x] Buy groceries\n");
my $task_text_item = $task_text_doc->{children}[0]{children}[0];
is($task_text_item->{checked}, 1, 'task text: checked=1');
my $task_para = $task_text_item->{children}[0];
is($task_para->{type}, 'paragraph', 'task item: has paragraph child');
is($task_para->{children}[0]{value}, 'Buy groceries', 'task item: text content correct');

done_testing;
