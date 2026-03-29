use strict;
use warnings;
use Test2::V0;

# ============================================================================
# CodingAdventures::ExcelLexer — basic tokenization tests
# ============================================================================
#
# This test file exercises the Excel formula lexer comprehensively:
#
#   - Module loads correctly
#   - Empty and whitespace-only inputs produce only EOF
#   - EQUALS token (formula prefix "=")
#   - CELL references: A1, $B$2, mixed absolute/relative
#   - NUMBER literals: integer, float, decimal fraction, scientific
#   - STRING literals: simple, empty, doubled-quote escape
#   - TRUE / FALSE keyword detection (case-insensitive via lowercasing)
#   - ERROR_CONSTANT tokens: #DIV/0!, #VALUE!, #REF!, #NAME?, #N/A, #NULL!
#   - Arithmetic operators: + - * / ^ &
#   - Comparison operators: = <> <= >= < >
#   - PERCENT postfix
#   - LPAREN / RPAREN
#   - COLON (range separator)
#   - COMMA and SEMICOLON (argument separators)
#   - SPACE token (intersection operator)
#   - REF_PREFIX: bare and quoted sheet references
#   - STRUCTURED_KEYWORD and STRUCTURED_COLUMN
#   - AT (@) for dynamic arrays
#   - NAME for identifiers
#   - Composite formula: =SUM(A1:B10)
#   - Composite formula: =IF(A1>0,"pos","neg")
#   - Cross-sheet: =Sheet1!A1
#   - Error on unexpected character

ok( eval { require CodingAdventures::ExcelLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF and SPACE)
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::ExcelLexer->tokenize($source);
    return [
        map  { $_->{type} }
        grep { $_->{type} ne 'EOF' && $_->{type} ne 'SPACE' }
        @$tokens
    ];
}

sub types_of_with_space {
    my ($source) = @_;
    my $tokens = CodingAdventures::ExcelLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::ExcelLexer->tokenize($source);
    return [
        map  { $_->{value} }
        grep { $_->{type} ne 'EOF' && $_->{type} ne 'SPACE' }
        @$tokens
    ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'non-space whitespace produces only EOF' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize("\t\r\n");
    is( scalar @$tokens, 1,     '1 token after skipping non-space whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# EQUALS token
# ============================================================================

subtest 'standalone = produces EQUALS' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

# ============================================================================
# CELL tokens
# ============================================================================

subtest 'simple cell A1' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('A1');
    is( $tokens->[0]{type},  'CELL', 'type is CELL' );
    is( $tokens->[0]{value}, 'a1',   'value lowercased to a1' );
};

subtest 'absolute cell $B$2' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('$B$2');
    is( $tokens->[0]{type},  'CELL',  'type is CELL' );
    is( $tokens->[0]{value}, '$b$2', 'value lowercased to $b$2' );
};

subtest 'mixed absolute $C3' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('$C3');
    is( $tokens->[0]{type}, 'CELL', 'type is CELL' );
};

subtest 'multi-letter column AB100' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('AB100');
    is( $tokens->[0]{type},  'CELL',   'type is CELL' );
    is( $tokens->[0]{value}, 'ab100', 'value is ab100' );
};

# ============================================================================
# NUMBER tokens
# ============================================================================

subtest 'integer 42' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'float 3.14' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('3.14');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '3.14',   'value is 3.14' );
};

subtest 'decimal fraction .5' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('.5');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '.5',     'value is .5' );
};

subtest 'scientific notation 1.5e10' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('1.5e10');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

# ============================================================================
# STRING tokens
# ============================================================================

subtest 'simple string "hello"' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty string ""' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'doubled-quote escape "say ""hi"""' => sub {
    # Excel escapes a literal " inside a string by doubling it.
    my $tokens = CodingAdventures::ExcelLexer->tokenize('"say ""hi"""');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/""/, 'value contains doubled quote' );
};

# ============================================================================
# Boolean keyword tokens
# ============================================================================

subtest 'TRUE keyword' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('TRUE');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'true', 'value lowercased to true' );
};

subtest 'FALSE keyword' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('FALSE');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'false', 'value lowercased to false' );
};

subtest 'mixed-case True is still TRUE' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('True');
    is( $tokens->[0]{type}, 'TRUE', 'type is TRUE after case normalization' );
};

# ============================================================================
# ERROR_CONSTANT tokens
# ============================================================================

subtest '#DIV/0!' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#DIV/0!');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

subtest '#VALUE!' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#VALUE!');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

subtest '#REF!' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#REF!');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

subtest '#NAME?' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#NAME?');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

subtest '#N/A' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#N/A');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

subtest '#NULL!' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('#NULL!');
    is( $tokens->[0]{type}, 'ERROR_CONSTANT', 'type is ERROR_CONSTANT' );
};

# ============================================================================
# Operator tokens
# ============================================================================

subtest 'arithmetic operators +-*/^&' => sub {
    is(
        types_of('+-*/^&'),
        [qw(PLUS MINUS STAR SLASH CARET AMP)],
        'all arithmetic operator types in order'
    );
};

subtest 'comparison operators = <> <= >= < >' => sub {
    is(
        types_of('=<><=>=<>'),
        [qw(EQUALS NOT_EQUALS LESS_EQUALS GREATER_EQUALS LESS_THAN GREATER_THAN)],
        'all comparison operator types in order'
    );
};

