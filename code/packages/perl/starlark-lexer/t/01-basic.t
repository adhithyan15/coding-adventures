use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::StarlarkLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers: collect token types and values (excluding EOF, NEWLINE, INDENT,
# DEDENT) from a source string.  The "content_types" helper filters out
# structural indentation tokens so tests can focus on meaningful token types.
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub content_types_of {
    my ($source) = @_;
    my %skip = ( EOF => 1, NEWLINE => 1, INDENT => 1, DEDENT => 1 );
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($source);
    return [ map { $_->{type} } grep { !$skip{$_->{type}} } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my %skip = ( EOF => 1, NEWLINE => 1, INDENT => 1, DEDENT => 1 );
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($source);
    return [ map { $_->{value} } grep { !$skip{$_->{type}} } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('');
    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
    my @content = grep { $_->{type} !~ /^(?:EOF|NEWLINE|INDENT|DEDENT)$/ } @$tokens;
    is( scalar @content, 0, 'no content tokens' );
};

subtest 'whitespace-only produces no content tokens' => sub {
    is( content_types_of('   '), [], 'only structural tokens' );
};

# ============================================================================
# Keywords
# ============================================================================

subtest 'keyword: if' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('if');
    my ($tok) = grep { $_->{type} eq 'IF' } @$tokens;
    ok( $tok,              'has IF token'    );
    is( $tok->{value}, 'if', 'value is if'  );
};

subtest 'keyword: elif' => sub {
    my ($tok) = grep { $_->{type} eq 'ELIF' }
                @{ CodingAdventures::StarlarkLexer->tokenize('elif') };
    ok( $tok,               'has ELIF token'  );
    is( $tok->{value}, 'elif', 'value is elif' );
};

subtest 'keyword: else' => sub {
    my ($tok) = grep { $_->{type} eq 'ELSE' }
                @{ CodingAdventures::StarlarkLexer->tokenize('else') };
    ok( $tok,               'has ELSE token'  );
    is( $tok->{value}, 'else', 'value is else' );
};

subtest 'keyword: for' => sub {
    is( content_types_of('for'), ['FOR'], 'for keyword type' );
};

subtest 'keyword: def' => sub {
    my ($tok) = grep { $_->{type} eq 'DEF' }
                @{ CodingAdventures::StarlarkLexer->tokenize('def') };
    ok( $tok,               'has DEF token'  );
    is( $tok->{value}, 'def', 'value is def' );
};

subtest 'keyword: return' => sub {
    is( content_types_of('return'), ['RETURN'], 'return keyword type' );
};

subtest 'keyword: pass' => sub {
    is( content_types_of('pass'), ['PASS'], 'pass keyword type' );
};

subtest 'keyword: break' => sub {
    is( content_types_of('break'), ['BREAK'], 'break keyword type' );
};

subtest 'keyword: continue' => sub {
    is( content_types_of('continue'), ['CONTINUE'], 'continue keyword type' );
};

subtest 'keyword: and' => sub {
    is( content_types_of('and'), ['AND'], 'and keyword type' );
};

subtest 'keyword: or' => sub {
    is( content_types_of('or'), ['OR'], 'or keyword type' );
};

subtest 'keyword: not' => sub {
    is( content_types_of('not'), ['NOT'], 'not keyword type' );
};

subtest 'keyword: in' => sub {
    is( content_types_of('in'), ['IN'], 'in keyword type' );
};

subtest 'keyword: lambda' => sub {
    is( content_types_of('lambda'), ['LAMBDA'], 'lambda keyword type' );
};

subtest 'keyword: load' => sub {
    is( content_types_of('load'), ['LOAD'], 'load keyword type' );
};

subtest 'keyword: True' => sub {
    my ($tok) = grep { $_->{type} eq 'TRUE' }
                @{ CodingAdventures::StarlarkLexer->tokenize('True') };
    ok( $tok,                'has TRUE token'   );
    is( $tok->{value}, 'True', 'value is True'  );
};

subtest 'keyword: False' => sub {
    my ($tok) = grep { $_->{type} eq 'FALSE' }
                @{ CodingAdventures::StarlarkLexer->tokenize('False') };
    ok( $tok,                 'has FALSE token'  );
    is( $tok->{value}, 'False', 'value is False' );
};

subtest 'keyword: None' => sub {
    my ($tok) = grep { $_->{type} eq 'NONE' }
                @{ CodingAdventures::StarlarkLexer->tokenize('None') };
    ok( $tok,                'has NONE token'  );
    is( $tok->{value}, 'None', 'value is None' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('my_var');
    my ($tok) = grep { $_->{type} eq 'NAME' } @$tokens;
    ok( $tok,                  'has NAME token'      );
    is( $tok->{value}, 'my_var', 'value is my_var'   );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('_private');
    my ($tok) = grep { $_->{type} eq 'NAME' } @$tokens;
    ok( $tok,                    'has NAME token'       );
    is( $tok->{value}, '_private', 'value is _private' );
};

subtest 'identifier with digits' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('abc123');
    my ($tok) = grep { $_->{type} eq 'NAME' } @$tokens;
    ok( $tok,                  'has NAME token'    );
    is( $tok->{value}, 'abc123', 'value is abc123' );
};

subtest 'define is not a keyword (starts with def but isnt def)' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('define');
    my ($tok) = grep { $_->{type} eq 'NAME' } @$tokens;
    ok( $tok,                  'has NAME token'    );
    is( $tok->{value}, 'define', 'value is define' );
};

# ============================================================================
# Integer tokens
# ============================================================================

subtest 'decimal integer' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('42');
    my ($tok) = grep { $_->{type} eq 'INT' } @$tokens;
    ok( $tok,               'has INT token'  );
    is( $tok->{value}, '42', 'value is 42'  );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('0');
    my ($tok) = grep { $_->{type} eq 'INT' } @$tokens;
    ok( $tok,              'has INT token' );
    is( $tok->{value}, '0', 'value is 0'  );
};

