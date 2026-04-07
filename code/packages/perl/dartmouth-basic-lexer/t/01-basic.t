use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DartmouthBasicLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================
#
# These helpers extract specific fields from the token list for easier testing.
# We always exclude EOF from type and value lists so assertions stay concise.

# Return a list of token types (excluding EOF) from a source string.
sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Return a list of token values (excluding EOF) from a source string.
sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Return token pairs [type, value] (excluding EOF) for both type and value
# checking in one assertion.
sub pairs_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize($source);
    return [ map { [$_->{type}, $_->{value}] } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize("   \t  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# LINE_NUM disambiguation
# ============================================================================
#
# The trickiest part of BASIC lexing: bare integers serve two roles.
#
#   10 LET X = 5    →  LINE_NUM(10)
#   GOTO 10         →  NUMBER(10)    (inside a statement — not line start)
#
# The post-tokenize hook relabels the first NUMBER on each line as LINE_NUM.

subtest 'integer at start of first line is LINE_NUM' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 5');
    is( $toks->[0]{type},  'LINE_NUM', 'first token is LINE_NUM' );
    is( $toks->[0]{value}, '10',       'value is 10' );
};

subtest 'integer inside statement is NUMBER not LINE_NUM' => sub {
    # "5" in "X = 5" is a value, not a line label.
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 5');
    # tokens: LINE_NUM(10) KEYWORD(LET) NAME(X) EQ(=) NUMBER(5) NEWLINE
    is( $toks->[4]{type},  'NUMBER', '5 in expression is NUMBER' );
    is( $toks->[4]{value}, '5',      'value is 5' );
};

subtest 'GOTO target is NUMBER not LINE_NUM' => sub {
    # The target of GOTO is a number, not a line label.
    # "30 GOTO 10" — 30 is LINE_NUM, 10 inside statement is NUMBER.
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('30 GOTO 10');
    is( $toks->[0]{type},  'LINE_NUM', '30 is LINE_NUM' );
    is( $toks->[2]{type},  'NUMBER',   '10 after GOTO is NUMBER' );
    is( $toks->[2]{value}, '10',       'value is 10' );
};

subtest 'second line integer is LINE_NUM, not NUMBER' => sub {
    # A multi-line program: each leading integer should be LINE_NUM.
    my $src = "10 LET X = 1\n20 PRINT X";
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize($src);
    # Find the first token after the NEWLINE
    my ($newline_idx) = grep { $toks->[$_]{type} eq 'NEWLINE' } 0..$#$toks;
    my $after_newline = $toks->[$newline_idx + 1];
    is( $after_newline->{type},  'LINE_NUM', 'token after NEWLINE is LINE_NUM' );
    is( $after_newline->{value}, '20',       'value is 20' );
};

# ============================================================================
# Complete simple programs (LINE_NUM context)
# ============================================================================

subtest '10 LET X = 5 — full token sequence' => sub {
    is(
        types_of('10 LET X = 5'),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE)],
        'LET statement token types'
    );
    is(
        values_of('10 LET X = 5'),
        ['10', 'LET', 'X', '=', '5', "\n"],
        'LET statement token values'
    );
};

subtest '20 PRINT X, Y — PRINT with comma' => sub {
    is(
        types_of('20 PRINT X, Y'),
        [qw(LINE_NUM KEYWORD NAME COMMA NAME NEWLINE)],
        'PRINT with comma separator'
    );
};

subtest '30 GOTO 10 — GOTO statement' => sub {
    is(
        types_of('30 GOTO 10'),
        [qw(LINE_NUM KEYWORD NUMBER NEWLINE)],
        'GOTO target is NUMBER'
    );
};

subtest '40 IF X > 0 THEN 100 — IF/THEN' => sub {
    is(
        types_of('40 IF X > 0 THEN 100'),
        [qw(LINE_NUM KEYWORD NAME GT NUMBER KEYWORD NUMBER NEWLINE)],
        'IF/THEN statement'
    );
};

subtest '50 FOR I = 1 TO 10 STEP 2 — FOR loop' => sub {
    is(
        types_of('50 FOR I = 1 TO 10 STEP 2'),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER KEYWORD NUMBER KEYWORD NUMBER NEWLINE)],
        'FOR loop header'
    );
};

