use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CssLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::CssLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::CssLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize("   \t\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'CSS comment only produces only EOF' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('/* just a comment */');
    is( scalar @$tokens, 1,    '1 token after skipping comment' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Selector tokens
# ============================================================================

subtest 'type selector: h1' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('h1');
    is( $tokens->[0]{type},  'IDENT', 'type is IDENT' );
    is( $tokens->[0]{value}, 'h1',    'value is h1' );
};

subtest 'class selector: .class' => sub {
    is(
        types_of('.class'),
        [qw(DOT IDENT)],
        '.class produces DOT IDENT'
    );
    is( values_of('.class'), ['.', 'class'], 'values correct' );
};

subtest 'ID selector: #header' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('#header');
    is( $tokens->[0]{type},  'HASH',    'type is HASH' );
    is( $tokens->[0]{value}, '#header', 'value is #header' );
};

subtest 'attribute selector brackets: [attr]' => sub {
    is(
        types_of('[attr]'),
        [qw(LBRACKET IDENT RBRACKET)],
        '[attr] produces LBRACKET IDENT RBRACKET'
    );
};

# ============================================================================
# DIMENSION compound tokens — the key CSS tokenization challenge
# ============================================================================
#
# DIMENSION must match before NUMBER so that "10px" is one token, not two.

subtest 'DIMENSION: 10px is one token' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('10px');
    # Must be exactly ONE non-EOF token
    my @real = grep { $_->{type} ne 'EOF' } @$tokens;
    is( scalar @real, 1, '10px is ONE token' );
    is( $real[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $real[0]{value}, '10px',      'value is 10px' );
};

subtest 'DIMENSION: 1.5em' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('1.5em');
    is( $tokens->[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $tokens->[0]{value}, '1.5em',     'value is 1.5em' );
};

subtest 'DIMENSION: 100vh' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('100vh');
    is( $tokens->[0]{type}, 'DIMENSION', 'type is DIMENSION' );
};

subtest 'DIMENSION: 360deg' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('360deg');
    is( $tokens->[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $tokens->[0]{value}, '360deg',    'value is 360deg' );
};

subtest 'DIMENSION: 0.3s' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('0.3s');
    is( $tokens->[0]{type}, 'DIMENSION', 'type is DIMENSION' );
};

subtest 'DIMENSION: 300ms' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('300ms');
    is( $tokens->[0]{type}, 'DIMENSION', 'type is DIMENSION' );
};

# ============================================================================
# PERCENTAGE compound tokens
# ============================================================================

subtest 'PERCENTAGE: 50% is one token' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('50%');
    my @real = grep { $_->{type} ne 'EOF' } @$tokens;
    is( scalar @real, 1, '50% is ONE token' );
    is( $real[0]{type},  'PERCENTAGE', 'type is PERCENTAGE' );
    is( $real[0]{value}, '50%',        'value is 50%' );
};

subtest 'PERCENTAGE: 100%' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('100%');
    is( $tokens->[0]{type},  'PERCENTAGE', 'type is PERCENTAGE' );
    is( $tokens->[0]{value}, '100%',       'value is 100%' );
};

subtest 'PERCENTAGE: 0%' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('0%');
    is( $tokens->[0]{type}, 'PERCENTAGE', 'type is PERCENTAGE' );
};

# ============================================================================
# NUMBER tokens (bare numbers — only after checking DIMENSION and PERCENTAGE)
# ============================================================================

subtest 'NUMBER: 42' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'NUMBER: 3.14' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('3.14');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

# ============================================================================
# HASH tokens
# ============================================================================

subtest 'HASH: #333 (hex color)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('#333');
    is( $tokens->[0]{type},  'HASH', 'type is HASH' );
    is( $tokens->[0]{value}, '#333', 'value is #333' );
};

subtest 'HASH: #ff0000 (hex color)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('#ff0000');
    is( $tokens->[0]{type},  'HASH',    'type is HASH' );
    is( $tokens->[0]{value}, '#ff0000', 'value is #ff0000' );
};

subtest 'HASH: #header (ID selector)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('#header');
    is( $tokens->[0]{type},  'HASH',    'type is HASH' );
    is( $tokens->[0]{value}, '#header', 'value is #header' );
};

# ============================================================================
# AT_KEYWORD tokens
# ============================================================================

subtest 'AT_KEYWORD: @media' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('@media');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@media',     'value is @media' );
};

subtest 'AT_KEYWORD: @import' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('@import');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@import',    'value is @import' );
};

subtest 'AT_KEYWORD: @font-face (hyphenated)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('@font-face');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@font-face', 'value is @font-face' );
};

subtest 'AT_KEYWORD: @keyframes' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('@keyframes');
    is( $tokens->[0]{type}, 'AT_KEYWORD', 'type is AT_KEYWORD' );
};

# ============================================================================
# FUNCTION tokens (identifier + opening paren = ONE token)
# ============================================================================

