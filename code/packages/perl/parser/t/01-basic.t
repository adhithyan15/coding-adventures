use strict;
use warnings;
use Test2::V0;
use FindBin;
use lib "$FindBin::Bin/../../lexer/lib";

ok(eval { require CodingAdventures::Lexer;  1 }, 'Lexer loads');
ok(eval { require CodingAdventures::Parser; 1 }, 'Parser loads');

# Helper: tokenize source and parse it
sub _parse {
    my ($src) = @_;
    my @tokens = CodingAdventures::Lexer->new($src)->tokenize();
    my $parser = CodingAdventures::Parser->new(\@tokens);
    return $parser->parse();
}

# Helper: parse and return the first statement's AST
sub _parse_expr {
    my ($src) = @_;
    my $ast = _parse($src);
    return $ast->{stmts}[0];
}

# ============================================================================
# Module structure
# ============================================================================

subtest 'Parser is a class' => sub {
    my @tokens = CodingAdventures::Lexer->new('42')->tokenize();
    my $parser = CodingAdventures::Parser->new(\@tokens);
    ok($parser->isa('CodingAdventures::Parser'), 'isa Parser');
};

# ============================================================================
# Number literals
# ============================================================================

subtest 'number literal' => sub {
    my $node = _parse_expr('42');
    is($node->{type},  'number', 'type is number');
    is($node->{value}, 42,       'value is 42');
};

subtest 'float literal' => sub {
    my $node = _parse_expr('3.14');
    is($node->{type}, 'number', 'type is number');
    ok($node->{value} > 3.13 && $node->{value} < 3.15, 'value ~3.14');
};

# ============================================================================
# String literals
# ============================================================================

subtest 'string literal' => sub {
    my $node = _parse_expr('"hello"');
    is($node->{type},  'string', 'type is string');
    is($node->{value}, 'hello',  'value is hello (no quotes)');
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'identifier' => sub {
    my $node = _parse_expr('myVar');
    is($node->{type}, 'ident', 'type is ident');
    is($node->{name}, 'myVar', 'name is myVar');
};

# ============================================================================
# Boolean / nil keywords
# ============================================================================

subtest 'true literal' => sub {
    my $node = _parse_expr('true');
    is($node->{type},  'bool', 'type is bool');
    is($node->{value}, 1,      'value is 1');
};

subtest 'false literal' => sub {
    my $node = _parse_expr('false');
    is($node->{type},  'bool', 'type is bool');
    is($node->{value}, 0,      'value is 0');
};

subtest 'nil literal' => sub {
    my $node = _parse_expr('nil');
    is($node->{type}, 'nil', 'type is nil');
};

# ============================================================================
# Binary operators
# ============================================================================

subtest 'addition' => sub {
    my $node = _parse_expr('1 + 2');
    is($node->{type},        'binop',  'type is binop');
    is($node->{op},          '+',      'op is +');
    is($node->{left}{type},  'number', 'left is number');
    is($node->{right}{type}, 'number', 'right is number');
};

subtest 'multiplication' => sub {
    my $node = _parse_expr('3 * 4');
    is($node->{op}, '*', 'op is *');
};

subtest 'operator precedence: 1+2*3' => sub {
    my $node = _parse_expr('1 + 2 * 3');
    # Should be 1 + (2 * 3)
    is($node->{op},          '+', 'outer op is +');
    is($node->{right}{op},   '*', 'inner op is *');
    is($node->{left}{value}, 1,   'left leaf is 1');
};

subtest 'subtraction' => sub {
    my $node = _parse_expr('10 - 3');
    is($node->{op}, '-', 'op is -');
};

subtest 'division' => sub {
    my $node = _parse_expr('8 / 2');
    is($node->{op}, '/', 'op is /');
};

# ============================================================================
# Unary operators
# ============================================================================

subtest 'unary minus' => sub {
    my $node = _parse_expr('-5');
    is($node->{type},        'unary', 'type is unary');
    is($node->{op},          '-',     'op is -');
    is($node->{expr}{value}, 5,       'operand is 5');
};

subtest 'unary not' => sub {
    my $node = _parse_expr('!x');
    is($node->{type}, 'unary', 'type is unary');
    is($node->{op},   '!',     'op is !');
};

# ============================================================================
# Function calls
# ============================================================================

subtest 'function call no args' => sub {
    my $node = _parse_expr('foo()');
    is($node->{type}, 'call', 'type is call');
    is($node->{name}, 'foo',  'name is foo');
    is(scalar @{ $node->{args} }, 0, 'no args');
};

subtest 'function call with args' => sub {
    my $node = _parse_expr('add(1, 2)');
    is($node->{type}, 'call', 'type is call');
    is($node->{name}, 'add',  'name is add');
    is(scalar @{ $node->{args} }, 2, '2 args');
    is($node->{args}[0]{value}, 1,   'first arg is 1');
    is($node->{args}[1]{value}, 2,   'second arg is 2');
};

# ============================================================================
# If expression
# ============================================================================

subtest 'if expression' => sub {
    my $node = _parse_expr('if x then 1 else 2');
    is($node->{type},        'if', 'type is if');
    is($node->{cond}{name},  'x',  'cond is x');
    is($node->{then}{value}, 1,    'then is 1');
    is($node->{else}{value}, 2,    'else is 2');
};

# ============================================================================
# Let binding
# ============================================================================

subtest 'let binding' => sub {
    my $ast  = _parse('let x = 42');
    my $node = $ast->{stmts}[0];
    is($node->{type},         'let', 'type is let');
    is($node->{name},         'x',   'name is x');
    is($node->{value}{value}, 42,    'value is 42');
};

# ============================================================================
# Program with multiple statements
# ============================================================================

subtest 'program with multiple statements' => sub {
    my $ast = _parse("let x = 1\nlet y = 2");
    is($ast->{type}, 'program', 'root is program');
    is(scalar @{ $ast->{stmts} }, 2, 'two statements');
};

# ============================================================================
# Parenthesized expressions
# ============================================================================

subtest 'parenthesized expression' => sub {
    my $node = _parse_expr('(1 + 2) * 3');
    is($node->{op},          '*', 'outer is *');
    is($node->{left}{op},    '+', 'inner left is +');
};

done_testing;
