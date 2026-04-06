use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::AlgolLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

# Return a list of token types (excluding EOF) from a source string.
sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::AlgolLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Return a list of token values (excluding EOF) from a source string.
sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::AlgolLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::AlgolLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::AlgolLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Keywords — all ALGOL 60 reserved words
# ============================================================================
#
# ALGOL 60 has an unusually large set of reserved words for its era.
# They cover block structure, control flow, declarations, types, and
# all the boolean and arithmetic operators (which are words, not symbols).

subtest 'keyword: begin' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('begin');
    is( $toks->[0]{type}, 'BEGIN', 'begin → BEGIN' );
};

subtest 'keyword: end' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('end');
    is( $toks->[0]{type}, 'END', 'end → END' );
};

subtest 'keyword: if' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('if');
    is( $toks->[0]{type}, 'IF', 'if → IF' );
};

subtest 'keyword: then' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('then');
    is( $toks->[0]{type}, 'THEN', 'then → THEN' );
};

subtest 'keyword: else' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('else');
    is( $toks->[0]{type}, 'ELSE', 'else → ELSE' );
};

subtest 'keyword: for' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('for');
    is( $toks->[0]{type}, 'FOR', 'for → FOR' );
};

subtest 'keyword: do' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('do');
    is( $toks->[0]{type}, 'DO', 'do → DO' );
};

subtest 'keyword: step' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('step');
    is( $toks->[0]{type}, 'STEP', 'step → STEP' );
};

subtest 'keyword: until' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('until');
    is( $toks->[0]{type}, 'UNTIL', 'until → UNTIL' );
};

subtest 'keyword: while' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('while');
    is( $toks->[0]{type}, 'WHILE', 'while → WHILE' );
};

subtest 'keyword: goto' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('goto');
    is( $toks->[0]{type}, 'GOTO', 'goto → GOTO' );
};

subtest 'keyword: switch' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('switch');
    is( $toks->[0]{type}, 'SWITCH', 'switch → SWITCH' );
};

subtest 'keyword: procedure' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('procedure');
    is( $toks->[0]{type}, 'PROCEDURE', 'procedure → PROCEDURE' );
};

subtest 'keyword: integer' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('integer');
    is( $toks->[0]{type}, 'INTEGER', 'integer → INTEGER' );
};

subtest 'keyword: real' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('real');
    is( $toks->[0]{type}, 'REAL', 'real → REAL' );
};

subtest 'keyword: boolean' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('boolean');
    is( $toks->[0]{type}, 'BOOLEAN', 'boolean → BOOLEAN' );
};

subtest 'keyword: string' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('string');
    is( $toks->[0]{type}, 'STRING', 'string → STRING' );
};

subtest 'keyword: array' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('array');
    is( $toks->[0]{type}, 'ARRAY', 'array → ARRAY' );
};

subtest 'keyword: value' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('value');
    is( $toks->[0]{type}, 'VALUE', 'value → VALUE' );
};

subtest 'keyword: true' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('true');
    is( $toks->[0]{type}, 'TRUE', 'true → TRUE' );
};

subtest 'keyword: false' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('false');
    is( $toks->[0]{type}, 'FALSE', 'false → FALSE' );
};

subtest 'keyword: not' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('not');
    is( $toks->[0]{type}, 'NOT', 'not → NOT' );
};

subtest 'keyword: and' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('and');
    is( $toks->[0]{type}, 'AND', 'and → AND' );
};

subtest 'keyword: or' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('or');
    is( $toks->[0]{type}, 'OR', 'or → OR' );
};

subtest 'keyword: impl' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('impl');
    is( $toks->[0]{type}, 'IMPL', 'impl → IMPL' );
};

subtest 'keyword: eqv' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('eqv');
    is( $toks->[0]{type}, 'EQV', 'eqv → EQV' );
};

subtest 'keyword: div' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('div');
    is( $toks->[0]{type}, 'DIV', 'div → DIV' );
};