subtest '60 DEF FNA(X) = X * X — user-defined function' => sub {
    is(
        types_of('60 DEF FNA(X) = X * X'),
        [qw(LINE_NUM KEYWORD USER_FN LPAREN NAME RPAREN EQ NAME STAR NAME NEWLINE)],
        'DEF with USER_FN'
    );
};

subtest '70 LET Y = SIN(X) + COS(X) — built-in functions' => sub {
    is(
        types_of('70 LET Y = SIN(X) + COS(X)'),
        [qw(LINE_NUM KEYWORD NAME EQ BUILTIN_FN LPAREN NAME RPAREN
            PLUS BUILTIN_FN LPAREN NAME RPAREN NEWLINE)],
        'expression with BUILTIN_FN'
    );
};

# ============================================================================
# Multi-line program
# ============================================================================

subtest 'multi-line program: 10 LET / 20 PRINT / 30 END' => sub {
    my $src = "10 LET X = 1\n20 PRINT X\n30 END";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE
            LINE_NUM KEYWORD NAME NEWLINE
            LINE_NUM KEYWORD NEWLINE)],
        'three-line BASIC program'
    );
};

# ============================================================================
# All 20 BASIC keywords
# ============================================================================
#
# Every reserved word must tokenize as KEYWORD, never as NAME.
# The @case_insensitive flag means we test both lowercase and uppercase forms.

subtest 'keyword: LET' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('LET');
    is( $toks->[0]{type}, 'KEYWORD', 'LET → KEYWORD' );
    is( $toks->[0]{value}, 'LET', 'value is LET' );
};

subtest 'keyword: PRINT' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('PRINT');
    is( $toks->[0]{type}, 'KEYWORD', 'PRINT → KEYWORD' );
};

subtest 'keyword: INPUT' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('INPUT');
    is( $toks->[0]{type}, 'KEYWORD', 'INPUT → KEYWORD' );
};

subtest 'keyword: IF' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('IF');
    is( $toks->[0]{type}, 'KEYWORD', 'IF → KEYWORD' );
};

subtest 'keyword: THEN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('THEN');
    is( $toks->[0]{type}, 'KEYWORD', 'THEN → KEYWORD' );
};

subtest 'keyword: GOTO' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('GOTO');
    is( $toks->[0]{type}, 'KEYWORD', 'GOTO → KEYWORD' );
};

subtest 'keyword: GOSUB' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('GOSUB');
    is( $toks->[0]{type}, 'KEYWORD', 'GOSUB → KEYWORD' );
};

subtest 'keyword: RETURN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('RETURN');
    is( $toks->[0]{type}, 'KEYWORD', 'RETURN → KEYWORD' );
};

subtest 'keyword: FOR' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('FOR');
    is( $toks->[0]{type}, 'KEYWORD', 'FOR → KEYWORD' );
};

subtest 'keyword: TO' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('TO');
    is( $toks->[0]{type}, 'KEYWORD', 'TO → KEYWORD' );
};

subtest 'keyword: STEP' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('STEP');
    is( $toks->[0]{type}, 'KEYWORD', 'STEP → KEYWORD' );
};

subtest 'keyword: NEXT' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('NEXT');
    is( $toks->[0]{type}, 'KEYWORD', 'NEXT → KEYWORD' );
};

subtest 'keyword: END' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('END');
    is( $toks->[0]{type}, 'KEYWORD', 'END → KEYWORD' );
};

subtest 'keyword: STOP' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('STOP');
    is( $toks->[0]{type}, 'KEYWORD', 'STOP → KEYWORD' );
};

subtest 'keyword: REM' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('REM');
    is( $toks->[0]{type}, 'KEYWORD', 'REM → KEYWORD' );
    is( $toks->[0]{value}, 'REM', 'value is REM' );
};

subtest 'keyword: READ' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('READ');
    is( $toks->[0]{type}, 'KEYWORD', 'READ → KEYWORD' );
};

subtest 'keyword: DATA' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('DATA');
    is( $toks->[0]{type}, 'KEYWORD', 'DATA → KEYWORD' );
};

subtest 'keyword: RESTORE' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('RESTORE');
    is( $toks->[0]{type}, 'KEYWORD', 'RESTORE → KEYWORD' );
};

subtest 'keyword: DIM' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('DIM');
    is( $toks->[0]{type}, 'KEYWORD', 'DIM → KEYWORD' );
};

subtest 'keyword: DEF' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('DEF');
    is( $toks->[0]{type}, 'KEYWORD', 'DEF → KEYWORD' );
};

