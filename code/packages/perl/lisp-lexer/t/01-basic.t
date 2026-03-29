use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LispLexer;

# ============================================================================
# Helpers
# ============================================================================

# Return arrayref of token types (excluding EOF) from a source string.
sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::LispLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Return arrayref of token values (excluding EOF) from a source string.
sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::LispLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'comment-only produces only EOF' => sub {
    # Lisp line comments: ; to end-of-line
    my $tokens = CodingAdventures::LispLexer->tokenize('; this is a comment');
    is( scalar @$tokens, 1,       '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'multiple comment lines produce only EOF' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize("; line 1\n; line 2");
    is( scalar @$tokens, 1,       '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# NUMBER tokens
# ============================================================================

subtest 'positive integer' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

subtest 'negative integer' => sub {
    # In Lisp, -7 is a number literal, not unary minus applied to 7.
    my $tokens = CodingAdventures::LispLexer->tokenize('-7');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '-7',     'value is -7' );
};

subtest 'multiple numbers separated by spaces' => sub {
    is( types_of('1 2 3'),  [qw(NUMBER NUMBER NUMBER)], 'three NUMBER tokens' );
    is( values_of('1 2 3'), ['1', '2', '3'],             'values are 1 2 3'  );
};

# ============================================================================
# SYMBOL tokens
# ============================================================================
#
# Lisp symbols can contain many punctuation characters.
# In Lisp, + is a function, not an operator.  (+ 1 2) calls the function +.

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('define');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, 'define', 'value is define' );
};

subtest 'lambda keyword' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('lambda');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, 'lambda', 'value is lambda' );
};

subtest 'arithmetic operator + is a symbol' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('+');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, '+',      'value is +' );
};

subtest 'arithmetic operators - * /' => sub {
    is( types_of('- * /'),  [qw(SYMBOL SYMBOL SYMBOL)], 'three SYMBOL tokens' );
    is( values_of('- * /'), ['-', '*', '/'],             'correct values'      );
};

subtest 'comparison operators = < >' => sub {
    is( types_of('= < >'), [qw(SYMBOL SYMBOL SYMBOL)], 'three SYMBOL tokens' );
};

subtest 'predicate symbol null?' => sub {
    # By convention, Lisp predicate functions end with ?
    my $tokens = CodingAdventures::LispLexer->tokenize('null?');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, 'null?',  'value is null?' );
};

subtest 'mutating symbol set!' => sub {
    # By convention, mutating functions end with !
    my $tokens = CodingAdventures::LispLexer->tokenize('set!');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, 'set!',   'value is set!' );
};

subtest 'symbol starting with underscore' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('_internal');
    is( $tokens->[0]{type},  'SYMBOL',    'type is SYMBOL' );
    is( $tokens->[0]{value}, '_internal', 'value is _internal' );
};

# ============================================================================
# STRING tokens
# ============================================================================

subtest 'simple string' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty string' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'string with backslash escape' => sub {
    # The lexer returns the raw source text — escape sequences are NOT decoded.
    my $tokens = CodingAdventures::LispLexer->tokenize('"hello\\nworld"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'escape sequence preserved' );
};

subtest 'string with escaped quote' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('"say \\"hi\\""');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
};

subtest 'string with spaces inside' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('"hello world"');
    is( $tokens->[0]{type},  'STRING',        'type is STRING' );
    is( $tokens->[0]{value}, '"hello world"', 'value preserved' );
};

# ============================================================================
# LPAREN and RPAREN tokens
# ============================================================================

subtest 'empty list ()' => sub {
    is( types_of('()'),  [qw(LPAREN RPAREN)], 'LPAREN RPAREN' );
    is( values_of('()'), ['(', ')'],           'correct values' );
};

subtest 'single LPAREN' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('(');
    is( $tokens->[0]{type},  'LPAREN', 'type is LPAREN' );
    is( $tokens->[0]{value}, '(',      'value is (' );
};

subtest 'single RPAREN' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize(')');
    is( $tokens->[0]{type},  'RPAREN', 'type is RPAREN' );
    is( $tokens->[0]{value}, ')',      'value is )' );
};

# ============================================================================
# QUOTE token
# ============================================================================
#
# The apostrophe ' is a reader macro for (quote ...).
# 'x expands to (quote x) — the symbol x is returned unevaluated.

subtest 'single QUOTE token' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize("'");
    is( $tokens->[0]{type},  'QUOTE', 'type is QUOTE' );
    is( $tokens->[0]{value}, "'",     "value is '" );
};

subtest "'x tokenizes as QUOTE SYMBOL" => sub {
    is( types_of("'x"),  [qw(QUOTE SYMBOL)], 'QUOTE SYMBOL' );
    is( values_of("'x"), ["'", 'x'],          'correct values' );
};

subtest "'(1 2) tokenizes correctly" => sub {
    is(
        types_of("'(1 2)"),
        [qw(QUOTE LPAREN NUMBER NUMBER RPAREN)],
        'QUOTE LPAREN NUMBER NUMBER RPAREN'
    );
};

# ============================================================================
# DOT token
# ============================================================================
#
# DOT is used for cons cell notation.  (a . b) is a pair where car=a, cdr=b.
# This is the fundamental building block of all Lisp lists.

subtest 'single DOT token' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('.');
    is( $tokens->[0]{type},  'DOT', 'type is DOT' );
    is( $tokens->[0]{value}, '.',   'value is .' );
};