subtest 'FUNCTION: rgba(' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('rgba(') };
    is( scalar @real, 1,        'rgba( is ONE token' );
    is( $real[0]{type},  'FUNCTION', 'type is FUNCTION' );
    is( $real[0]{value}, 'rgba(',    'value is rgba(' );
};

subtest 'FUNCTION: calc(' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('calc(');
    is( $tokens->[0]{type},  'FUNCTION', 'type is FUNCTION' );
    is( $tokens->[0]{value}, 'calc(',    'value is calc(' );
};

subtest 'FUNCTION: linear-gradient(' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('linear-gradient(');
    is( $tokens->[0]{type},  'FUNCTION',           'type is FUNCTION' );
    is( $tokens->[0]{value}, 'linear-gradient(',   'value is linear-gradient(' );
};

subtest 'FUNCTION: var(' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('var(');
    is( $tokens->[0]{type},  'FUNCTION', 'type is FUNCTION' );
    is( $tokens->[0]{value}, 'var(',     'value is var(' );
};

# ============================================================================
# URL_TOKEN (must come before FUNCTION)
# ============================================================================

subtest 'URL_TOKEN: url(./image.png)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('url(./image.png)');
    is( $tokens->[0]{type},  'URL_TOKEN',        'type is URL_TOKEN' );
    is( $tokens->[0]{value}, 'url(./image.png)', 'value is url(./image.png)' );
};

# ============================================================================
# STRING tokens
# ============================================================================

subtest 'STRING: double-quoted' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'STRING: single-quoted' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize("'world'");
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, "'world'", 'value preserved with quotes' );
};

subtest 'STRING: empty double-quoted' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'value is empty string' );
};

# ============================================================================
# CUSTOM_PROPERTY tokens
# ============================================================================

subtest 'CUSTOM_PROPERTY: --main-color' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('--main-color');
    is( $tokens->[0]{type},  'CUSTOM_PROPERTY', 'type is CUSTOM_PROPERTY' );
    is( $tokens->[0]{value}, '--main-color',     'value is --main-color' );
};

subtest 'CUSTOM_PROPERTY: --bg' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('--bg');
    is( $tokens->[0]{type}, 'CUSTOM_PROPERTY', 'type is CUSTOM_PROPERTY' );
};

# ============================================================================
# COLON_COLON (double colon — must come before COLON)
# ============================================================================

subtest 'COLON_COLON: :: is ONE token' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('::') };
    is( scalar @real, 1,              ':: is ONE token' );
    is( $real[0]{type},  'COLON_COLON', 'type is COLON_COLON' );
    is( $real[0]{value}, '::',          'value is ::' );
};

subtest 'COLON_COLON + IDENT: ::before' => sub {
    is(
        types_of('::before'),
        [qw(COLON_COLON IDENT)],
        '::before produces COLON_COLON IDENT'
    );
};

# ============================================================================
# Multi-character attribute operators
# ============================================================================

subtest 'TILDE_EQUALS: ~=' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('~=');
    is( $tokens->[0]{type}, 'TILDE_EQUALS', 'type is TILDE_EQUALS' );
};

subtest 'PIPE_EQUALS: |=' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('|=');
    is( $tokens->[0]{type}, 'PIPE_EQUALS', 'type is PIPE_EQUALS' );
};

subtest 'CARET_EQUALS: ^=' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('^=');
    is( $tokens->[0]{type}, 'CARET_EQUALS', 'type is CARET_EQUALS' );
};

subtest 'DOLLAR_EQUALS: $=' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('$=');
    is( $tokens->[0]{type}, 'DOLLAR_EQUALS', 'type is DOLLAR_EQUALS' );
};

subtest 'STAR_EQUALS: *=' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('*=');
    is( $tokens->[0]{type}, 'STAR_EQUALS', 'type is STAR_EQUALS' );
};

# ============================================================================
# Single-character delimiters
# ============================================================================

subtest 'LBRACE: {' => sub {
    is( types_of('{'), ['LBRACE'], 'type is LBRACE' );
};

subtest 'RBRACE: }' => sub {
    is( types_of('}'), ['RBRACE'], 'type is RBRACE' );
};

subtest 'SEMICOLON: ;' => sub {
    is( types_of(';'), ['SEMICOLON'], 'type is SEMICOLON' );
};

subtest 'COLON: :' => sub {
    is( types_of(':'), ['COLON'], 'type is COLON' );
};

subtest 'COMMA: ,' => sub {
    is( types_of(','), ['COMMA'], 'type is COMMA' );
};

subtest 'DOT: .' => sub {
    is( types_of('.'), ['DOT'], 'type is DOT' );
};

subtest 'GREATER: >' => sub {
    is( types_of('>'), ['GREATER'], 'type is GREATER' );
};

subtest 'PLUS: +' => sub {
    is( types_of('+'), ['PLUS'], 'type is PLUS' );
};

subtest 'TILDE: ~' => sub {
    is( types_of('~'), ['TILDE'], 'type is TILDE' );
};

subtest 'BANG: !' => sub {
    is( types_of('!'), ['BANG'], 'type is BANG' );
};

# ============================================================================
# Composite CSS snippets
# ============================================================================