# ============================================================================
# Case insensitivity
# ============================================================================
#
# Because @case_insensitive true uppercases the whole source, lowercase and
# mixed-case input produce identical tokens to their uppercase counterparts.

subtest 'case insensitivity: "10 print x" == "10 PRINT X"' => sub {
    my $lower = types_of('10 print x');
    my $upper = types_of('10 PRINT X');
    is( $lower, $upper, 'lowercase print matches uppercase PRINT' );
};

subtest 'case insensitivity: "20 Let A = 1" == "20 LET A = 1"' => sub {
    is(
        types_of('20 Let A = 1'),
        types_of('20 LET A = 1'),
        'mixed-case LET matches uppercase'
    );
};

subtest 'case insensitivity: "30 goto 20" == "30 GOTO 20"' => sub {
    is(
        types_of('30 goto 20'),
        types_of('30 GOTO 20'),
        'lowercase goto matches uppercase'
    );
};

subtest 'case insensitivity: values are uppercased' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 let x = 5');
    # After uc(), 'let' becomes 'LET', 'x' becomes 'X'
    is( $toks->[1]{value}, 'LET', 'let uppercased to LET' );
    is( $toks->[2]{value}, 'X',   'x uppercased to X' );
};

# ============================================================================
# All 11 built-in functions
# ============================================================================
#
# These must tokenize as BUILTIN_FN, not as NAME.
# They must appear before the NAME rule in the grammar so they are not
# partially matched (e.g., SIN would match NAME "SI" then NAME "N").

subtest 'BUILTIN_FN: SIN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('SIN');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'SIN → BUILTIN_FN' );
    is( $toks->[0]{value}, 'SIN', 'value is SIN' );
};

subtest 'BUILTIN_FN: COS' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('COS');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'COS → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: TAN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('TAN');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'TAN → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: ATN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('ATN');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'ATN → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: EXP' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('EXP');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'EXP → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: LOG' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('LOG');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'LOG → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: ABS' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('ABS');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'ABS → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: SQR' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('SQR');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'SQR → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: INT' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('INT');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'INT → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: RND' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('RND');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'RND → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: SGN' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('SGN');
    is( $toks->[0]{type}, 'BUILTIN_FN', 'SGN → BUILTIN_FN' );
};

subtest 'BUILTIN_FN: lowercase sin is uppercased and matched' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('sin');
    is( $toks->[0]{type},  'BUILTIN_FN', 'sin → BUILTIN_FN after uc()' );
    is( $toks->[0]{value}, 'SIN',         'value is uppercased to SIN' );
};

# ============================================================================
# User-defined functions (FNA..FNZ)
# ============================================================================
#
# User-defined functions follow the pattern FN + exactly one uppercase letter.
# Examples: FNA, FNB, FNZ.  They must appear before NAME in the grammar so
# that "FNA" is not tokenized as NAME("FN") + NAME("A").

subtest 'USER_FN: FNA' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('FNA');
    is( $toks->[0]{type},  'USER_FN', 'FNA → USER_FN' );
    is( $toks->[0]{value}, 'FNA',      'value is FNA' );
};

subtest 'USER_FN: FNZ' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('FNZ');
    is( $toks->[0]{type},  'USER_FN', 'FNZ → USER_FN' );
    is( $toks->[0]{value}, 'FNZ',      'value is FNZ' );
};

subtest 'USER_FN: lowercase fna uppercased to FNA' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('fna');
    is( $toks->[0]{type},  'USER_FN', 'fna → USER_FN' );
    is( $toks->[0]{value}, 'FNA',      'value uppercased to FNA' );
};

# ============================================================================
# Variable names (NAME)
# ============================================================================
#
# In Dartmouth BASIC 1964, variable names are exactly:
#   - One uppercase letter: A..Z  (26 scalars)
#   - One uppercase letter + one digit: A0..Z9  (260 more)
# Total: 286 possible variable names.

subtest 'NAME: single letter X' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('X');
    is( $toks->[0]{type},  'NAME', 'X → NAME' );
    is( $toks->[0]{value}, 'X',     'value is X' );
};

subtest 'NAME: letter + digit A1' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('A1');
    is( $toks->[0]{type},  'NAME', 'A1 → NAME' );
    is( $toks->[0]{value}, 'A1',    'value is A1' );
};