subtest 'keyword: mod' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('mod');
    is( $toks->[0]{type}, 'MOD', 'mod → MOD' );
};

# ============================================================================
# Keyword case-insensitivity
# ============================================================================
#
# ALGOL 60 keywords are case-insensitive in this implementation.
# The grammar comment states: "Case-insensitive: BEGIN, Begin, begin all
# produce the same token kind."

subtest 'keywords are case-insensitive: BEGIN' => sub {
    my $lower = CodingAdventures::AlgolLexer->tokenize('begin');
    my $upper = CodingAdventures::AlgolLexer->tokenize('BEGIN');
    my $mixed = CodingAdventures::AlgolLexer->tokenize('Begin');
    is( $lower->[0]{type}, 'BEGIN', 'begin → BEGIN' );
    is( $upper->[0]{type}, 'BEGIN', 'BEGIN → BEGIN' );
    is( $mixed->[0]{type}, 'BEGIN', 'Begin → BEGIN' );
};

subtest 'keywords are case-insensitive: INTEGER' => sub {
    my $lower = CodingAdventures::AlgolLexer->tokenize('integer');
    my $upper = CodingAdventures::AlgolLexer->tokenize('INTEGER');
    is( $lower->[0]{type}, 'INTEGER', 'integer → INTEGER' );
    is( $upper->[0]{type}, 'INTEGER', 'INTEGER → INTEGER' );
};

# ============================================================================
# Keyword boundary: partial match must produce NAME
# ============================================================================
#
# "beginning" starts with "begin" but it is NOT the keyword BEGIN.
# A keyword match requires the entire identifier to be a reserved word.
# This rule prevents keywords from splitting inside longer names.

subtest 'beginning is NAME not BEGIN' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('beginning');
    is( $toks->[0]{type},  'NAME',     '"beginning" → NAME' );
    is( $toks->[0]{value}, 'beginning', 'value preserved' );
};

subtest 'ending is NAME not END' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('ending');
    is( $toks->[0]{type}, 'NAME', '"ending" → NAME' );
};

subtest 'integer2 is NAME not INTEGER' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('integer2');
    is( $toks->[0]{type}, 'NAME', '"integer2" → NAME (trailing digit)' );
};

subtest 'truefalse is NAME' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('truefalse');
    is( $toks->[0]{type}, 'NAME', '"truefalse" → NAME' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'single-letter identifier' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('x');
    is( $toks->[0]{type},  'NAME', 'type is NAME' );
    is( $toks->[0]{value}, 'x',     'value is x' );
};

subtest 'multi-letter identifier' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('sum');
    is( $toks->[0]{type},  'NAME', 'type is NAME' );
    is( $toks->[0]{value}, 'sum',   'value is sum' );
};

subtest 'alphanumeric identifier' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('A1');
    is( $toks->[0]{type},  'NAME', 'type is NAME' );
    is( $toks->[0]{value}, 'A1',    'value is A1' );
};

subtest 'mixed-case identifier' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('myVar');
    is( $toks->[0]{type},  'NAME',  'type is NAME' );
    is( $toks->[0]{value}, 'myVar',  'value is myVar' );
};

# ============================================================================
# Integer literals
# ============================================================================

subtest 'zero' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('0');
    is( $toks->[0]{type},  'INTEGER_LIT', 'type is INTEGER_LIT' );
    is( $toks->[0]{value}, '0',           'value is 0' );
};

subtest 'multi-digit integer' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('42');
    is( $toks->[0]{type},  'INTEGER_LIT', 'type is INTEGER_LIT' );
    is( $toks->[0]{value}, '42',          'value is 42' );
};

subtest 'large integer' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('1000');
    is( $toks->[0]{type},  'INTEGER_LIT', 'type is INTEGER_LIT' );
    is( $toks->[0]{value}, '1000',        'value is 1000' );
};