subtest 'hex integer' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('0xFF');
    my ($tok) = grep { $_->{type} eq 'INT' } @$tokens;
    ok( $tok,                'has INT token'  );
    is( $tok->{value}, '0xFF', 'value is 0xFF' );
};

subtest 'octal integer' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('0o77');
    my ($tok) = grep { $_->{type} eq 'INT' } @$tokens;
    ok( $tok,                'has INT token'  );
    is( $tok->{value}, '0o77', 'value is 0o77' );
};

# ============================================================================
# Float tokens
# ============================================================================

subtest 'simple float' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('3.14');
    my ($tok) = grep { $_->{type} eq 'FLOAT' } @$tokens;
    ok( $tok,                 'has FLOAT token'  );
    is( $tok->{value}, '3.14', 'value is 3.14'  );
};

subtest 'float with exponent' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('1e10');
    my ($tok) = grep { $_->{type} eq 'FLOAT' } @$tokens;
    ok( $tok, 'has FLOAT token' );
};

subtest 'float starting with dot' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('.5');
    my ($tok) = grep { $_->{type} eq 'FLOAT' } @$tokens;
    ok( $tok,               'has FLOAT token' );
    is( $tok->{value}, '.5', 'value is .5'   );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('"hello"');
    my ($tok) = grep { $_->{type} eq 'STRING' } @$tokens;
    ok( $tok,                    'has STRING token'   );
    is( $tok->{value}, '"hello"', 'value preserved'  );
};

subtest 'single-quoted string' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize("'hello'");
    my ($tok) = grep { $_->{type} eq 'STRING' } @$tokens;
    ok( $tok,                    'has STRING token' );
    is( $tok->{value}, "'hello'", 'value preserved' );
};

subtest 'raw string' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('r"raw"');
    my ($tok) = grep { $_->{type} eq 'STRING' } @$tokens;
    ok( $tok, 'has STRING token' );
};

subtest 'bytes string' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('b"bytes"');
    my ($tok) = grep { $_->{type} eq 'STRING' } @$tokens;
    ok( $tok, 'has STRING token' );
};

# ============================================================================
# Operators
# ============================================================================

subtest 'three-char operator: **=' => sub {
    my ($tok) = grep { $_->{type} eq 'DOUBLE_STAR_EQUALS' }
                @{ CodingAdventures::StarlarkLexer->tokenize('**=') };
    ok( $tok,                 'has DOUBLE_STAR_EQUALS' );
    is( $tok->{value}, '**=', 'value is **='           );
};

subtest 'three-char operator: <<=' => sub {
    my ($tok) = grep { $_->{type} eq 'LEFT_SHIFT_EQUALS' }
                @{ CodingAdventures::StarlarkLexer->tokenize('<<=') };
    ok( $tok, 'has LEFT_SHIFT_EQUALS' );
};

subtest 'two-char operator: ==' => sub {
    is( content_types_of('=='), ['EQUALS_EQUALS'], '== produces EQUALS_EQUALS' );
};

subtest 'two-char operator: !=' => sub {
    is( content_types_of('!='), ['NOT_EQUALS'], '!= produces NOT_EQUALS' );
};

subtest 'two-char operator: **' => sub {
    is( content_types_of('**'), ['DOUBLE_STAR'], '** produces DOUBLE_STAR' );
};

subtest 'two-char operator: //' => sub {
    is( content_types_of('//'), ['FLOOR_DIV'], '// produces FLOOR_DIV' );
};

subtest 'two-char operator: +=' => sub {
    is( content_types_of('+='), ['PLUS_EQUALS'], '+= produces PLUS_EQUALS' );
};

subtest 'single-char operator: +' => sub {
    is( content_types_of('+'), ['PLUS'], '+ produces PLUS' );
};

subtest 'single-char operator: -' => sub {
    is( content_types_of('-'), ['MINUS'], '- produces MINUS' );
};

subtest 'single-char operator: *' => sub {
    is( content_types_of('*'), ['STAR'], '* produces STAR' );
};