subtest 'NAME: letter + digit Z9' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('Z9');
    is( $toks->[0]{type},  'NAME', 'Z9 → NAME' );
    is( $toks->[0]{value}, 'Z9',    'value is Z9' );
};

subtest 'NAME: lowercase x is uppercased to X' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('x');
    is( $toks->[0]{type},  'NAME', 'x → NAME after uc()' );
    is( $toks->[0]{value}, 'X',     'value uppercased to X' );
};

# ============================================================================
# Numeric literals (NUMBER)
# ============================================================================
#
# All numbers in BASIC 1964 are stored as floats internally, even integers.
# The grammar handles five formats:
#   42        plain integer
#   3.14      decimal
#   .5        leading-dot decimal
#   1.5E3     scientific (= 1500.0)
#   1.5E-3    negative exponent (= 0.0015)

subtest 'NUMBER: plain integer 42' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 42');
    # LINE_NUM KEYWORD NAME EQ NUMBER ...
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '42',     'value is 42' );
};

subtest 'NUMBER: decimal 3.14' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 3.14');
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '3.14',   'value is 3.14' );
};

subtest 'NUMBER: leading dot .5' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = .5');
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '.5',     'value is .5' );
};

subtest 'NUMBER: scientific 1.5E3' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 1.5E3');
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '1.5E3',  'value is 1.5E3' );
};

subtest 'NUMBER: negative exponent 1.5E-3' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 1.5E-3');
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '1.5E-3', 'value is 1.5E-3' );
};

subtest 'NUMBER: integer + exponent 1E10' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 1E10');
    is( $toks->[4]{type},  'NUMBER', 'type is NUMBER' );
    is( $toks->[4]{value}, '1E10',   'value is 1E10' );
};

# ============================================================================
# String literals
# ============================================================================
#
# Strings are double-quoted.  The original 1964 spec had no escape sequences —
# a double quote cannot appear inside a string literal.  The token value
# includes the surrounding double quotes.

subtest 'STRING: basic string literal' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 PRINT "HELLO WORLD"');
    # LINE_NUM KEYWORD STRING NEWLINE
    is( $toks->[2]{type},  'STRING',         'type is STRING' );
    is( $toks->[2]{value}, '"HELLO WORLD"',  'value includes quotes' );
};

subtest 'STRING: empty string' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 PRINT ""');
    is( $toks->[2]{type},  'STRING', 'type is STRING' );
    is( $toks->[2]{value}, '""',     'value is ""' );
};

subtest 'STRING: string with spaces and punctuation' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 PRINT "HELLO, WORLD!"');
    is( $toks->[2]{type},  'STRING',           'type is STRING' );
    is( $toks->[2]{value}, '"HELLO, WORLD!"',  'value includes quotes and content' );
};

# ============================================================================
# Multi-character operators (priority tests)
# ============================================================================
#
# LE, GE, NE must be matched as single tokens before their component characters.
# If the grammar order were wrong:
#   "<=" would lex as LT("< ") then EQ("=") — wrong!
#   ">=" would lex as GT(">") then EQ("=") — wrong!
#   "<>" would lex as LT("<") then GT(">") — wrong!

subtest 'LE <= is a single token' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 IF X <= Y THEN 50');
    # Find the LE token
    my ($le) = grep { $_->{type} eq 'LE' } @$toks;
    ok( $le, 'LE token found' );
    is( $le->{value}, '<=', 'LE value is <=' );
    # Confirm it is not two separate tokens
    my @lt_eq = grep { $_->{type} eq 'LT' || $_->{type} eq 'EQ' } @$toks;
    is( scalar @lt_eq, 0, 'no separate LT or EQ tokens for <=' );
};

subtest 'GE >= is a single token' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 IF X >= Y THEN 50');
    my ($ge) = grep { $_->{type} eq 'GE' } @$toks;
    ok( $ge, 'GE token found' );
    is( $ge->{value}, '>=', 'GE value is >=' );
    my @gt_eq = grep { $_->{type} eq 'GT' || $_->{type} eq 'EQ' } @$toks;
    is( scalar @gt_eq, 0, 'no separate GT or EQ tokens for >=' );
};

subtest 'NE <> is a single token' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 IF X <> Y THEN 50');
    my ($ne) = grep { $_->{type} eq 'NE' } @$toks;
    ok( $ne, 'NE token found' );
    is( $ne->{value}, '<>', 'NE value is <>' );
    my @lt_gt = grep { $_->{type} eq 'LT' || $_->{type} eq 'GT' } @$toks;
    is( scalar @lt_gt, 0, 'no separate LT or GT tokens for <>' );
};

