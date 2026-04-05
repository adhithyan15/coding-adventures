use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES1Lexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('');
    is( scalar @$tokens, 1, '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1, '1 token' );
};

# ============================================================================
# Keywords
# ============================================================================

subtest 'keyword: var' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('var');
    is( $tokens->[0]{type}, 'VAR', 'type is VAR' );
};

subtest 'keyword: function' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('function');
    is( $tokens->[0]{type}, 'FUNCTION', 'type is FUNCTION' );
};

subtest 'keyword: return' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('return');
    is( $tokens->[0]{type}, 'RETURN', 'type is RETURN' );
};

subtest 'keywords: if and else' => sub {
    is( types_of('if else'), [qw(IF ELSE)], 'if and else' );
};

subtest 'keywords: for and while' => sub {
    is( types_of('for while'), [qw(FOR WHILE)], 'for and while' );
};

subtest 'keyword: break' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('break');
    is( $tokens->[0]{type}, 'BREAK', 'type is BREAK' );
};

subtest 'keyword: switch' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('switch');
    is( $tokens->[0]{type}, 'SWITCH', 'type is SWITCH' );
};

subtest 'keyword: new' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('new');
    is( $tokens->[0]{type}, 'NEW', 'type is NEW' );
};

subtest 'keyword: typeof' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('typeof');
    is( $tokens->[0]{type}, 'TYPEOF', 'type is TYPEOF' );
};

subtest 'keyword: this' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('this');
    is( $tokens->[0]{type}, 'THIS', 'type is THIS' );
};

subtest 'keywords: true, false, null' => sub {
    is( types_of('true false null'), [qw(TRUE FALSE NULL)], 'literal keywords' );
};

# ============================================================================
# Identifiers and literals
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('myVar');
    is( $tokens->[0]{type}, 'NAME', 'type is NAME' );
    is( $tokens->[0]{value}, 'myVar', 'value is myVar' );
};

subtest 'dollar-prefixed identifier' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('$el');
    is( $tokens->[0]{type}, 'NAME', 'type is NAME' );
};

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('42');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42', 'value is 42' );
};

subtest 'hex number' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('0xFF');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

subtest 'float number' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('3.14');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('"hello"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
};

subtest 'single-quoted string' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize("'world'");
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
};

# ============================================================================
# Operators (ES1 — no === or !==)
# ============================================================================

subtest 'equals equals ==' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('==');
    is( $tokens->[0]{type}, 'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
};

subtest 'not equals !=' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('!=');
    is( $tokens->[0]{type}, 'NOT_EQUALS', 'type is NOT_EQUALS' );
};

subtest 'assignment =' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('=');
    is( $tokens->[0]{type}, 'EQUALS', 'type is EQUALS' );
};

subtest 'plus +' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('+');
    is( $tokens->[0]{type}, 'PLUS', 'type is PLUS' );
};

subtest 'unsigned right shift >>>' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('>>>');
    is( $tokens->[0]{type}, 'UNSIGNED_RIGHT_SHIFT', 'type is UNSIGNED_RIGHT_SHIFT' );
};

subtest 'logical AND &&' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('&&');
    is( $tokens->[0]{type}, 'AND_AND', 'type is AND_AND' );
};

# ============================================================================
# Delimiters
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'braces' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'semicolon' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize(';');
    is( $tokens->[0]{type}, 'SEMICOLON', 'type is SEMICOLON' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'var declaration: var x = 1;' => sub {
    is(
        types_of('var x = 1;'),
        [qw(VAR NAME EQUALS NUMBER SEMICOLON)],
        'var declaration types'
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

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: var x = 1;' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('var x = 1;');
    is( $tokens->[0]{col}, 1, 'var at col 1' );
    is( $tokens->[1]{col}, 5, 'x at col 5' );
};

# ============================================================================
# EOF and errors
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('1');
    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

subtest 'unexpected character # raises die' => sub {
    ok(
        dies { CodingAdventures::EcmascriptES1Lexer->tokenize('#') },
        'unexpected # causes die'
    );
};

done_testing;
