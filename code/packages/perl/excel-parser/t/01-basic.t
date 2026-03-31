use strict;
use warnings;
use Test2::V0;

# ============================================================================
# CodingAdventures::ExcelParser — basic parsing tests
# ============================================================================
#
# Tests cover:
#   - Module loads
#   - Formula root node (with and without leading =)
#   - Literal values: number, string, bool, error
#   - Cell references: A1, $B$2
#   - Range references: A1:B10
#   - Cross-sheet references: Sheet1!A1
#   - Arithmetic operators and precedence (* over +, parens override)
#   - Unary prefix: -A1, --A1
#   - Postfix %: 50%, A1*100%
#   - Comparison operators: =, <>, <=, >=, <, >
#   - Concatenation: &
#   - Function calls: SUM(), IF(), COUNT(), nested, IFERROR
#   - Array constants: {1,2,3}, {1,2;3,4}, negative numbers, strings
#   - Error on trailing content
#   - Error on unclosed parenthesis
#   - Error on empty formula (= with nothing after)

ok( eval { require CodingAdventures::ExcelParser; 1 }, 'module loads' );

# ============================================================================
# Helper: parse and return root, or re-die with source info
# ============================================================================

sub parse_ok {
    my ($source) = @_;
    my $ast;
    eval { $ast = CodingAdventures::ExcelParser->parse($source) };
    if ($@) {
        die "parse_ok failed for '$source': $@";
    }
    return $ast;
}

sub parse_dies {
    my ($source, $label) = @_;
    $label //= $source;
    ok( dies { CodingAdventures::ExcelParser->parse($source) },
        "parse dies for: $label" );
}

# ============================================================================
# Module surface
# ============================================================================

subtest 'VERSION is defined' => sub {
    ok( CodingAdventures::ExcelParser->VERSION, 'VERSION is set' );
};

# ============================================================================
# Formula root node
# ============================================================================

subtest 'root node has rule_name formula' => sub {
    my $ast = parse_ok('=1');
    is( $ast->rule_name, 'formula', 'rule_name is formula' );
};

subtest 'eq field captured for leading =' => sub {
    my $ast = parse_ok('=1');
    ok( defined $ast->{eq}, 'eq field is defined' );
    is( $ast->{eq}{type}, 'EQUALS', 'eq type is EQUALS' );
};

subtest 'formula without leading = has eq => undef' => sub {
    my $ast = parse_ok('1+2');
    ok( !defined $ast->{eq}, 'eq is undef when no leading =' );
    is( $ast->{body}->rule_name, 'binop', 'body is binop' );
};

# ============================================================================
# Literal values
# ============================================================================

subtest 'integer number 42' => sub {
    my $ast = parse_ok('=42');
    is( $ast->{body}->rule_name, 'number', 'rule_name is number' );
    is( $ast->{body}{token}{value}, '42', 'value is 42' );
};

subtest 'float 3.14' => sub {
    my $ast = parse_ok('=3.14');
    is( $ast->{body}->rule_name, 'number', 'rule_name is number' );
};

subtest 'decimal fraction .5' => sub {
    my $ast = parse_ok('=.5');
    is( $ast->{body}->rule_name, 'number', 'rule_name is number' );
};

subtest 'double-quoted string "hello"' => sub {
    my $ast = parse_ok('="hello"');
    is( $ast->{body}->rule_name, 'string', 'rule_name is string' );
    is( $ast->{body}{token}{value}, '"hello"', 'value is "hello"' );
};

subtest 'boolean TRUE' => sub {
    my $ast = parse_ok('=TRUE');
    is( $ast->{body}->rule_name, 'bool', 'rule_name is bool' );
    is( $ast->{body}{token}{value}, 'true', 'value is lowercased true' );
};

subtest 'boolean FALSE' => sub {
    my $ast = parse_ok('=FALSE');
    is( $ast->{body}->rule_name, 'bool', 'rule_name is bool' );
};

