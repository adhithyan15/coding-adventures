use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LatticeLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers: collect token types and values (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::LatticeLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::LatticeLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize("   \t\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Lattice VARIABLE tokens
# ============================================================================

subtest 'VARIABLE: $color' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$color');
    is( $tokens->[0]{type},  'VARIABLE', 'type is VARIABLE' );
    is( $tokens->[0]{value}, '$color',   'value is $color'  );
};

subtest 'VARIABLE: $font-size (hyphenated)' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$font-size');
    is( $tokens->[0]{type},  'VARIABLE',    'type is VARIABLE'    );
    is( $tokens->[0]{value}, '$font-size',  'value is $font-size' );
};

subtest 'VARIABLE: $my_var (underscore)' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$my_var');
    is( $tokens->[0]{type},  'VARIABLE', 'type is VARIABLE' );
    is( $tokens->[0]{value}, '$my_var',  'value is $my_var' );
};

subtest '$= is DOLLAR_EQUALS not VARIABLE' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$=');
    is( $tokens->[0]{type}, 'DOLLAR_EQUALS', 'type is DOLLAR_EQUALS' );
};

# ============================================================================
# Lattice PLACEHOLDER tokens
# ============================================================================

subtest 'PLACEHOLDER: %button-base' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('%button-base');
    is( $tokens->[0]{type},  'PLACEHOLDER',  'type is PLACEHOLDER'  );
    is( $tokens->[0]{value}, '%button-base', 'value is %button-base' );
};

subtest 'PLACEHOLDER: %flex-center' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('%flex-center');
    is( $tokens->[0]{type},  'PLACEHOLDER',  'type is PLACEHOLDER' );
    is( $tokens->[0]{value}, '%flex-center', 'value is %flex-center' );
};

# ============================================================================
# Numeric tokens (DIMENSION > PERCENTAGE > NUMBER priority)
# ============================================================================

subtest 'DIMENSION: 10px' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('10px');
    is( $tokens->[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $tokens->[0]{value}, '10px',      'value is 10px'     );
};

subtest 'DIMENSION: 1.5em' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('1.5em');
    is( $tokens->[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $tokens->[0]{value}, '1.5em',     'value is 1.5em'    );
};

subtest 'DIMENSION: -2rem' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('-2rem');
    is( $tokens->[0]{type},  'DIMENSION', 'type is DIMENSION' );
    is( $tokens->[0]{value}, '-2rem',     'value is -2rem'    );
};

subtest 'PERCENTAGE: 50%' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('50%');
    is( $tokens->[0]{type},  'PERCENTAGE', 'type is PERCENTAGE' );
    is( $tokens->[0]{value}, '50%',        'value is 50%'       );
};

subtest 'PERCENTAGE: 100%' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('100%');
    is( $tokens->[0]{type},  'PERCENTAGE', 'type is PERCENTAGE' );
    is( $tokens->[0]{value}, '100%',       'value is 100%'      );
};

subtest 'NUMBER: 0' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0'     );
};

subtest 'NUMBER: 3.14' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('3.14');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '3.14',   'value is 3.14'  );
};

subtest 'NUMBER: -1' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('-1');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '-1',     'value is -1'    );
};

# ============================================================================
# HASH tokens
# ============================================================================

subtest 'HASH: #ff0000' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('#ff0000');
    is( $tokens->[0]{type},  'HASH',    'type is HASH'    );
    is( $tokens->[0]{value}, '#ff0000', 'value is #ff0000' );
};

subtest 'HASH: #abc' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('#abc');
    is( $tokens->[0]{type},  'HASH', 'type is HASH' );
    is( $tokens->[0]{value}, '#abc', 'value is #abc' );
};

subtest 'HASH: #my-button' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('#my-button');
    is( $tokens->[0]{type},  'HASH',       'type is HASH'       );
    is( $tokens->[0]{value}, '#my-button', 'value is #my-button' );
};

# ============================================================================
# AT_KEYWORD tokens
# ============================================================================

subtest 'AT_KEYWORD: @media' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('@media');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@media',     'value is @media'    );
};

subtest 'AT_KEYWORD: @mixin' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('@mixin');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@mixin',     'value is @mixin'    );
};

subtest 'AT_KEYWORD: @if' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('@if');
    is( $tokens->[0]{type},  'AT_KEYWORD', 'type is AT_KEYWORD' );
    is( $tokens->[0]{value}, '@if',        'value is @if'       );
};

# ============================================================================
# FUNCTION tokens
# ============================================================================

