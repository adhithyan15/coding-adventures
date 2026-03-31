use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::XmlLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::XmlLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub first_of {
    my ($tokens, $type) = @_;
    for my $tok (@$tokens) {
        return $tok if $tok->{type} eq $type;
    }
    return undef;
}

sub count_of {
    my ($tokens, $type) = @_;
    return scalar grep { $_->{type} eq $type } @$tokens;
}

# ============================================================================
# Empty input
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

# ============================================================================
# Self-closing tag
# ============================================================================

subtest 'self-closing tag <br/>' => sub {
    is(
        types_of('<br/>'),
        [qw(OPEN_TAG_START TAG_NAME SELF_CLOSE)],
        'three tokens'
    );

    my $tokens = CodingAdventures::XmlLexer->tokenize('<br/>');
    is( $tokens->[0]{value}, '<',   'OPEN_TAG_START value' );
    is( $tokens->[1]{value}, 'br',  'TAG_NAME value' );
    is( $tokens->[2]{value}, '/>',  'SELF_CLOSE value' );
};

# ============================================================================
# Opening tag
# ============================================================================

subtest 'opening tag <root>' => sub {
    is(
        types_of('<root>'),
        [qw(OPEN_TAG_START TAG_NAME TAG_CLOSE)],
        'three tokens for opening tag'
    );

    my $tokens = CodingAdventures::XmlLexer->tokenize('<root>');
    is( $tokens->[0]{value}, '<',    'OPEN_TAG_START value' );
    is( $tokens->[1]{value}, 'root', 'TAG_NAME value' );
    is( $tokens->[2]{value}, '>',    'TAG_CLOSE value' );
};

# ============================================================================
# Closing tag
# ============================================================================

subtest 'closing tag </root>' => sub {
    is(
        types_of('</root>'),
        [qw(CLOSE_TAG_START TAG_NAME TAG_CLOSE)],
        'three tokens for closing tag'
    );

    my $tokens = CodingAdventures::XmlLexer->tokenize('</root>');
    is( $tokens->[0]{value}, '</',   'CLOSE_TAG_START value' );
    is( $tokens->[1]{value}, 'root', 'TAG_NAME value' );
    is( $tokens->[2]{value}, '>',    'TAG_CLOSE value' );
};

# ============================================================================
# Attributes
# ============================================================================

subtest 'double-quoted attribute' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<a href="url">');
    my $av = first_of($tokens, 'ATTR_VALUE');
    ok( defined $av, 'ATTR_VALUE token found' );
    is( $av->{value}, '"url"', 'ATTR_VALUE includes quotes' );
};

subtest 'single-quoted attribute aliases to ATTR_VALUE' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize("<a href='url'>");
    my $av = first_of($tokens, 'ATTR_VALUE');
    ok( defined $av, 'ATTR_VALUE token found for single-quoted' );
    is( $av->{value}, "'url'", 'single-quoted value preserved' );
};

subtest 'multiple attributes' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<img src="a.png" alt="pic"/>');
    is( count_of($tokens, 'ATTR_VALUE'), 2, 'two ATTR_VALUE tokens' );
};

# ============================================================================
# Text content
# ============================================================================

subtest 'text content between tags' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<a>hello</a>');
    my $txt = first_of($tokens, 'TEXT');
    ok( defined $txt, 'TEXT token found' );
    is( $txt->{value}, 'hello', 'text value' );
};

# ============================================================================
# Entity references
# ============================================================================

subtest 'entity reference &amp;' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('&amp;');
    is( $tokens->[0]{type},  'ENTITY_REF', 'type is ENTITY_REF' );
    is( $tokens->[0]{value}, '&amp;',      'value is &amp;' );
};

subtest 'entity references &lt; and &gt;' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('&lt;&gt;');
    is( $tokens->[0]{type}, 'ENTITY_REF', 'first is ENTITY_REF' );
    is( $tokens->[1]{type}, 'ENTITY_REF', 'second is ENTITY_REF' );
};

# ============================================================================
# Character references
# ============================================================================

