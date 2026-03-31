use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavascriptLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::JavascriptLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::JavascriptLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Keywords — variable declarations
# ============================================================================

subtest 'keyword: var' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('var');
    is( $tokens->[0]{type},  'VAR', 'type is VAR' );
    is( $tokens->[0]{value}, 'var', 'value is var' );
};

subtest 'keyword: let' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('let');
    is( $tokens->[0]{type},  'LET', 'type is LET' );
    is( $tokens->[0]{value}, 'let', 'value is let' );
};

subtest 'keyword: const' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('const');
    is( $tokens->[0]{type},  'CONST', 'type is CONST' );
    is( $tokens->[0]{value}, 'const', 'value is const' );
};

subtest 'keyword: function' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('function');
    is( $tokens->[0]{type},  'FUNCTION', 'type is FUNCTION' );
};

subtest 'keyword: return' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('return');
    is( $tokens->[0]{type},  'RETURN', 'type is RETURN' );
};

subtest 'keywords: if and else' => sub {
    is( types_of('if else'), [qw(IF ELSE)], 'if and else types' );
};

subtest 'keywords: for and while' => sub {
    is( types_of('for while'), [qw(FOR WHILE)], 'for and while types' );
};

subtest 'keyword: class' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('class');
    is( $tokens->[0]{type}, 'CLASS', 'type is CLASS' );
};

subtest 'keywords: new and this' => sub {
    is( types_of('new this'), [qw(NEW THIS)], 'new and this types' );
};

subtest 'keywords: typeof and instanceof' => sub {
    is( types_of('typeof instanceof'), [qw(TYPEOF INSTANCEOF)], 'typeof and instanceof' );
};

subtest 'keyword: true' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('true');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'true', 'value is true' );
};

subtest 'keyword: false' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('false');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'false', 'value is false' );
};

subtest 'keyword: null' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('null');
    is( $tokens->[0]{type},  'NULL', 'type is NULL' );
    is( $tokens->[0]{value}, 'null', 'value is null' );
};

subtest 'keyword: undefined' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('undefined');
    is( $tokens->[0]{type},  'UNDEFINED', 'type is UNDEFINED' );
    is( $tokens->[0]{value}, 'undefined', 'value is undefined' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('myVar');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'myVar', 'value is myVar' );
};

subtest 'identifier with dollar sign prefix' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('$el');
    is( $tokens->[0]{type},  'NAME', 'type is NAME' );
    is( $tokens->[0]{value}, '$el',  'value is $el' );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('_priv');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, '_priv', 'value is _priv' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'string with escape sequence' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('"a\\nb"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'escape sequence preserved in value' );
};

# ============================================================================
# Operator tokens
# ============================================================================

subtest 'strict equals ===' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('===');
    is( $tokens->[0]{type},  'STRICT_EQUALS', 'type is STRICT_EQUALS' );
    is( $tokens->[0]{value}, '===',           'value is ===' );
};

subtest 'strict not equals !==' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('!==');
    is( $tokens->[0]{type},  'STRICT_NOT_EQUALS', 'type is STRICT_NOT_EQUALS' );
    is( $tokens->[0]{value}, '!==',               'value is !==' );
};

subtest 'loose equals ==' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('==');
    is( $tokens->[0]{type},  'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
    is( $tokens->[0]{value}, '==',            'value is ==' );
};

subtest 'not equals !=' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('!=');
    is( $tokens->[0]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[0]{value}, '!=',         'value is !=' );
};

subtest 'arrow =>' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('=>');
    is( $tokens->[0]{type},  'ARROW', 'type is ARROW' );
    is( $tokens->[0]{value}, '=>',    'value is =>' );
};

subtest 'less than or equal <=' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('<=');
    is( $tokens->[0]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[0]{value}, '<=',          'value is <=' );
};

subtest 'greater than or equal >=' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('>=');
    is( $tokens->[0]{type},  'GREATER_EQUALS', 'type is GREATER_EQUALS' );
    is( $tokens->[0]{value}, '>=',             'value is >=' );
};

subtest 'assignment =' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

subtest 'plus +' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('+');
    is( $tokens->[0]{type},  'PLUS', 'type is PLUS' );
    is( $tokens->[0]{value}, '+',    'value is +' );
};

subtest 'minus -' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('-');
    is( $tokens->[0]{type},  'MINUS', 'type is MINUS' );
    is( $tokens->[0]{value}, '-',     'value is -' );
};

subtest 'star *' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('*');
    is( $tokens->[0]{type},  'STAR', 'type is STAR' );
    is( $tokens->[0]{value}, '*',    'value is *' );
};

subtest 'slash /' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('/');
    is( $tokens->[0]{type},  'SLASH', 'type is SLASH' );
    is( $tokens->[0]{value}, '/',     'value is /' );
};

subtest 'less than <' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('<');
    is( $tokens->[0]{type},  'LESS_THAN', 'type is LESS_THAN' );
    is( $tokens->[0]{value}, '<',         'value is <' );
};

subtest 'greater than >' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('>');
    is( $tokens->[0]{type},  'GREATER_THAN', 'type is GREATER_THAN' );
    is( $tokens->[0]{value}, '>',            'value is >' );
};