subtest 'error constant #DIV/0!' => sub {
    my $ast = parse_ok('=#DIV/0!');
    is( $ast->{body}->rule_name, 'error', 'rule_name is error' );
};

subtest 'error constant #VALUE!' => sub {
    my $ast = parse_ok('=#VALUE!');
    is( $ast->{body}->rule_name, 'error', 'rule_name is error' );
};

# ============================================================================
# Cell and range references
# ============================================================================

subtest 'cell reference A1' => sub {
    my $ast = parse_ok('=A1');
    is( $ast->{body}->rule_name, 'cell', 'rule_name is cell' );
    is( $ast->{body}{token}{value}, 'a1', 'value is lowercased a1' );
};

subtest 'absolute cell reference $B$2' => sub {
    my $ast = parse_ok('=$B$2');
    is( $ast->{body}->rule_name, 'cell', 'rule_name is cell' );
};

subtest 'range A1:B10' => sub {
    my $ast = parse_ok('=A1:B10');
    is( $ast->{body}->rule_name, 'range', 'rule_name is range' );
    is( $ast->{body}{start_ref}->rule_name, 'cell', 'start_ref is cell' );
    is( $ast->{body}{end_ref}->rule_name,   'cell', 'end_ref is cell' );
};

subtest 'cross-sheet reference Sheet1!A1' => sub {
    my $ast = parse_ok('=Sheet1!A1');
    is( $ast->{body}->rule_name, 'ref_prefix', 'rule_name is ref_prefix' );
    is( $ast->{body}{prefix}{type}, 'REF_PREFIX', 'prefix type is REF_PREFIX' );
    is( $ast->{body}{ref}->rule_name, 'cell', 'ref is cell' );
};

# ============================================================================
# Arithmetic operators and precedence
# ============================================================================

subtest 'addition A1+B2' => sub {
    my $ast = parse_ok('=A1+B2');
    is( $ast->{body}->rule_name, 'binop', 'binop' );
    is( $ast->{body}{op}{type}, 'PLUS', 'op is PLUS' );
};

subtest 'subtraction A1-B2' => sub {
    my $ast = parse_ok('=A1-B2');
    is( $ast->{body}{op}{type}, 'MINUS', 'op is MINUS' );
};

subtest 'multiplication A1*B2' => sub {
    my $ast = parse_ok('=A1*B2');
    is( $ast->{body}{op}{type}, 'STAR', 'op is STAR' );
};

subtest 'division A1/B2' => sub {
    my $ast = parse_ok('=A1/B2');
    is( $ast->{body}{op}{type}, 'SLASH', 'op is SLASH' );
};

subtest 'exponentiation A1^2' => sub {
    my $ast = parse_ok('=A1^2');
    is( $ast->{body}{op}{type}, 'CARET', 'op is CARET' );
};

subtest '* binds tighter than + in 1+2*3' => sub {
    my $ast = parse_ok('=1+2*3');
    # Root should be PLUS; right operand should be STAR
    is( $ast->{body}{op}{type},           'PLUS', 'root op is PLUS' );
    is( $ast->{body}{right}->rule_name,   'binop', 'right is binop' );
    is( $ast->{body}{right}{op}{type},    'STAR',  'right op is STAR' );
};

subtest 'parentheses override precedence: (1+2)*3' => sub {
    my $ast = parse_ok('=(1+2)*3');
    is( $ast->{body}{op}{type},          'STAR',  'root op is STAR' );
    is( $ast->{body}{left}->rule_name,   'group', 'left is group' );
};

# ============================================================================
# Unary and postfix operators
# ============================================================================

subtest 'unary minus -A1' => sub {
    my $ast = parse_ok('=-A1');
    is( $ast->{body}->rule_name, 'unop',   'rule_name is unop' );
    is( $ast->{body}{op}{type},  'MINUS',  'op is MINUS' );
    is( $ast->{body}{operand}->rule_name, 'cell', 'operand is cell' );
};