subtest 'single-char operator: /' => sub {
    is( content_types_of('/'), ['SLASH'], '/ produces SLASH' );
};

subtest 'single-char operator: =' => sub {
    is( content_types_of('='), ['EQUALS'], '= produces EQUALS' );
};

subtest '== before = (first-match-wins)' => sub {
    is( content_types_of('=='), ['EQUALS_EQUALS'], '== is one token not two EQUALS' );
};

# ============================================================================
# Delimiter tokens
# ============================================================================

subtest 'parentheses ()' => sub {
    is( content_types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'brackets []' => sub {
    is( content_types_of('[]'), [qw(LBRACKET RBRACKET)], 'bracket types' );
};

subtest 'braces {}' => sub {
    is( content_types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'comma' => sub {
    is( content_types_of(','), ['COMMA'], 'comma type' );
};

subtest 'colon' => sub {
    is( content_types_of(':'), ['COLON'], 'colon type' );
};

subtest 'dot' => sub {
    is( content_types_of('.'), ['DOT'], 'dot type' );
};

# ============================================================================
# Indentation tokens
# ============================================================================

subtest 'NEWLINE emitted at end of logical line' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize("x = 1\n");
    my ($nl) = grep { $_->{type} eq 'NEWLINE' } @$tokens;
    ok( $nl, 'has NEWLINE token' );
};

subtest 'INDENT and DEDENT emitted for indented block' => sub {
    my $src = "def f():\n    pass\n";
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($src);
    my ($indent) = grep { $_->{type} eq 'INDENT' } @$tokens;
    my ($dedent) = grep { $_->{type} eq 'DEDENT' } @$tokens;
    ok( $indent, 'has INDENT token' );
    ok( $dedent, 'has DEDENT token' );
};

subtest 'no INDENT inside parens (implicit continuation)' => sub {
    my $src = "foo(\n    x,\n    y\n)";
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($src);
    my ($indent) = grep { $_->{type} eq 'INDENT' } @$tokens;
    ok( !$indent, 'no INDENT inside parens' );
};

# ============================================================================
# Comment handling
# ============================================================================

subtest '# comment is consumed silently' => sub {
    is( content_types_of('x # this is a comment'), ['NAME'], 'only NAME token' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'simple assignment: x = 1' => sub {
    is(
        content_types_of('x = 1'),
        [qw(NAME EQUALS INT)],
        'assignment types'
    );
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('x = 1');
    my ($name) = grep { $_->{type} eq 'NAME' } @$tokens;
    is( $name->{value}, 'x', 'identifier value is x' );
};

subtest 'equality check: x == 1' => sub {
    is(
        content_types_of('x == 1'),
        [qw(NAME EQUALS_EQUALS INT)],
        'equality types'
    );
};

subtest 'function definition: def foo(x):' => sub {
    is(
        content_types_of('def foo(x):'),
        [qw(DEF NAME LPAREN NAME RPAREN COLON)],
        'def header types'
    );
};

subtest 'list literal: [1, 2, 3]' => sub {
    is(
        content_types_of('[1, 2, 3]'),
        [qw(LBRACKET INT COMMA INT COMMA INT RBRACKET)],
        'list types'
    );
};

subtest 'for x in y:' => sub {
    is(
        content_types_of('for x in y:'),
        [qw(FOR NAME IN NAME COLON)],
        'for in types'
    );
};

subtest 'boolean expression: a and b or not c' => sub {
    is(
        content_types_of('a and b or not c'),
        [qw(NAME AND NAME OR NOT NAME)],
        'boolean expression types'
    );
};

subtest 'power: x ** 2' => sub {
    is(
        content_types_of('x ** 2'),
        [qw(NAME DOUBLE_STAR INT)],
        'power expression types'
    );
};

subtest 'augmented assignment: x += 1' => sub {
    is(
        content_types_of('x += 1'),
        [qw(NAME PLUS_EQUALS INT)],
        'augmented assignment types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens consumed silently' => sub {
    is( content_types_of('x = 1'), [qw(NAME EQUALS INT)], 'no whitespace tokens' );
};

subtest 'tabs between tokens consumed silently' => sub {
    is( content_types_of("x\t=\t1"), [qw(NAME EQUALS INT)], 'tabs consumed' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: x = 42' => sub {
    # x _ = _ 4 2
    # 1 2 3 4 5 6
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('x = 42');
    my ($name) = grep { $_->{type} eq 'NAME'   } @$tokens;
    my ($eq)   = grep { $_->{type} eq 'EQUALS' } @$tokens;
    my ($int)  = grep { $_->{type} eq 'INT'    } @$tokens;
    is( $name->{col}, 1, 'x at col 1' );
    is( $eq->{col},   3, '= at col 3' );
    is( $int->{col},  5, '42 at col 5' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('x = 1');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::StarlarkLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::StarlarkLexer->tokenize('`') },
        'backtick causes die'
    );
};

done_testing;