subtest 'bang !' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('!');
    is( $tokens->[0]{type},  'BANG', 'type is BANG' );
    is( $tokens->[0]{value}, '!',    'value is !' );
};

# ============================================================================
# Punctuation tokens
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
    is( values_of('()'), ['(', ')'], 'paren values' );
};

subtest 'braces' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'brackets' => sub {
    is( types_of('[]'), [qw(LBRACKET RBRACKET)], 'bracket types' );
};

subtest 'semicolon' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize(';');
    is( $tokens->[0]{type},  'SEMICOLON', 'type is SEMICOLON' );
    is( $tokens->[0]{value}, ';',         'value is ;' );
};

subtest 'comma' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize(',');
    is( $tokens->[0]{type},  'COMMA', 'type is COMMA' );
    is( $tokens->[0]{value}, ',',     'value is ,' );
};

subtest 'dot' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('.');
    is( $tokens->[0]{type},  'DOT', 'type is DOT' );
    is( $tokens->[0]{value}, '.',   'value is .' );
};

subtest 'colon' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize(':');
    is( $tokens->[0]{type},  'COLON', 'type is COLON' );
    is( $tokens->[0]{value}, ':',     'value is :' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'variable declaration: var x = 1;' => sub {
    is(
        types_of('var x = 1;'),
        [qw(VAR NAME EQUALS NUMBER SEMICOLON)],
        'var declaration types'
    );
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('var x = 1;');
    is( $tokens->[1]{value}, 'x', 'identifier value is x' );
};

subtest 'const declaration: const PI = 3;' => sub {
    is(
        types_of('const PI = 3;'),
        [qw(CONST NAME EQUALS NUMBER SEMICOLON)],
        'const declaration types'
    );
};

subtest 'arrow function: (x) => x + 1' => sub {
    is(
        types_of('(x) => x + 1'),
        [qw(LPAREN NAME RPAREN ARROW NAME PLUS NUMBER)],
        'arrow function types'
    );
};

subtest 'strict equality: a === b' => sub {
    is(
        types_of('a === b'),
        [qw(NAME STRICT_EQUALS NAME)],
        'strict equality types'
    );
};

subtest 'function declaration' => sub {
    is(
        types_of('function add(a, b) { return a + b; }'),
        [qw(FUNCTION NAME LPAREN NAME COMMA NAME RPAREN
            LBRACE RETURN NAME PLUS NAME SEMICOLON RBRACE)],
        'function declaration types'
    );
};

subtest 'typeof expression' => sub {
    is( types_of('typeof x'), [qw(TYPEOF NAME)], 'typeof types' );
};

subtest 'instanceof expression' => sub {
    is( types_of('x instanceof Foo'), [qw(NAME INSTANCEOF NAME)], 'instanceof types' );
};

subtest 'new expression' => sub {
    is( types_of('new Foo()'), [qw(NEW NAME LPAREN RPAREN)], 'new expression types' );
};

subtest 'method call: obj.method(arg)' => sub {
    is(
        types_of('obj.method(arg)'),
        [qw(NAME DOT NAME LPAREN NAME RPAREN)],
        'method call types'
    );
};

subtest 'if/else statement' => sub {
    my $src = 'if (x) { return true; } else { return false; }';
    my $tokens = CodingAdventures::JavascriptLexer->tokenize($src);
    my ($if_tok)    = grep { $_->{type} eq 'IF' }    @$tokens;
    my ($else_tok)  = grep { $_->{type} eq 'ELSE' }  @$tokens;
    my ($true_tok)  = grep { $_->{type} eq 'TRUE' }  @$tokens;
    my ($false_tok) = grep { $_->{type} eq 'FALSE' } @$tokens;
    ok( $if_tok,    'has IF token' );
    ok( $else_tok,  'has ELSE token' );
    ok( $true_tok,  'has TRUE token' );
    ok( $false_tok, 'has FALSE token' );
};

subtest 'class declaration' => sub {
    is(
        types_of('class Animal { }'),
        [qw(CLASS NAME LBRACE RBRACE)],
        'class declaration types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('var x = 1'),
        [qw(VAR NAME EQUALS NUMBER)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed silently' => sub {
    is(
        types_of("var\n\tx\n=\n1"),
        [qw(VAR NAME EQUALS NUMBER)],
        'only value tokens in output'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: var x = 1;' => sub {
    # v a r _ x _ = _ 1 ;
    # 1 . . 4 5 6 7 8 9 10
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('var x = 1;');
    is( $tokens->[0]{col},  1, 'var at col 1' );
    is( $tokens->[1]{col},  5, 'x at col 5' );
    is( $tokens->[2]{col},  7, '= at col 7' );
    is( $tokens->[3]{col},  9, '1 at col 9' );
    is( $tokens->[4]{col}, 10, '; at col 10' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('var x = 1;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::JavascriptLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character @ raises die' => sub {
    ok(
        dies { CodingAdventures::JavascriptLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

subtest 'backtick raises die (template literals not in grammar)' => sub {
    ok(
        dies { CodingAdventures::JavascriptLexer->tokenize('`hello`') },
        'backtick causes die'
    );
};

done_testing;