subtest 'double negation --A1' => sub {
    my $ast = parse_ok('=--A1');
    is( $ast->{body}->rule_name,           'unop', 'outer unop' );
    is( $ast->{body}{operand}->rule_name,  'unop', 'inner unop' );
};

subtest 'postfix percent 50%' => sub {
    my $ast = parse_ok('=50%');
    is( $ast->{body}->rule_name,           'postfix', 'rule_name is postfix' );
    is( $ast->{body}{op}{type},            'PERCENT',  'op is PERCENT' );
    is( $ast->{body}{operand}->rule_name,  'number',   'operand is number' );
};

subtest 'A1*100% percent binds tighter than *' => sub {
    my $ast = parse_ok('=A1*100%');
    is( $ast->{body}{op}{type},           'STAR',    'root is STAR' );
    is( $ast->{body}{right}->rule_name,   'postfix', 'right is postfix' );
};

# ============================================================================
# Comparison and concatenation
# ============================================================================

subtest 'equality A1=B1' => sub {
    my $ast = parse_ok('=A1=B1');
    is( $ast->{body}{op}{type}, 'EQUALS', 'op is EQUALS' );
};

subtest 'not-equal A1<>B1' => sub {
    my $ast = parse_ok('=A1<>B1');
    is( $ast->{body}{op}{type}, 'NOT_EQUALS', 'op is NOT_EQUALS' );
};

subtest 'greater-equal A1>=0' => sub {
    my $ast = parse_ok('=A1>=0');
    is( $ast->{body}{op}{type}, 'GREATER_EQUALS', 'op is GREATER_EQUALS' );
};

subtest 'less-than A1<0' => sub {
    my $ast = parse_ok('=A1<0');
    is( $ast->{body}{op}{type}, 'LESS_THAN', 'op is LESS_THAN' );
};

subtest 'concatenation A1&" world"' => sub {
    my $ast = parse_ok('=A1&" world"');
    is( $ast->{body}{op}{type}, 'AMP', 'op is AMP' );
};

# ============================================================================
# Function calls
# ============================================================================

subtest 'SUM(A1:B10) — single range argument' => sub {
    my $ast = parse_ok('=SUM(A1:B10)');
    is( $ast->{body}->rule_name,       'call',  'rule_name is call' );
    is( $ast->{body}{name}{value},     'sum',   'name is sum (lowercased)' );
    is( scalar @{ $ast->{body}{args} }, 1,      'one argument' );
    is( $ast->{body}{args}[0]->rule_name, 'range', 'argument is range' );
};

subtest 'IF(A1>0,"pos","neg") — three arguments' => sub {
    my $ast = parse_ok('=IF(A1>0,"pos","neg")');
    is( $ast->{body}->rule_name,         'call',  'rule_name is call' );
    is( $ast->{body}{name}{value},       'if',    'name is if' );
    is( scalar @{ $ast->{body}{args} },  3,       'three arguments' );
    is( $ast->{body}{args}[0]->rule_name, 'binop', 'first arg is binop (A1>0)' );
    is( $ast->{body}{args}[1]->rule_name, 'string', 'second arg is string' );
    is( $ast->{body}{args}[2]->rule_name, 'string', 'third arg is string' );
};

subtest 'COUNT() — zero arguments' => sub {
    my $ast = parse_ok('=COUNT()');
    is( $ast->{body}->rule_name,         'call', 'rule_name is call' );
    is( scalar @{ $ast->{body}{args} },  0,      'zero arguments' );
};

subtest 'ABS(SUM(A1:A10)) — nested function calls' => sub {
    my $ast = parse_ok('=ABS(SUM(A1:A10))');
    is( $ast->{body}->rule_name,               'call', 'outer is call' );
    is( $ast->{body}{name}{value},             'abs',  'outer is ABS' );
    is( $ast->{body}{args}[0]->rule_name,      'call', 'inner is call' );
    is( $ast->{body}{args}[0]{name}{value},    'sum',  'inner is SUM' );
};