subtest 'full rule: h1 { color: red; }' => sub {
    is(
        types_of('h1 { color: red; }'),
        [qw(IDENT LBRACE IDENT COLON IDENT SEMICOLON RBRACE)],
        'h1 { color: red; } types'
    );
};

subtest 'font-size declaration: font-size: 16px' => sub {
    is(
        types_of('font-size: 16px;'),
        [qw(IDENT COLON DIMENSION SEMICOLON)],
        'DIMENSION survives in declaration'
    );
    is( values_of('font-size: 16px;'), ['font-size', ':', '16px', ';'],
        'values correct' );
};

subtest 'calc() expression: calc(100% - 20px)' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('calc(100% - 20px)');
    is( $tokens->[0]{type},  'FUNCTION', 'first token is FUNCTION' );
    is( $tokens->[0]{value}, 'calc(',    'value is calc(' );
    is( $tokens->[1]{type},  'PERCENTAGE', 'second token is PERCENTAGE' );
    is( $tokens->[3]{type},  'DIMENSION',  'fourth token is DIMENSION' );
};

subtest '@media query structure' => sub {
    my $t = types_of('@media screen and (min-width: 768px)');
    # @media  screen  and  (  min-width  :  768px  )
    is( $t->[0], 'AT_KEYWORD', 'first is AT_KEYWORD' );
    is( $t->[1], 'IDENT',      'second is IDENT (screen)' );
    is( $t->[2], 'IDENT',      'third is IDENT (and)' );
    is( $t->[3], 'LPAREN',     'fourth is LPAREN' );
    is( $t->[4], 'IDENT',      'fifth is IDENT (min-width)' );
    is( $t->[5], 'COLON',      'sixth is COLON' );
    is( $t->[6], 'DIMENSION',  'seventh is DIMENSION (768px)' );
    is( $t->[7], 'RPAREN',     'eighth is RPAREN' );
};

subtest '!important' => sub {
    is( types_of('!important'), [qw(BANG IDENT)], '!important is BANG IDENT' );
    is( values_of('!important'), ['!', 'important'], 'values correct' );
};

subtest 'CSS nesting selector: & .child' => sub {
    is(
        types_of('& .child'),
        [qw(AMPERSAND DOT IDENT)],
        '& .child types'
    );
};

subtest 'child combinator: div > p' => sub {
    is(
        types_of('div > p'),
        [qw(IDENT GREATER IDENT)],
        'div > p types'
    );
};

# ============================================================================
# Whitespace and comment handling
# ============================================================================

subtest 'spaces between tokens consumed' => sub {
    is(
        types_of('h1 { }'),
        [qw(IDENT LBRACE RBRACE)],
        'no whitespace tokens in output'
    );
};

subtest 'CSS comment consumed' => sub {
    is(
        types_of('/* comment */ color'),
        ['IDENT'],
        'comment stripped, IDENT remains'
    );
    is( values_of('/* comment */ color'), ['color'], 'value is color' );
};

subtest 'multi-line comment consumed' => sub {
    is(
        types_of("/* line1\nline2 */ color"),
        ['IDENT'],
        'multi-line comment stripped'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: h1 { }' => sub {
    # h  1  _  {  _  }
    # 1  2  3  4  5  6
    my $tokens = CodingAdventures::CssLexer->tokenize('h1 { }');
    is( $tokens->[0]{col}, 1, 'h1 at col 1' );
    is( $tokens->[1]{col}, 4, '{ at col 4' );
    is( $tokens->[2]{col}, 6, '} at col 6' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('color: red;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::CssLexer->tokenize('h1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Priority ordering verification
# ============================================================================

subtest 'DIMENSION wins over NUMBER + IDENT for 10px' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('10px') };
    is( scalar @real, 1, '10px is ONE token' );
    is( $real[0]{type}, 'DIMENSION', 'type is DIMENSION' );
};

subtest 'PERCENTAGE wins over NUMBER for 50%' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('50%') };
    is( scalar @real, 1, '50% is ONE token' );
    is( $real[0]{type}, 'PERCENTAGE', 'type is PERCENTAGE' );
};

subtest 'FUNCTION wins over IDENT for rgba(' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('rgba(') };
    is( scalar @real, 1, 'rgba( is ONE token' );
    is( $real[0]{type}, 'FUNCTION', 'type is FUNCTION' );
};

subtest 'COLON_COLON wins over two COLONs for ::' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('::') };
    is( scalar @real, 1, ':: is ONE token' );
    is( $real[0]{type}, 'COLON_COLON', 'type is COLON_COLON' );
};

subtest 'CUSTOM_PROPERTY wins over IDENT for --var-name' => sub {
    my @real = grep { $_->{type} ne 'EOF' }
               @{ CodingAdventures::CssLexer->tokenize('--var-name') };
    is( scalar @real, 1, '--var-name is ONE token' );
    is( $real[0]{type}, 'CUSTOM_PROPERTY', 'type is CUSTOM_PROPERTY' );
};

done_testing;