subtest 'FUNCTION: rgb(' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('rgb(');
    is( $tokens->[0]{type},  'FUNCTION', 'type is FUNCTION' );
    is( $tokens->[0]{value}, 'rgb(',     'value is rgb('    );
};

subtest 'FUNCTION: calc(' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('calc(');
    is( $tokens->[0]{type},  'FUNCTION', 'type is FUNCTION' );
    is( $tokens->[0]{value}, 'calc(',    'value is calc('   );
};

# ============================================================================
# IDENT tokens
# ============================================================================

subtest 'IDENT: red' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('red');
    is( $tokens->[0]{type},  'IDENT', 'type is IDENT' );
    is( $tokens->[0]{value}, 'red',   'value is red'  );
};

subtest 'IDENT: border-radius' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('border-radius');
    is( $tokens->[0]{type},  'IDENT',          'type is IDENT'         );
    is( $tokens->[0]{value}, 'border-radius',  'value is border-radius' );
};

# ============================================================================
# CUSTOM_PROPERTY tokens
# ============================================================================

subtest 'CUSTOM_PROPERTY: --primary-color' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('--primary-color');
    is( $tokens->[0]{type},  'CUSTOM_PROPERTY', 'type is CUSTOM_PROPERTY' );
    is( $tokens->[0]{value}, '--primary-color', 'value is --primary-color' );
};

# ============================================================================
# Multi-character operators
# ============================================================================

subtest 'COLON_COLON: ::' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('::');
    is( $tokens->[0]{type},  'COLON_COLON', 'type is COLON_COLON' );
    is( $tokens->[0]{value}, '::',          'value is ::'         );
};

subtest 'TILDE_EQUALS: ~=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('~=');
    is( $tokens->[0]{type},  'TILDE_EQUALS', 'type is TILDE_EQUALS' );
    is( $tokens->[0]{value}, '~=',           'value is ~='          );
};

subtest 'PIPE_EQUALS: |=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('|=');
    is( $tokens->[0]{type},  'PIPE_EQUALS', 'type is PIPE_EQUALS' );
    is( $tokens->[0]{value}, '|=',          'value is |='         );
};

subtest 'CARET_EQUALS: ^=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('^=');
    is( $tokens->[0]{type},  'CARET_EQUALS', 'type is CARET_EQUALS' );
    is( $tokens->[0]{value}, '^=',           'value is ^='          );
};

subtest 'DOLLAR_EQUALS: $=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$=');
    is( $tokens->[0]{type},  'DOLLAR_EQUALS', 'type is DOLLAR_EQUALS' );
    is( $tokens->[0]{value}, '$=',            'value is $='           );
};

subtest 'STAR_EQUALS: *=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('*=');
    is( $tokens->[0]{type},  'STAR_EQUALS', 'type is STAR_EQUALS' );
    is( $tokens->[0]{value}, '*=',          'value is *='         );
};

subtest 'EQUALS_EQUALS: ==' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('==');
    is( $tokens->[0]{type},  'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
    is( $tokens->[0]{value}, '==',            'value is =='           );
};

subtest 'NOT_EQUALS: !=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('!=');
    is( $tokens->[0]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[0]{value}, '!=',         'value is !='        );
};

subtest 'GREATER_EQUALS: >=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('>=');
    is( $tokens->[0]{type},  'GREATER_EQUALS', 'type is GREATER_EQUALS' );
    is( $tokens->[0]{value}, '>=',             'value is >='            );
};

subtest 'LESS_EQUALS: <=' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('<=');
    is( $tokens->[0]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[0]{value}, '<=',          'value is <='         );
};

# ============================================================================
# Lattice bang tokens
# ============================================================================

subtest 'BANG_DEFAULT: !default' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('!default');
    is( $tokens->[0]{type},  'BANG_DEFAULT', 'type is BANG_DEFAULT' );
    is( $tokens->[0]{value}, '!default',     'value is !default'    );
};

subtest 'BANG_GLOBAL: !global' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('!global');
    is( $tokens->[0]{type},  'BANG_GLOBAL', 'type is BANG_GLOBAL' );
    is( $tokens->[0]{value}, '!global',     'value is !global'    );
};

subtest 'BANG: bare !' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('!');
    is( $tokens->[0]{type},  'BANG', 'type is BANG' );
    is( $tokens->[0]{value}, '!',    'value is !'   );
};

# ============================================================================
# Delimiter tokens
# ============================================================================

subtest 'braces {} ' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'parentheses ()' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'brackets []' => sub {
    is( types_of('[]'), [qw(LBRACKET RBRACKET)], 'bracket types' );
};

subtest 'semicolon ;' => sub {
    is( types_of(';'), ['SEMICOLON'], 'semicolon type' );
};

