use strict;
use warnings;
use Test2::V0;

use CodingAdventures::AsciidocParser;

sub parse {
    return CodingAdventures::AsciidocParser::parse(@_);
}

sub first_child {
    return parse($_[0])->{children}[0];
}

sub find_inline {
    my ($children, $type) = @_;
    for my $node (@$children) {
        return $node if ($node->{type} // '') eq $type;
    }
    return undef;
}

subtest 'document root' => sub {
    my $doc = parse('');
    is($doc->{type}, 'document', 'document node');
    is($doc->{children}, [], 'empty input has no children');
};

subtest 'headings and paragraphs' => sub {
    my $doc = parse("= Title\n\nHello world\n");
    is(scalar @{ $doc->{children} }, 2, 'two top-level blocks');
    is($doc->{children}[0]{type}, 'heading', 'first block heading');
    is($doc->{children}[0]{level}, 1, 'heading level');
    is($doc->{children}[1]{type}, 'paragraph', 'second block paragraph');
    ok(find_inline($doc->{children}[0]{children}, 'text'), 'heading has text inline');
};

subtest 'code blocks and thematic breaks' => sub {
    my $code = first_child("[source,perl]\n----\nsay 'hi';\n----\n");
    is($code->{type}, 'code_block', 'code block');
    is($code->{language}, 'perl', 'language retained');
    like($code->{value}, qr/say 'hi'/, 'code value retained');

    my $rule = first_child("'''\n");
    is($rule->{type}, 'thematic_break', 'thematic break');
};

subtest 'lists use shared list_item children shape' => sub {
    my $list = first_child("* Alpha\n* Beta\n");
    is($list->{type}, 'list', 'list node');
    is($list->{ordered}, 0, 'unordered list');
    is(scalar @{ $list->{children} }, 2, 'two list items');
    is($list->{children}[0]{type}, 'list_item', 'list item node');
    is($list->{children}[0]{children}[0]{type}, 'paragraph', 'list item wraps paragraph');
};

subtest 'inline shape matches document AST convention' => sub {
    my $para = first_child("*bold* _italic_ `code` link:https://example.com[site] image:photo.png[Photo]\n");
    ok(find_inline($para->{children}, 'strong'), 'strong node');
    ok(find_inline($para->{children}, 'emphasis'), 'emphasis node');
    ok(find_inline($para->{children}, 'code_span'), 'code span node');

    my $link = find_inline($para->{children}, 'link');
    ok($link, 'link node');
    is($link->{destination}, 'https://example.com', 'link destination');

    my $image = find_inline($para->{children}, 'image');
    ok($image, 'image node');
    is($image->{destination}, 'photo.png', 'image destination');
    is($image->{alt}, 'Photo', 'image alt');
};

subtest 'blockquote children are converted recursively' => sub {
    my $quote = first_child("____\nA quote\n____\n");
    is($quote->{type}, 'blockquote', 'blockquote');
    is($quote->{children}[0]{type}, 'paragraph', 'quote child paragraph');
};

done_testing;