subtest 'dotted pair (a . b)' => sub {
    is(
        types_of('(a . b)'),
        [qw(LPAREN SYMBOL DOT SYMBOL RPAREN)],
        'LPAREN SYMBOL DOT SYMBOL RPAREN'
    );
    is( values_of('(a . b)'), ['(', 'a', '.', 'b', ')'], 'correct values' );
};

subtest 'improper list (1 2 . 3)' => sub {
    is(
        types_of('(1 2 . 3)'),
        [qw(LPAREN NUMBER NUMBER DOT NUMBER RPAREN)],
        'improper list token types'
    );
};

# ============================================================================
# Comment handling
# ============================================================================

subtest 'inline comment is consumed' => sub {
    is(
        types_of('define ; this is a comment'),
        [qw(SYMBOL)],
        'only SYMBOL, comment stripped'
    );
};

subtest 'comment between tokens is consumed' => sub {
    is(
        types_of("(\n; comment\nx)"),
        [qw(LPAREN SYMBOL RPAREN)],
        'comment between tokens stripped'
    );
};

subtest 'double-semicolon comment is consumed' => sub {
    is(
        types_of(";; section header\n42"),
        [qw(NUMBER)],
        'only NUMBER, comment stripped'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('( define x 42 )'),
        [qw(LPAREN SYMBOL SYMBOL NUMBER RPAREN)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed silently' => sub {
    is(
        types_of("(\tdefine\nx\t42\n)"),
        [qw(LPAREN SYMBOL SYMBOL NUMBER RPAREN)],
        'only value tokens in output'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking on single line' => sub {
    # Input: (+ 1 2)
    # col:   1234567
    my $tokens = CodingAdventures::LispLexer->tokenize('(+ 1 2)');
    is( $tokens->[0]{col}, 1, '( at col 1' );
    is( $tokens->[1]{col}, 2, '+ at col 2' );
    is( $tokens->[2]{col}, 4, '1 at col 4' );
    is( $tokens->[3]{col}, 6, '2 at col 6' );
    is( $tokens->[4]{col}, 7, ') at col 7' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('(+ 1 2)');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# Composite Lisp expressions
# ============================================================================

subtest 'function call (+ 1 2)' => sub {
    is(
        types_of('(+ 1 2)'),
        [qw(LPAREN SYMBOL NUMBER NUMBER RPAREN)],
        'function call token types'
    );
};

subtest '(define x 42)' => sub {
    is(
        types_of('(define x 42)'),
        [qw(LPAREN SYMBOL SYMBOL NUMBER RPAREN)],
        'define token types'
    );
};

subtest 'nested list (car (cdr x))' => sub {
    is(
        types_of('(car (cdr x))'),
        [qw(LPAREN SYMBOL LPAREN SYMBOL SYMBOL RPAREN RPAREN)],
        'nested list token types'
    );
};

subtest 'two top-level expressions' => sub {
    is(
        types_of('(define x 42) (display x)'),
        [qw(LPAREN SYMBOL SYMBOL NUMBER RPAREN LPAREN SYMBOL SYMBOL RPAREN)],
        'two expressions token types'
    );
};

subtest 'association list' => sub {
    # alist: '((x . 1) (y . 2))
    is(
        types_of("'((x . 1) (y . 2))"),
        [qw(QUOTE LPAREN
            LPAREN SYMBOL DOT NUMBER RPAREN
            LPAREN SYMBOL DOT NUMBER RPAREN
            RPAREN)],
        'association list token types'
    );
};

subtest 'fibonacci program snippet' => sub {
    # Tokenize the fibonacci function definition.
    # Spot-check that all the right symbols are present.
    my $src = <<'END_LISP';
;; Fibonacci
(define (fib n)
  (if (< n 2)
      n
      (+ (fib (- n 1))
         (fib (- n 2)))))
END_LISP

    my $tokens = CodingAdventures::LispLexer->tokenize($src);

    # Collect symbol values
    my %syms = map { $_->{value} => 1 }
               grep { $_->{type} eq 'SYMBOL' } @$tokens;

    ok( $syms{define}, 'has define' );
    ok( $syms{fib},    'has fib' );
    ok( $syms{'if'},   'has if' );
    ok( $syms{'+'},    'has +' );
    ok( $syms{'-'},    'has -' );
    ok( $syms{'<'},    'has <' );

    # No WHITESPACE or COMMENT tokens
    my @bad = grep { $_->{type} eq 'WHITESPACE' || $_->{type} eq 'COMMENT' }
              @$tokens;
    is( scalar @bad, 0, 'no WHITESPACE or COMMENT tokens in output' );

    # EOF is last
    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('42');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

subtest 'EOF is the only token for empty input' => sub {
    my $tokens = CodingAdventures::LispLexer->tokenize('');
    is( scalar @$tokens,        1,     '1 token' );
    is( $tokens->[-1]{type}, 'EOF', 'that token is EOF' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character @ raises die' => sub {
    ok(
        dies { CodingAdventures::LispLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

subtest 'hash # raises die (not in this grammar)' => sub {
    # Full Scheme uses #t and #f for booleans, but our grammar does not.
    ok(
        dies { CodingAdventures::LispLexer->tokenize('#t') },
        '# causes die'
    );
};

done_testing;