# ============================================================================
# Single-character operators
# ============================================================================

subtest 'PLUS +' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('+');
    is( $toks->[0]{type},  'PLUS', 'type is PLUS' );
    is( $toks->[0]{value}, '+',     'value is +' );
};

subtest 'MINUS -' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('-');
    is( $toks->[0]{type},  'MINUS', 'type is MINUS' );
    is( $toks->[0]{value}, '-',      'value is -' );
};

subtest 'STAR *' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('*');
    is( $toks->[0]{type},  'STAR', 'type is STAR' );
    is( $toks->[0]{value}, '*',     'value is *' );
};

subtest 'SLASH /' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('/');
    is( $toks->[0]{type},  'SLASH', 'type is SLASH' );
    is( $toks->[0]{value}, '/',      'value is /' );
};

subtest 'CARET ^' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('^');
    is( $toks->[0]{type},  'CARET', 'type is CARET' );
    is( $toks->[0]{value}, '^',      'value is ^' );
};

subtest 'EQ =' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('=');
    is( $toks->[0]{type},  'EQ', 'type is EQ' );
    is( $toks->[0]{value}, '=',   'value is =' );
};

subtest 'LT <' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('<');
    is( $toks->[0]{type},  'LT', 'type is LT' );
    is( $toks->[0]{value}, '<',   'value is <' );
};

subtest 'GT >' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('>');
    is( $toks->[0]{type},  'GT', 'type is GT' );
    is( $toks->[0]{value}, '>',   'value is >' );
};

# ============================================================================
# Delimiters and separators
# ============================================================================

subtest 'LPAREN (' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('(');
    is( $toks->[0]{type},  'LPAREN', 'type is LPAREN' );
    is( $toks->[0]{value}, '(',       'value is (' );
};

subtest 'RPAREN )' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize(')');
    is( $toks->[0]{type},  'RPAREN', 'type is RPAREN' );
    is( $toks->[0]{value}, ')',       'value is )' );
};

subtest 'COMMA , — PRINT zone separator' => sub {
    # PRINT X, Y advances to the next print zone (multiple of col 14) before Y.
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize(',');
    is( $toks->[0]{type},  'COMMA', 'type is COMMA' );
    is( $toks->[0]{value}, ',',      'value is ,' );
};

subtest 'SEMICOLON ; — PRINT concatenation separator' => sub {
    # PRINT X; Y prints X immediately followed by Y with no space.
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize(';');
    is( $toks->[0]{type},  'SEMICOLON', 'type is SEMICOLON' );
    is( $toks->[0]{value}, ';',          'value is ;' );
};

subtest 'PRINT comma vs semicolon distinction' => sub {
    my $comma_toks = types_of('10 PRINT X, Y');
    my $semi_toks  = types_of('10 PRINT X; Y');
    my ($comma_sep) = grep { $_ eq 'COMMA' } @$comma_toks;
    my ($semi_sep)  = grep { $_ eq 'SEMICOLON' } @$semi_toks;
    ok( $comma_sep, 'COMMA separator in PRINT X, Y' );
    ok( $semi_sep,  'SEMICOLON separator in PRINT X; Y' );
};

# ============================================================================
# NEWLINE token (significant, not skipped)
# ============================================================================
#
# In Dartmouth BASIC, the line structure is:
#   LINE_NUM statement NEWLINE
#
# The NEWLINE is the statement terminator.  It must appear in the token stream
# so the parser knows where each statement ends.

subtest 'NEWLINE is present in token stream' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize("10 LET X = 1\n");
    my ($nl) = grep { $_->{type} eq 'NEWLINE' } @$toks;
    ok( $nl, 'NEWLINE token present' );
    is( $nl->{value}, "\n", 'NEWLINE value is \n' );
};

subtest 'Windows CRLF newline is a single NEWLINE token' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize("10 LET X = 1\r\n");
    my @newlines = grep { $_->{type} eq 'NEWLINE' } @$toks;
    is( scalar @newlines, 1, 'exactly one NEWLINE token for \\r\\n' );
};

# ============================================================================
# REM comment handling
# ============================================================================
#
# REM introduces a comment that runs to the end of the line.
# Everything after REM is suppressed (not emitted as tokens).
# The REM token itself and the following NEWLINE are kept.