subtest 'decimal char ref &#65;' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('&#65;');
    is( $tokens->[0]{type},  'CHAR_REF', 'type is CHAR_REF' );
    is( $tokens->[0]{value}, '&#65;',    'value is &#65;' );
};

subtest 'hex char ref &#x41;' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('&#x41;');
    is( $tokens->[0]{type},  'CHAR_REF', 'type is CHAR_REF' );
    is( $tokens->[0]{value}, '&#x41;',   'value is &#x41;' );
};

# ============================================================================
# Comments
# ============================================================================

subtest 'comment <!-- hello -->' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<!-- hello -->');

    my $cs = first_of($tokens, 'COMMENT_START');
    ok( defined $cs,       'COMMENT_START token present' );
    is( $cs->{value}, '<!--', 'COMMENT_START value' );

    my $ct = first_of($tokens, 'COMMENT_TEXT');
    ok( defined $ct, 'COMMENT_TEXT token present' );
    like( $ct->{value}, qr/hello/, 'COMMENT_TEXT contains hello' );

    my $ce = first_of($tokens, 'COMMENT_END');
    ok( defined $ce,       'COMMENT_END token present' );
    is( $ce->{value}, '-->', 'COMMENT_END value' );
};

# ============================================================================
# CDATA sections
# ============================================================================

subtest 'CDATA <![CDATA[raw text]]>' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<![CDATA[raw text]]>');

    ok( defined first_of($tokens, 'CDATA_START'), 'CDATA_START present' );

    my $ct = first_of($tokens, 'CDATA_TEXT');
    ok( defined $ct,           'CDATA_TEXT present' );
    is( $ct->{value}, 'raw text', 'CDATA_TEXT value' );

    ok( defined first_of($tokens, 'CDATA_END'), 'CDATA_END present' );
};

subtest 'CDATA content may contain < and & literally' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<![CDATA[<div>&amp;</div>]]>');
    my $ct = first_of($tokens, 'CDATA_TEXT');
    ok( defined $ct, 'CDATA_TEXT present' );
    like( $ct->{value}, qr/<div>/, 'content contains <div>' );
};

# ============================================================================
# Processing instructions
# ============================================================================

subtest 'processing instruction <?xml version="1.0"?>' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<?xml version="1.0"?>');

    ok( defined first_of($tokens, 'PI_START'),  'PI_START present' );

    my $pt = first_of($tokens, 'PI_TARGET');
    ok( defined $pt,       'PI_TARGET present' );
    is( $pt->{value}, 'xml', 'PI_TARGET is xml' );

    ok( defined first_of($tokens, 'PI_END'),    'PI_END present' );
};

# ============================================================================
# Composite document
# ============================================================================

subtest 'full XML fragment' => sub {
    my $src = '<root id="1"><child attr=\'v\'>text &amp; &#65;</child><!-- note --></root>';
    my $tokens = CodingAdventures::XmlLexer->tokenize($src);

    ok( scalar(@$tokens) > 10, 'many tokens' );

    is( count_of($tokens, 'OPEN_TAG_START'),  2, '2 OPEN_TAG_START' );
    is( count_of($tokens, 'CLOSE_TAG_START'), 2, '2 CLOSE_TAG_START' );

    ok( count_of($tokens, 'ATTR_VALUE') >= 1, 'at least 1 ATTR_VALUE' );
    ok( count_of($tokens, 'TEXT')       >= 1, 'at least 1 TEXT' );
    ok( count_of($tokens, 'ENTITY_REF') >= 1, 'at least 1 ENTITY_REF' );
    ok( count_of($tokens, 'CHAR_REF')   >= 1, 'at least 1 CHAR_REF' );

    is( count_of($tokens, 'COMMENT_START'), 1, '1 COMMENT_START' );
    is( count_of($tokens, 'COMMENT_END'),   1, '1 COMMENT_END' );

    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'OPEN_TAG_START starts at col 1' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<a>');
    is( $tokens->[0]{col}, 1, 'OPEN_TAG_START col 1' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<a/>');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::XmlLexer->tokenize('<a/>');
    is( $tokens->[-1]{type},  'EOF', 'last is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

done_testing;