subtest 'IFERROR(A1/B1,#DIV/0!) — error constant argument' => sub {
    my $ast = parse_ok('=IFERROR(A1/B1,#DIV/0!)');
    is( $ast->{body}->rule_name,           'call', 'rule_name is call' );
    is( scalar @{ $ast->{body}{args} },    2,      'two arguments' );
    is( $ast->{body}{args}[1]->rule_name,  'error', 'second arg is error' );
};

subtest 'VLOOKUP(A2,B:C,2,FALSE) — 4 arguments including bool' => sub {
    my $ast = parse_ok('=VLOOKUP(A2,B:C,2,FALSE)');
    is( $ast->{body}->rule_name,           'call',   'rule_name is call' );
    is( scalar @{ $ast->{body}{args} },    4,        '4 arguments' );
    is( $ast->{body}{args}[3]->rule_name,  'bool',   'last arg is bool' );
};

# ============================================================================
# Array constants
# ============================================================================

subtest '1-D array {1,2,3}' => sub {
    my $ast = parse_ok('={1,2,3}');
    is( $ast->{body}->rule_name,          'array', 'rule_name is array' );
    is( scalar @{ $ast->{body}{rows} },   1,       '1 row' );
    is( scalar @{ $ast->{body}{rows}[0] }, 3,      '3 items in row' );
};

subtest '2-D array {1,2;3,4}' => sub {
    my $ast = parse_ok('={1,2;3,4}');
    is( $ast->{body}->rule_name,          'array', 'rule_name is array' );
    is( scalar @{ $ast->{body}{rows} },   2,       '2 rows' );
    is( scalar @{ $ast->{body}{rows}[0] }, 2,      '2 items in row 1' );
    is( scalar @{ $ast->{body}{rows}[1] }, 2,      '2 items in row 2' );
};

subtest 'array with strings {"a","b"}' => sub {
    my $ast = parse_ok('={"a","b"}');
    is( $ast->{body}{rows}[0][0]->rule_name, 'string', 'first item is string' );
};

subtest 'array with negative number {-1,2}' => sub {
    my $ast = parse_ok('={-1,2}');
    is( $ast->{body}{rows}[0][0]->rule_name, 'unop', 'first item is unop' );
    is( $ast->{body}{rows}[0][0]{op}{type}, 'MINUS', 'sign is MINUS' );
};

# ============================================================================
# Complex / real-world formulas
# ============================================================================

subtest '=A1+Sheet1!B2*0.1' => sub {
    my $ast = parse_ok('=A1+Sheet1!B2*0.1');
    is( $ast->{body}->rule_name, 'binop', 'root is binop' );
    is( $ast->{body}{op}{type},  'PLUS',  'root op is PLUS' );
};

subtest '=A1^2+B1^2 (Pythagorean sum of squares)' => sub {
    my $ast = parse_ok('=A1^2+B1^2');
    is( $ast->{body}{op}{type},        'PLUS',  'root is PLUS' );
    is( $ast->{body}{left}{op}{type},  'CARET', 'left is CARET' );
    is( $ast->{body}{right}{op}{type}, 'CARET', 'right is CARET' );
};

subtest '=SUMIF(A1:A10,">0",B1:B10)' => sub {
    my $ast = parse_ok('=SUMIF(A1:A10,">0",B1:B10)');
    is( $ast->{body}->rule_name,          'call', 'rule_name is call' );
    is( scalar @{ $ast->{body}{args} },   3,      '3 arguments' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'trailing content raises die' => sub {
    parse_dies('=1 2', 'trailing 2 after 1');
};

subtest 'unclosed parenthesis raises die' => sub {
    parse_dies('=SUM(A1', 'unclosed paren');
};

subtest 'empty formula (= only) raises die' => sub {
    parse_dies('=', 'equals with no expression');
};

done_testing;