subtest 'REM: comment text is suppressed' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 REM THIS IS A COMMENT');
    # Should see: LINE_NUM(10) KEYWORD(REM) NEWLINE EOF
    # NAME tokens for THIS, IS, A, COMMENT are all suppressed.
    my @non_eof = grep { $_->{type} ne 'EOF' } @$toks;
    is( scalar @non_eof, 3, '3 non-EOF tokens: LINE_NUM, KEYWORD, NEWLINE' );
    is( $non_eof[0]{type},  'LINE_NUM', 'first is LINE_NUM' );
    is( $non_eof[1]{type},  'KEYWORD',  'second is KEYWORD(REM)' );
    is( $non_eof[1]{value}, 'REM',      'keyword value is REM' );
    is( $non_eof[2]{type},  'NEWLINE',  'third is NEWLINE' );
};

subtest 'REM: subsequent lines are unaffected' => sub {
    my $src = "10 REM\n20 LET X = 1";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NEWLINE LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE)],
        'line after REM tokenizes normally'
    );
};

subtest 'REM: empty REM (no comment text)' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize("10 REM\n");
    my @non_eof = grep { $_->{type} ne 'EOF' } @$toks;
    is( $non_eof[0]{type},  'LINE_NUM', 'LINE_NUM present' );
    is( $non_eof[1]{type},  'KEYWORD',  'KEYWORD(REM) present' );
    is( $non_eof[1]{value}, 'REM',      'keyword is REM' );
    is( $non_eof[2]{type},  'NEWLINE',  'NEWLINE present' );
};

subtest 'REM: full two-line program with comment' => sub {
    my $src = "10 REM THIS IS A COMMENT\n20 LET X = 1";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NEWLINE LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE)],
        'REM line followed by LET line'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================
#
# Horizontal whitespace (spaces, tabs) is silently consumed.
# Tokens are not emitted for whitespace.

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('10 LET X = 5'),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'extra spaces produce same tokens as single spaces' => sub {
    my $single = types_of('10  LET  X  =  5');
    my $normal = types_of('10 LET X = 5');
    is( $single, $normal, 'extra spaces are normalized away' );
};

subtest 'no-space tokenization (10 LET X=5)' => sub {
    is(
        types_of('10 LET X=5'),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER NEWLINE)],
        'spaces not required around ='
    );
};

# ============================================================================
# GOSUB / RETURN subroutine example
# ============================================================================

subtest 'GOSUB and RETURN' => sub {
    my $src = "10 GOSUB 100\n20 RETURN";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NUMBER NEWLINE LINE_NUM KEYWORD NEWLINE)],
        'GOSUB + RETURN sequence'
    );
};

# ============================================================================
# FOR loop full syntax
# ============================================================================
#
# FOR I = start TO limit STEP increment
# The body follows; a NEXT I ends the loop.

subtest 'complete FOR/NEXT loop' => sub {
    my $src = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NAME EQ NUMBER KEYWORD NUMBER NEWLINE
            LINE_NUM KEYWORD NAME NEWLINE
            LINE_NUM KEYWORD NAME NEWLINE)],
        'FOR / PRINT / NEXT loop'
    );
};

# ============================================================================
# DIM array declaration
# ============================================================================

subtest 'DIM statement' => sub {
    is(
        types_of('10 DIM A(10)'),
        [qw(LINE_NUM KEYWORD NAME LPAREN NUMBER RPAREN NEWLINE)],
        'DIM A(10)'
    );
};

# ============================================================================
# READ / DATA / RESTORE
# ============================================================================

subtest 'READ and DATA' => sub {
    my $src = "10 READ X\n20 DATA 3.14";
    is(
        types_of($src),
        [qw(LINE_NUM KEYWORD NAME NEWLINE LINE_NUM KEYWORD NUMBER NEWLINE)],
        'READ and DATA statements'
    );
};

subtest 'RESTORE' => sub {
    is(
        types_of('10 RESTORE'),
        [qw(LINE_NUM KEYWORD NEWLINE)],
        'RESTORE statement'
    );
};

# ============================================================================
# INPUT statement
# ============================================================================

subtest 'INPUT statement' => sub {
    is(
        types_of('10 INPUT X'),
        [qw(LINE_NUM KEYWORD NAME NEWLINE)],
        'INPUT X'
    );
};

# ============================================================================
# STOP statement
# ============================================================================

