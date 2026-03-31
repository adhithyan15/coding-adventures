use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::SqlLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::SqlLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::SqlLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub count_type {
    my ($tokens, $type) = @_;
    return scalar grep { $_->{type} eq $type } @$tokens;
}

sub first_of_type {
    my ($tokens, $type) = @_;
    my ($tok) = grep { $_->{type} eq $type } @$tokens;
    return $tok;
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'line comment only produces only EOF' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('-- this is a comment');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'block comment only produces only EOF' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('/* block comment */');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# SELECT queries
# ============================================================================

subtest 'SELECT * FROM users' => sub {
    is(
        types_of('SELECT * FROM users'),
        [qw(SELECT STAR FROM NAME)],
        'basic SELECT token types'
    );
};

subtest 'SELECT value is case-preserved' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT * FROM users');
    is( $tokens->[0]{value}, 'SELECT', 'SELECT value' );
    is( $tokens->[3]{value}, 'users',  'identifier value' );
};

subtest 'select (lowercase) produces SELECT token' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('select * from users');
    is( $tokens->[0]{type}, 'SELECT', 'lowercase select → SELECT type' );
    is( $tokens->[2]{type}, 'FROM',   'lowercase from → FROM type' );
};

subtest 'SELECT * FROM users WHERE id = 1' => sub {
    is(
        types_of('SELECT * FROM users WHERE id = 1'),
        [qw(SELECT STAR FROM NAME WHERE NAME EQUALS NUMBER)],
        'SELECT with WHERE token types'
    );
};

subtest 'SELECT with column list' => sub {
    is(
        types_of('SELECT id, name FROM users'),
        [qw(SELECT NAME COMMA NAME FROM NAME)],
        'SELECT column list token types'
    );
};

subtest 'SELECT with comparison operators' => sub {
    is(
        types_of('SELECT * FROM t WHERE age >= 18 AND age < 65'),
        [qw(SELECT STAR FROM NAME WHERE NAME GREATER_EQUALS NUMBER AND NAME LESS_THAN NUMBER)],
        'SELECT with WHERE and operators'
    );
};

subtest 'SELECT with ORDER BY and LIMIT' => sub {
    is(
        types_of('SELECT * FROM t ORDER BY id DESC LIMIT 10'),
        [qw(SELECT STAR FROM NAME ORDER BY NAME DESC LIMIT NUMBER)],
        'SELECT with ORDER BY and LIMIT'
    );
};

subtest 'SELECT DISTINCT' => sub {
    is(
        types_of('SELECT DISTINCT name FROM users'),
        [qw(SELECT DISTINCT NAME FROM NAME)],
        'SELECT DISTINCT token types'
    );
};

# ============================================================================
# INSERT queries
# ============================================================================

subtest 'INSERT INTO table VALUES' => sub {
    is(
        types_of("INSERT INTO users VALUES (1, 'Alice')"),
        [qw(INSERT INTO NAME VALUES LPAREN NUMBER COMMA STRING RPAREN)],
        'INSERT INTO VALUES token types'
    );
};

subtest 'INSERT with column list' => sub {
    is(
        types_of("INSERT INTO t (id, name) VALUES (1, 'Bob')"),
        [qw(INSERT INTO NAME LPAREN NAME COMMA NAME RPAREN VALUES LPAREN NUMBER COMMA STRING RPAREN)],
        'INSERT with column list token types'
    );
};

# ============================================================================
# UPDATE and DELETE
# ============================================================================

subtest 'UPDATE ... SET ... WHERE' => sub {
    is(
        types_of("UPDATE t SET name = 'Bob' WHERE id = 1"),
        [qw(UPDATE NAME SET NAME EQUALS STRING WHERE NAME EQUALS NUMBER)],
        'UPDATE SET WHERE token types'
    );
};

subtest 'DELETE FROM ... WHERE' => sub {
    is(
        types_of('DELETE FROM t WHERE id = 1'),
        [qw(DELETE FROM NAME WHERE NAME EQUALS NUMBER)],
        'DELETE FROM WHERE token types'
    );
};

# ============================================================================
# String literals
# ============================================================================

