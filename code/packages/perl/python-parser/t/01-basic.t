use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::PythonParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::PythonParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse Python and return the root AST node.
sub parse_py {
    my ($src) = @_;
    return CodingAdventures::PythonParser->parse_python($src);
}

# Find the first node with the given rule_name (depth-first).
sub find_node {
    my ($node, $rule_name) = @_;
    return undef unless ref($node);
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if defined $found;
    }
    return undef;
}

# Count all nodes with the given rule_name (full traversal).
sub count_nodes {
    my ($node, $rule_name) = @_;
    return 0 unless ref($node);
    my $n = ($node->rule_name eq $rule_name) ? 1 : 0;
    for my $child (@{ $node->children }) {
        $n += count_nodes($child, $rule_name);
    }
    return $n;
}

# ============================================================================
# ASTNode unit tests
# ============================================================================

subtest 'ASTNode inner node' => sub {
    my $node = CodingAdventures::PythonParser::ASTNode->new('assignment', []);
    is( $node->rule_name, 'assignment', 'rule_name' );
    is( $node->is_leaf,   0,            'not a leaf' );
    is( ref($node->children), 'ARRAY',  'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NAME', value => 'x', line => 1, col => 1 };
    my $leaf = CodingAdventures::PythonParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'NAME',  'token type is NAME' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_py("x = 5");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_py("x = 5");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_py("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'x = 5' => sub {
    my $ast = parse_py("x = 5");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'assignment'), 'assignment node' );
};

subtest 'string assignment' => sub {
    my $ast = parse_py('name = "Alice"');
    ok( defined find_node($ast, 'assignment'), 'assignment for string' );
};

subtest 'assignment contains expression' => sub {
    my $ast = parse_py("x = 42");
    ok( defined find_node($ast, 'expression'), 'expression inside assignment' );
};

subtest 'multiple assignments' => sub {
    my $ast = parse_py("x = 1\ny = 2");
    my $count = count_nodes($ast, 'assignment');
    is( $count, 2, '2 assignment nodes' );
};

# ============================================================================
# Function definitions
# ============================================================================

subtest 'def add(a, b): return a + b' => sub {
    my $src = "def add(a, b):\n    return a + b\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'function_def'), 'function_def' );
    ok( defined find_node($ast, 'param_list'),   'param_list' );
    ok( defined find_node($ast, 'block'),        'block' );
    ok( defined find_node($ast, 'return_stmt'),  'return_stmt' );
};

subtest 'function with no parameters' => sub {
    my $ast = parse_py("def noop():\n    x = 1\n");
    ok( defined find_node($ast, 'function_def'), 'function_def' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if x == 0: return x' => sub {
    my $src = "if x == 0:\n    return x\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    ok( defined find_node($ast, 'block'),   'block' );
};

subtest 'if/else' => sub {
    my $src = "if x == 0:\n    return x\nelse:\n    return 0\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    my $block_count = count_nodes($ast, 'block');
    ok( $block_count >= 2, "at least 2 blocks (got $block_count)" );
};

subtest 'if/elif/else' => sub {
    my $src = "if x == 1:\n    return 1\nelif x == 2:\n    return 2\nelse:\n    return 0\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt with elif' );
    my $block_count = count_nodes($ast, 'block');
    ok( $block_count >= 3, "at least 3 blocks for if/elif/else (got $block_count)" );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for i in range(10): pass-like' => sub {
    my $src = "for i in range(10):\n    x = 1\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'for_stmt'),  'for_stmt' );
    ok( defined find_node($ast, 'call_expr'), 'call_expr for range(10)' );
    ok( defined find_node($ast, 'block'),     'block' );
};

# ============================================================================
# While loops
# ============================================================================

subtest 'while x == 0: x = x + 1' => sub {
    my $src = "while x == 0:\n    x = x + 1\n";
    my $ast = parse_py($src);
    ok( defined find_node($ast, 'while_stmt'), 'while_stmt' );
    ok( defined find_node($ast, 'block'),      'block' );
};

# ============================================================================
# Return statements
# ============================================================================

subtest 'return value' => sub {
    my $ast = parse_py("def f():\n    return 42\n");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'return with expression' => sub {
    my $ast = parse_py("def f():\n    return 1 + 2\n");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt with expr' );
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr inside return' );
};

# ============================================================================
# Import statements
# ============================================================================

subtest 'import math' => sub {
    my $ast = parse_py("import math");
    ok( defined find_node($ast, 'import_stmt'), 'import_stmt' );
};

subtest 'from math import sqrt' => sub {
    my $ast = parse_py("from math import sqrt");
    ok( defined find_node($ast, 'from_import_stmt'), 'from_import_stmt' );
};

# ============================================================================
# Function calls
# ============================================================================

subtest 'print("hello")' => sub {
    my $ast = parse_py('print("hello")');
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'call with no args' => sub {
    my $ast = parse_py("noop()");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter' => sub {
    my $ast = parse_py("r = 1 + 2 * 3");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes for 1+2*3 (got $count)" );
};

subtest 'equality expression ==' => sub {
    my $ast = parse_py("r = a == b");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for ==' );
};

subtest 'unary negation' => sub {
    my $ast = parse_py("r = -x");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_py("r = (2 + 3)");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'function with if and return' => sub {
    my $src = <<'END_PY';
def abs_val(x):
    if x == 0:
        return 0
    return x
END_PY
    my $ast = parse_py($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'function_def'), 'function_def' );
    ok( defined find_node($ast, 'if_stmt'),      'if_stmt' );
    is( count_nodes($ast, 'return_stmt'), 2, '2 return statements' );
};

subtest 'import then assignment' => sub {
    my $ast = parse_py("import math\nx = 5");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'import_stmt'), 'import_stmt' );
    ok( defined find_node($ast, 'assignment'),  'assignment' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::PythonParser->parse_python('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

done_testing;