subtest 'STOP statement' => sub {
    is(
        types_of('10 STOP'),
        [qw(LINE_NUM KEYWORD NEWLINE)],
        'STOP statement'
    );
};

# ============================================================================
# Arithmetic expressions
# ============================================================================

subtest 'arithmetic: X + Y * Z - W / V' => sub {
    is(
        types_of('10 LET R = X + Y * Z - W / V'),
        [qw(LINE_NUM KEYWORD NAME EQ NAME PLUS NAME STAR NAME MINUS NAME SLASH NAME NEWLINE)],
        'arithmetic operators'
    );
};

subtest 'exponentiation: X ^ 2' => sub {
    is(
        types_of('10 LET Y = X ^ 2'),
        [qw(LINE_NUM KEYWORD NAME EQ NAME CARET NUMBER NEWLINE)],
        'CARET is exponentiation'
    );
};

subtest 'parenthesized expression' => sub {
    is(
        types_of('10 LET Y = (X + 1) * 2'),
        [qw(LINE_NUM KEYWORD NAME EQ LPAREN NAME PLUS NUMBER RPAREN STAR NUMBER NEWLINE)],
        'parenthesized expression'
    );
};

# ============================================================================
# Comparison operators in IF
# ============================================================================

subtest 'IF with all comparison operators' => sub {
    my @ops = (
        [ '10 IF X < Y THEN 99',  'LT' ],
        [ '10 IF X > Y THEN 99',  'GT' ],
        [ '10 IF X = Y THEN 99',  'EQ' ],
        [ '10 IF X <= Y THEN 99', 'LE' ],
        [ '10 IF X >= Y THEN 99', 'GE' ],
        [ '10 IF X <> Y THEN 99', 'NE' ],
    );
    for my $pair (@ops) {
        my ($src, $expected_op) = @$pair;
        my $toks = CodingAdventures::DartmouthBasicLexer->tokenize($src);
        # token order: LINE_NUM KEYWORD NAME <OP> NAME KEYWORD NUMBER NEWLINE
        is( $toks->[3]{type}, $expected_op, "$expected_op found in: $src" );
    }
};

# ============================================================================
# EOF sentinel
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize('10 END');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

subtest 'EOF present even for empty input' => sub {
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize('');
    is( $tokens->[-1]{type}, 'EOF', 'EOF is last even for empty input' );
};

# ============================================================================
# Error recovery (UNKNOWN tokens)
# ============================================================================
#
# The grammar has an `errors: UNKNOWN = /./` catch-all.
# Instead of dying, unrecognized characters produce UNKNOWN tokens.
# The lexer continues after the bad character so subsequent tokens are fine.

subtest 'UNKNOWN token for @' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET @ = 1');
    my ($unk) = grep { $_->{type} eq 'UNKNOWN' } @$toks;
    ok( $unk, 'UNKNOWN token emitted for @' );
    is( $unk->{value}, '@', 'UNKNOWN value is @' );
};

subtest 'UNKNOWN token does not stop lexing' => sub {
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET @ = 1');
    # After the UNKNOWN(@) we should still get EQ(=) and NUMBER(1)
    my @types = map { $_->{type} } grep { $_->{type} ne 'EOF' } @$toks;
    ok( (grep { $_ eq 'EQ' } @types),     'EQ token still present after UNKNOWN' );
    ok( (grep { $_ eq 'NUMBER' } @types), 'NUMBER token still present after UNKNOWN' );
};

# ============================================================================
# Position tracking
# ============================================================================
#
# Each token records its line and column so error messages can pinpoint
# the location in the source.

subtest 'all non-EOF tokens on line 1 for single-line source' => sub {
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 5');
    # Exclude EOF: after the implicit trailing NEWLINE is processed, the line
    # counter increments to 2 before EOF is emitted, so EOF lives on line 2.
    # All content tokens (LINE_NUM through NEWLINE) are on line 1.
    my @content = grep { $_->{type} ne 'EOF' } @$tokens;
    for my $tok (@content) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

subtest 'second line tokens are on line 2' => sub {
    my $src = "10 LET X = 1\n20 END";
    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize($src);
    # Find END token — it's on the second line
    my ($end_tok) = grep { $_->{type} eq 'KEYWORD' && $_->{value} eq 'END' } @$tokens;
    ok( $end_tok, 'END token found' );
    is( $end_tok->{line}, 2, 'END is on line 2' );
};

done_testing;