# ============================================================================
# Real literals
# ============================================================================
#
# ALGOL 60 real literals cover several forms:
#   3.14       decimal fraction
#   1.5E3      integer + exponent (= 1500.0)
#   1.5E-3     integer + negative exponent (= 0.0015)
#   100E2      integer + exponent, no decimal point
#
# REAL_LIT must be matched before INTEGER_LIT so "3.14" becomes REAL_LIT,
# not INTEGER_LIT("3") + unknown(".") + INTEGER_LIT("14").

subtest 'real with decimal point: 3.14' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('3.14');
    is( $toks->[0]{type},  'REAL_LIT', 'type is REAL_LIT' );
    is( $toks->[0]{value}, '3.14',     'value is 3.14' );
};

subtest 'real with exponent: 1.5E3' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('1.5E3');
    is( $toks->[0]{type},  'REAL_LIT', 'type is REAL_LIT' );
    is( $toks->[0]{value}, '1.5E3',    'value is 1.5E3' );
};

subtest 'real with negative exponent: 1.5E-3' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('1.5E-3');
    is( $toks->[0]{type},  'REAL_LIT', 'type is REAL_LIT' );
    is( $toks->[0]{value}, '1.5E-3',   'value is 1.5E-3' );
};

subtest 'real integer + exponent no decimal: 100E2' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('100E2');
    is( $toks->[0]{type},  'REAL_LIT', 'type is REAL_LIT' );
    is( $toks->[0]{value}, '100E2',    'value is 100E2' );
};

subtest 'real with lowercase e: 2.5e10' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('2.5e10');
    is( $toks->[0]{type},  'REAL_LIT', 'type is REAL_LIT' );
    is( $toks->[0]{value}, '2.5e10',   'value is 2.5e10' );
};

# ============================================================================
# String literals
# ============================================================================
#
# ALGOL 60 strings are single-quoted: 'hello'.
# There are no escape sequences — a single quote cannot appear inside a string.

subtest "string literal 'hello'" => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize("'hello'");
    is( $toks->[0]{type},  'STRING_LIT', 'type is STRING_LIT' );
    is( $toks->[0]{value}, "'hello'",    "value is 'hello'" );
};

subtest "empty string ''" => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize("''");
    is( $toks->[0]{type},  'STRING_LIT', 'type is STRING_LIT' );
    is( $toks->[0]{value}, "''",         "value is ''" );
};

subtest "string with spaces: 'hello world'" => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize("'hello world'");
    is( $toks->[0]{type},  'STRING_LIT',    'type is STRING_LIT' );
    is( $toks->[0]{value}, "'hello world'", "value is 'hello world'" );
};

# ============================================================================
# Operators — multi-character (must beat single-char versions)
# ============================================================================
#
# These are the critical disambiguation tests.  In each case the multi-char
# operator must be returned as a single token, NOT as two separate tokens.

subtest 'ASSIGN := is a single token' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(':=');
    is( scalar(grep { $_->{type} ne 'EOF' } @$toks), 1, 'exactly one non-EOF token' );
    is( $toks->[0]{type},  'ASSIGN', 'type is ASSIGN' );
    is( $toks->[0]{value}, ':=',     'value is :=' );
};

subtest ':= is not COLON + EQ' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(':=');
    isnt( $toks->[0]{type}, 'COLON', ':= is not tokenized as COLON' );
    is(   $toks->[0]{type}, 'ASSIGN', ':= produces ASSIGN' );
};

subtest 'POWER ** is a single token' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('**');
    is( scalar(grep { $_->{type} ne 'EOF' } @$toks), 1, 'exactly one non-EOF token' );
    is( $toks->[0]{type},  'POWER', 'type is POWER' );
    is( $toks->[0]{value}, '**',    'value is **' );
};

subtest '** is not STAR + STAR' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('**');
    isnt( $toks->[0]{type}, 'STAR', '** first token is not STAR' );
    is(   $toks->[0]{type}, 'POWER', '** produces POWER' );
};