subtest 'colon :' => sub {
    is( types_of(':'), ['COLON'], 'colon type' );
};

subtest 'comma ,' => sub {
    is( types_of(','), ['COMMA'], 'comma type' );
};

subtest 'dot .' => sub {
    is( types_of('.'), ['DOT'], 'dot type' );
};

subtest 'ampersand &' => sub {
    is( types_of('&'), ['AMPERSAND'], 'ampersand type' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING'   );
    is( $tokens->[0]{value}, '"hello"', 'value with quotes' );
};

subtest 'single-quoted string' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize("'world'");
    is( $tokens->[0]{type},  'STRING',  'type is STRING'   );
    is( $tokens->[0]{value}, "'world'", 'value with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING'   );
    is( $tokens->[0]{value}, '""',     'empty string'     );
};

# ============================================================================
# Comment handling
# ============================================================================

subtest '// line comment consumed silently' => sub {
    # "color // comment" should produce just IDENT
    is( types_of('color // this is a comment'), ['IDENT'], 'comment consumed' );
};

subtest '/* block comment */ consumed silently' => sub {
    is( types_of('color /* block */ red'), [qw(IDENT IDENT)], 'block comment consumed' );
};

subtest 'comment text does not appear in token values' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('x /* secret */ y');
    for my $tok (@$tokens) {
        ok( $tok->{value} !~ /secret/, "$tok->{type} value does not contain comment text" );
    }
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'variable declaration: $color: #ff0000;' => sub {
    is(
        types_of('$color: #ff0000;'),
        [qw(VARIABLE COLON HASH SEMICOLON)],
        'variable declaration types'
    );
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$color: #ff0000;');
    is( $tokens->[0]{value}, '$color',  '$color variable value'  );
    is( $tokens->[2]{value}, '#ff0000', '#ff0000 hash value'      );
};

subtest 'CSS property: color: red;' => sub {
    is(
        types_of('color: red;'),
        [qw(IDENT COLON IDENT SEMICOLON)],
        'css property types'
    );
};

subtest 'dimension value: margin: 10px 20px;' => sub {
    is(
        types_of('margin: 10px 20px;'),
        [qw(IDENT COLON DIMENSION DIMENSION SEMICOLON)],
        'dimension value types'
    );
};

subtest '@if condition: @if $x == 1' => sub {
    is(
        types_of('@if $x == 1'),
        [qw(AT_KEYWORD VARIABLE EQUALS_EQUALS NUMBER)],
        '@if condition types'
    );
};

subtest '!default flag: $size: 10px !default;' => sub {
    is(
        types_of('$size: 10px !default;'),
        [qw(VARIABLE COLON DIMENSION BANG_DEFAULT SEMICOLON)],
        '!default flag types'
    );
};

subtest '@extend placeholder: @extend %button-base;' => sub {
    is(
        types_of('@extend %button-base;'),
        [qw(AT_KEYWORD PLACEHOLDER SEMICOLON)],
        '@extend placeholder types'
    );
};

subtest 'CSS selector block: .foo { color: red; }' => sub {
    is(
        types_of('.foo { color: red; }'),
        [qw(DOT IDENT LBRACE IDENT COLON IDENT SEMICOLON RBRACE)],
        'selector block types'
    );
};

subtest 'attribute selector: a[href$=\'pdf\']' => sub {
    is(
        types_of("a[href\$='pdf']"),
        [qw(IDENT LBRACKET IDENT DOLLAR_EQUALS STRING RBRACKET)],
        'attribute selector types'
    );
};

subtest 'pseudo-element: ::before' => sub {
    is(
        types_of('::before'),
        [qw(COLON_COLON IDENT)],
        'pseudo-element types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens consumed silently' => sub {
    is( types_of('color : red'), [qw(IDENT COLON IDENT)], 'spaces consumed' );
};

subtest 'newlines between tokens consumed silently' => sub {
    is( types_of("color\n:\nred"), [qw(IDENT COLON IDENT)], 'newlines consumed' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: $x: 1px;' => sub {
    # $ x  :  _  1  p  x  ;
    # 1 2  3  4  5  6  7  8
    my $tokens = CodingAdventures::LatticeLexer->tokenize('$x: 1px;');
    is( $tokens->[0]{col}, 1, '$x at col 1'   );
    is( $tokens->[1]{col}, 3, ': at col 3'    );
    is( $tokens->[2]{col}, 5, '1px at col 5'  );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('color: red;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::LatticeLexer->tokenize('color');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::LatticeLexer->tokenize('`') },
        'backtick causes die'
    );
};

done_testing;