subtest 'PERCENT postfix 50%' => sub {
    is( types_of('50%'), [qw(NUMBER PERCENT)], 'NUMBER then PERCENT' );
};

# ============================================================================
# Grouping and separator tokens
# ============================================================================

subtest 'LPAREN and RPAREN' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'COLON range separator A1:B2' => sub {
    is( types_of('A1:B2'), [qw(CELL COLON CELL)], 'CELL COLON CELL' );
};

subtest 'COMMA argument separator 1,2' => sub {
    is( types_of('1,2'), [qw(NUMBER COMMA NUMBER)], 'NUMBER COMMA NUMBER' );
};

subtest 'SEMICOLON locale-specific separator 1;2' => sub {
    is( types_of('1;2'), [qw(NUMBER SEMICOLON NUMBER)], 'NUMBER SEMICOLON NUMBER' );
};

# ============================================================================
# SPACE as intersection operator
# ============================================================================

subtest 'SPACE emitted between two cells A1 B2' => sub {
    is(
        types_of_with_space('A1 B2'),
        [qw(CELL SPACE CELL)],
        'SPACE emitted between cells (intersection operator)'
    );
};

# ============================================================================
# REF_PREFIX tokens
# ============================================================================

subtest 'bare sheet reference Sheet1!A1' => sub {
    is(
        types_of('Sheet1!A1'),
        [qw(REF_PREFIX CELL)],
        'REF_PREFIX then CELL for bare sheet ref'
    );
};

subtest "quoted sheet reference 'My Sheet'!A1" => sub {
    is(
        types_of("'My Sheet'!A1"),
        [qw(REF_PREFIX CELL)],
        'REF_PREFIX then CELL for quoted sheet ref'
    );
};

# ============================================================================
# STRUCTURED_KEYWORD and STRUCTURED_COLUMN
# ============================================================================

subtest 'STRUCTURED_KEYWORD [#Headers]' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('[#Headers]');
    is( $tokens->[0]{type}, 'STRUCTURED_KEYWORD', 'type is STRUCTURED_KEYWORD' );
};

subtest 'STRUCTURED_KEYWORD [#All]' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('[#All]');
    is( $tokens->[0]{type}, 'STRUCTURED_KEYWORD', 'type is STRUCTURED_KEYWORD' );
};

subtest 'STRUCTURED_COLUMN [Amount]' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('[Amount]');
    is( $tokens->[0]{type}, 'STRUCTURED_COLUMN', 'type is STRUCTURED_COLUMN' );
};

# ============================================================================
# AT and NAME tokens
# ============================================================================

subtest 'AT token @' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('@');
    is( $tokens->[0]{type}, 'AT', 'type is AT' );
};

subtest 'NAME identifier MyRange' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('MyRange');
    is( $tokens->[0]{type}, 'NAME', 'type is NAME' );
    is( $tokens->[0]{value}, 'myrange', 'value lowercased to myrange' );
};

# ============================================================================
# Composite formulas
# ============================================================================

subtest '=A1+B2' => sub {
    is(
        types_of('=A1+B2'),
        [qw(EQUALS CELL PLUS CELL)],
        'formula token sequence'
    );
};

subtest '=SUM(A1:B10)' => sub {
    is(
        types_of('=SUM(A1:B10)'),
        [qw(EQUALS NAME LPAREN CELL COLON CELL RPAREN)],
        'SUM formula token sequence'
    );
};

subtest '=IF(A1>0,"pos","neg")' => sub {
    is(
        types_of('=IF(A1>0,"pos","neg")'),
        [qw(EQUALS NAME LPAREN CELL GREATER_THAN NUMBER COMMA STRING COMMA STRING RPAREN)],
        'IF formula token sequence'
    );
};

subtest '=Sheet1!A1 cross-sheet reference' => sub {
    is(
        types_of('=Sheet1!A1'),
        [qw(EQUALS REF_PREFIX CELL)],
        'cross-sheet reference token sequence'
    );
};

subtest '=A1*100% percentage formula' => sub {
    is(
        types_of('=A1*100%'),
        [qw(EQUALS CELL STAR NUMBER PERCENT)],
        'percentage formula token sequence'
    );
};

subtest '={1,2;3,4} array constant' => sub {
    is(
        types_of('={1,2;3,4}'),
        [qw(EQUALS LBRACE NUMBER COMMA NUMBER SEMICOLON NUMBER COMMA NUMBER RBRACE)],
        'array constant token sequence'
    );
};

subtest '=A1&" world" concatenation' => sub {
    is(
        types_of('=A1&" world"'),
        [qw(EQUALS CELL AMP STRING)],
        'concatenation formula token sequence'
    );
};

subtest '=IFERROR(A1/B1,#DIV/0!) error handling formula' => sub {
    is(
        types_of('=IFERROR(A1/B1,#DIV/0!)'),
        [qw(EQUALS NAME LPAREN CELL SLASH CELL COMMA ERROR_CONSTANT RPAREN)],
        'IFERROR formula token sequence'
    );
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always the last token' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking on =A1' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('=A1');
    is( $tokens->[0]{col}, 1, '= at col 1' );
    is( $tokens->[1]{col}, 2, 'A1 at col 2' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::ExcelLexer->tokenize('=A1+B2');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected backtick raises die' => sub {
    ok(
        dies { CodingAdventures::ExcelLexer->tokenize('`') },
        'backtick causes die'
    );
};

done_testing;