subtest 'LEQ <= is a single token' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('<=');
    is( scalar(grep { $_->{type} ne 'EOF' } @$toks), 1, 'exactly one non-EOF token' );
    is( $toks->[0]{type},  'LEQ', 'type is LEQ' );
    is( $toks->[0]{value}, '<=',  'value is <=' );
};

subtest 'GEQ >= is a single token' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('>=');
    is( scalar(grep { $_->{type} ne 'EOF' } @$toks), 1, 'exactly one non-EOF token' );
    is( $toks->[0]{type},  'GEQ', 'type is GEQ' );
    is( $toks->[0]{value}, '>=',  'value is >=' );
};

subtest 'NEQ != is a single token' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('!=');
    is( scalar(grep { $_->{type} ne 'EOF' } @$toks), 1, 'exactly one non-EOF token' );
    is( $toks->[0]{type},  'NEQ', 'type is NEQ' );
    is( $toks->[0]{value}, '!=',  'value is !=' );
};

# ============================================================================
# Operators — single-character
# ============================================================================

subtest 'PLUS +' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('+');
    is( $toks->[0]{type}, 'PLUS', 'type is PLUS' );
};

subtest 'MINUS -' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('-');
    is( $toks->[0]{type}, 'MINUS', 'type is MINUS' );
};

subtest 'STAR *' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('*');
    is( $toks->[0]{type}, 'STAR', 'type is STAR' );
};

subtest 'SLASH /' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('/');
    is( $toks->[0]{type}, 'SLASH', 'type is SLASH' );
};

subtest 'EQ =' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('=');
    is( $toks->[0]{type}, 'EQ', 'type is EQ' );
};

subtest 'LT <' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('<');
    is( $toks->[0]{type}, 'LT', 'type is LT' );
};

subtest 'GT >' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('>');
    is( $toks->[0]{type}, 'GT', 'type is GT' );
};

subtest 'CARET ^' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('^');
    is( $toks->[0]{type}, 'CARET', 'type is CARET' );
};

# ============================================================================
# Delimiters
# ============================================================================

subtest 'LPAREN (' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('(');
    is( $toks->[0]{type}, 'LPAREN', 'type is LPAREN' );
};

subtest 'RPAREN )' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(')');
    is( $toks->[0]{type}, 'RPAREN', 'type is RPAREN' );
};

subtest 'LBRACKET [' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('[');
    is( $toks->[0]{type}, 'LBRACKET', 'type is LBRACKET' );
};

subtest 'RBRACKET ]' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(']');
    is( $toks->[0]{type}, 'RBRACKET', 'type is RBRACKET' );
};

subtest 'SEMICOLON ;' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(';');
    is( $toks->[0]{type}, 'SEMICOLON', 'type is SEMICOLON' );
};

subtest 'COMMA ,' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(',');
    is( $toks->[0]{type}, 'COMMA', 'type is COMMA' );
};

subtest 'COLON :' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize(':');
    is( $toks->[0]{type}, 'COLON', 'type is COLON' );
};

# ============================================================================
# Comment skipping
# ============================================================================
#
# ALGOL 60 comments begin with the word `comment` and end at the next `;`.
# The entire comment (including `comment` and the closing `;`) is consumed
# silently — no tokens are emitted for the comment itself.
#
# From the grammar: COMMENT = /comment[^;]*;/

subtest 'comment is consumed silently' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('comment this is ignored; x := 1');
    # Should see: NAME(x), ASSIGN(:=), INTEGER_LIT(1), EOF
    my $types = [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$toks ];
    is( $types, [qw(NAME ASSIGN INTEGER_LIT)], 'comment is skipped, x := 1 tokenized' );
    is( $toks->[0]{value}, 'x', 'first real token is x' );
};