subtest 'single-quoted string' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize("'hello'");
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, "'hello'", 'value preserved with quotes' );
};

subtest 'empty single-quoted string' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize("''");
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, "''",     'empty string value' );
};

subtest 'multiple strings separated by comma' => sub {
    is(
        types_of("'a', 'b'"),
        [qw(STRING COMMA STRING)],
        'two strings with comma'
    );
};

# ============================================================================
# Numeric literals
# ============================================================================

subtest 'positive integer' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

subtest 'decimal number' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('3.14');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '3.14',   'value is 3.14' );
};

# ============================================================================
# NULL, TRUE, FALSE literals
# ============================================================================

subtest 'NULL literal' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('NULL');
    is( $tokens->[0]{type},  'NULL', 'type is NULL' );
    is( $tokens->[0]{value}, 'NULL', 'value is NULL' );
};

subtest 'null (lowercase) → NULL token' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('null');
    is( $tokens->[0]{type}, 'NULL', 'lowercase null → NULL type' );
};

subtest 'TRUE literal' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('TRUE');
    is( $tokens->[0]{type}, 'TRUE', 'type is TRUE' );
};

subtest 'FALSE literal' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('FALSE');
    is( $tokens->[0]{type}, 'FALSE', 'type is FALSE' );
};

subtest 'IS NULL expression' => sub {
    is(
        types_of('col IS NULL'),
        [qw(NAME IS NULL)],
        'IS NULL token types'
    );
};

subtest 'IS NOT NULL expression' => sub {
    is(
        types_of('col IS NOT NULL'),
        [qw(NAME IS NOT NULL)],
        'IS NOT NULL token types'
    );
};

# ============================================================================
# Operators
# ============================================================================

subtest 'EQUALS operator' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a = 1');
    is( $tokens->[1]{type}, 'EQUALS', 'type is EQUALS' );
    is( $tokens->[1]{value}, '=',      'value is =' );
};

subtest 'NOT_EQUALS operator !=' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a != 1');
    is( $tokens->[1]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[1]{value}, '!=',         'value is !=' );
};

subtest 'NOT_EQUALS via NEQ_ANSI <>' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a <> 1');
    is( $tokens->[1]{type},  'NOT_EQUALS', 'type is NOT_EQUALS (aliased from NEQ_ANSI)' );
    is( $tokens->[1]{value}, '<>',         'value is <>' );
};

subtest 'LESS_THAN operator' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a < 1');
    is( $tokens->[1]{type}, 'LESS_THAN', 'type is LESS_THAN' );
};

subtest 'GREATER_THAN operator' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a > 1');
    is( $tokens->[1]{type}, 'GREATER_THAN', 'type is GREATER_THAN' );
};

subtest 'LESS_EQUALS operator' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a <= 1');
    is( $tokens->[1]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[1]{value}, '<=',           'value is <=' );
};

subtest 'GREATER_EQUALS operator' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('a >= 1');
    is( $tokens->[1]{type},  'GREATER_EQUALS', 'type is GREATER_EQUALS' );
    is( $tokens->[1]{value}, '>=',              'value is >=' );
};

subtest '<= matched before < (longest match wins)' => sub {
    is(
        types_of('a <= 1'),
        [qw(NAME LESS_EQUALS NUMBER)],
        '<= is a single token, not < and ='
    );
};

subtest 'arithmetic operators' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('1 + 2 - 3 * 4 / 5 % 6');
    my $types = [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
    is(
        $types,
        [qw(NUMBER PLUS NUMBER MINUS NUMBER STAR NUMBER SLASH NUMBER PERCENT NUMBER)],
        'all arithmetic operators'
    );
};

# ============================================================================
# Delimiters
# ============================================================================

subtest 'parentheses' => sub {
    is(
        types_of('(1)'),
        [qw(LPAREN NUMBER RPAREN)],
        'parentheses token types'
    );
};

subtest 'semicolon' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT 1;');
    is( $tokens->[2]{type}, 'SEMICOLON', 'type is SEMICOLON' );
};

subtest 'dot for table.column' => sub {
    is(
        types_of('t.col'),
        [qw(NAME DOT NAME)],
        'table.column dot notation'
    );
};

# ============================================================================
# Comment handling
# ============================================================================

subtest 'line comment after tokens stripped' => sub {
    is(
        types_of('SELECT 1 -- comment'),
        [qw(SELECT NUMBER)],
        'line comment not emitted'
    );
};

subtest 'block comment between tokens stripped' => sub {
    is(
        types_of('SELECT /* comment */ 1'),
        [qw(SELECT NUMBER)],
        'block comment not emitted'
    );
};

subtest 'multi-line block comment stripped' => sub {
    is(
        types_of("SELECT /*\n  big comment\n*/ 1"),
        [qw(SELECT NUMBER)],
        'multi-line block comment not emitted'
    );
};

# ============================================================================
# JOIN clauses
# ============================================================================

subtest 'INNER JOIN ... ON' => sub {
    is(
        types_of('SELECT * FROM a INNER JOIN b ON a.id = b.id'),
        [qw(SELECT STAR FROM NAME INNER JOIN NAME ON NAME DOT NAME EQUALS NAME DOT NAME)],
        'INNER JOIN token types'
    );
};

subtest 'LEFT JOIN' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize(
        'SELECT * FROM a LEFT JOIN b ON a.id = b.id'
    );
    is( $tokens->[4]{type}, 'LEFT', 'LEFT token' );
    is( $tokens->[5]{type}, 'JOIN', 'JOIN token' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens stripped' => sub {
    is(
        types_of('SELECT   *   FROM   t'),
        [qw(SELECT STAR FROM NAME)],
        'no whitespace tokens'
    );
};

subtest 'newlines between tokens stripped' => sub {
    is(
        types_of("SELECT\n*\nFROM\nt"),
        [qw(SELECT STAR FROM NAME)],
        'newlines not emitted'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking on single line' => sub {
    # Input: SELECT 1
    # col:   1234567 89
    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT 1');
    is( $tokens->[0]{col}, 1, 'SELECT at col 1' );
    is( $tokens->[1]{col}, 8, '1 at col 8' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT * FROM t');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# Composite SQL structures
# ============================================================================

subtest 'real-world SELECT with GROUP BY and HAVING' => sub {
    my $src = <<'END_SQL';
SELECT department, COUNT(*) AS cnt
FROM employees
WHERE salary > 50000
GROUP BY department
HAVING cnt > 5
ORDER BY cnt DESC
END_SQL

    my $tokens = CodingAdventures::SqlLexer->tokenize($src);
    ok( scalar(@$tokens) > 20, 'many tokens produced' );

    my $select_tok = first_of_type($tokens, 'SELECT');
    ok( defined $select_tok, 'SELECT token found' );

    my $from_tok = first_of_type($tokens, 'FROM');
    ok( defined $from_tok, 'FROM token found' );

    ok( count_type($tokens, 'NAME')   >= 5, 'at least 5 NAME tokens' );
    ok( count_type($tokens, 'NUMBER') >= 2, 'at least 2 NUMBER tokens' );

    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

subtest 'BETWEEN ... AND' => sub {
    is(
        types_of('age BETWEEN 18 AND 65'),
        [qw(NAME BETWEEN NUMBER AND NUMBER)],
        'BETWEEN AND token types'
    );
};

subtest 'LIKE pattern matching' => sub {
    is(
        types_of("name LIKE '%Alice%'"),
        [qw(NAME LIKE STRING)],
        'LIKE token types'
    );
};

subtest 'IN list' => sub {
    is(
        types_of('id IN (1, 2, 3)'),
        [qw(NAME IN LPAREN NUMBER COMMA NUMBER COMMA NUMBER RPAREN)],
        'IN list token types'
    );
};

subtest 'CREATE TABLE statement' => sub {
    is(
        types_of('CREATE TABLE users (id NUMBER, name STRING)'),
        [qw(CREATE TABLE NAME LPAREN NAME NAME COMMA NAME NAME RPAREN)],
        'CREATE TABLE token types'
    );
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT 1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::SqlLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

done_testing;