subtest 'comment at start of program' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('comment initialize;');
    # The comment consumes "comment initialize;" so only EOF remains.
    is( scalar @$toks, 1,     'only EOF token' );
    is( $toks->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'multiple comments' => sub {
    my $toks = CodingAdventures::AlgolLexer->tokenize('comment one; x := 1; comment two; y := 2');
    my $types = [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$toks ];
    # Should see: NAME(x), ASSIGN, INTEGER_LIT(1), SEMICOLON, NAME(y), ASSIGN, INTEGER_LIT(2)
    is( $types, [qw(NAME ASSIGN INTEGER_LIT SEMICOLON NAME ASSIGN INTEGER_LIT)],
        'both comments skipped, two assignments remain' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens consumed silently' => sub {
    is(
        types_of('x := 42'),
        [qw(NAME ASSIGN INTEGER_LIT)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed silently' => sub {
    is(
        types_of("begin\n\tinteger x\nend"),
        [qw(BEGIN INTEGER NAME END)],
        'only value tokens in output'
    );
};

# ============================================================================
# Composite: minimal program
# ============================================================================

subtest 'minimal program: begin integer x; x := 42 end' => sub {
    my $src = 'begin integer x; x := 42 end';
    is(
        types_of($src),
        [qw(BEGIN INTEGER NAME SEMICOLON NAME ASSIGN INTEGER_LIT END)],
        'correct token sequence for minimal program'
    );
};

subtest 'if/then/else' => sub {
    my $src = 'if x = 0 then x := 1 else x := 2';
    is(
        types_of($src),
        [qw(IF NAME EQ INTEGER_LIT THEN NAME ASSIGN INTEGER_LIT
            ELSE NAME ASSIGN INTEGER_LIT)],
        'if/then/else token sequence'
    );
};

subtest 'for loop' => sub {
    my $src = 'for i := 1 step 1 until 10 do x := x + 1';
    is(
        types_of($src),
        [qw(FOR NAME ASSIGN INTEGER_LIT STEP INTEGER_LIT UNTIL INTEGER_LIT
            DO NAME ASSIGN NAME PLUS INTEGER_LIT)],
        'for loop token sequence'
    );
};

subtest 'array subscript' => sub {
    my $src = 'A[i, j]';
    is(
        types_of($src),
        [qw(NAME LBRACKET NAME COMMA NAME RBRACKET)],
        'array subscript token sequence'
    );
};

subtest 'arithmetic expression' => sub {
    my $src = 'x + y * z - w / v';
    is(
        types_of($src),
        [qw(NAME PLUS NAME STAR NAME MINUS NAME SLASH NAME)],
        'arithmetic expression token sequence'
    );
};

subtest 'exponentiation ** and ^' => sub {
    my $src = 'a ** b ^ c';
    is(
        types_of($src),
        [qw(NAME POWER NAME CARET NAME)],
        '** is POWER, ^ is CARET'
    );
};

subtest 'boolean expression with keywords' => sub {
    my $src = 'not x and y or z';
    is(
        types_of($src),
        [qw(NOT NAME AND NAME OR NAME)],
        'boolean operator keywords tokenized correctly'
    );
};

subtest 'procedure declaration header' => sub {
    my $src = 'real procedure sum(x, y)';
    is(
        types_of($src),
        [qw(REAL PROCEDURE NAME LPAREN NAME COMMA NAME RPAREN)],
        'procedure declaration header'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: x := 42' => sub {
    # Input: x := 42
    # col:   1234567
    my $tokens = CodingAdventures::AlgolLexer->tokenize('x := 42');
    is( $tokens->[0]{col}, 1, 'x at col 1' );
    is( $tokens->[1]{col}, 3, ':= at col 3' );
    is( $tokens->[2]{col}, 6, '42 at col 6' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::AlgolLexer->tokenize('x := 42');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::AlgolLexer->tokenize('42');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

subtest 'hash character raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolLexer->tokenize('#') },
        'unexpected # causes die'
    );
};

done_testing;
