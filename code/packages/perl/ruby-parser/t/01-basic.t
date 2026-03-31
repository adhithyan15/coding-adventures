use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::RubyParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::RubyParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse Ruby and return the root AST node.
sub parse_rb {
    my ($src) = @_;
    return CodingAdventures::RubyParser->parse_ruby($src);
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
    my $node = CodingAdventures::RubyParser::ASTNode->new('method_def', []);
    is( $node->rule_name, 'method_def', 'rule_name' );
    is( $node->is_leaf,   0,            'not a leaf' );
    is( ref($node->children), 'ARRAY',  'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'DEF', value => 'def', line => 1, col => 1 };
    my $leaf = CodingAdventures::RubyParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'DEF',   'token type is DEF' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_rb("x = 5");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_rb("x = 5");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_rb("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'x = 5' => sub {
    my $ast = parse_rb("x = 5");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'assignment'), 'assignment node' );
};

subtest 'string assignment' => sub {
    my $ast = parse_rb('name = "Alice"');
    ok( defined find_node($ast, 'assignment'), 'assignment for string' );
};

subtest 'assignment contains expression' => sub {
    my $ast = parse_rb("x = 42");
    ok( defined find_node($ast, 'expression'), 'expression inside assignment' );
};

subtest 'multiple assignments' => sub {
    my $ast = parse_rb("x = 1\ny = 2");
    my $count = count_nodes($ast, 'assignment');
    is( $count, 2, '2 assignment nodes' );
};

# ============================================================================
# Method definitions
# ============================================================================

subtest 'def greet(name) ... end' => sub {
    my $src = "def greet(name)\n  puts name\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'method_def'), 'method_def' );
    ok( defined find_node($ast, 'param_list'), 'param_list' );
    ok( defined find_node($ast, 'body'),       'body' );
};

subtest 'def with no params' => sub {
    my $ast = parse_rb("def noop\n  x = 1\nend\n");
    ok( defined find_node($ast, 'method_def'), 'method_def no params' );
};

subtest 'def with return' => sub {
    my $src = "def double(x)\n  return x + x\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'method_def'),  'method_def' );
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

# ============================================================================
# Class definitions
# ============================================================================

subtest 'class Dog ... end' => sub {
    my $src = "class Dog\n  def bark\n    x = 1\n  end\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'class_def'),  'class_def' );
    ok( defined find_node($ast, 'method_def'), 'nested method_def' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if x > 0 ... end' => sub {
    my $src = "if x > 0\n  return x\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    ok( defined find_node($ast, 'body'),    'body' );
};

subtest 'if/else' => sub {
    my $src = "if x > 0\n  return x\nelse\n  return 0\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    my $body_count = count_nodes($ast, 'body');
    ok( $body_count >= 2, "at least 2 body nodes (got $body_count)" );
};

subtest 'if/elsif/else' => sub {
    my $src = "if x > 0\n  return 1\nelsif x == 0\n  return 0\nelse\n  return -1\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt with elsif' );
    my $body_count = count_nodes($ast, 'body');
    ok( $body_count >= 3, "at least 3 body nodes (got $body_count)" );
};

subtest 'if condition has comparison' => sub {
    my $src = "if x > 0\n  x = 1\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr in condition' );
};

# ============================================================================
# While/until loops
# ============================================================================

subtest 'while x > 0 ... end' => sub {
    my $src = "while x > 0\n  x = x - 1\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'while_stmt'), 'while_stmt' );
    ok( defined find_node($ast, 'body'),       'body' );
};

subtest 'until x == 0 ... end' => sub {
    my $src = "until x == 0\n  x = x - 1\nend\n";
    my $ast = parse_rb($src);
    ok( defined find_node($ast, 'until_stmt'), 'until_stmt' );
};

# ============================================================================
# Return statements
# ============================================================================

subtest 'return value' => sub {
    my $ast = parse_rb("def f\n  return 42\nend\n");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'return with arithmetic' => sub {
    my $ast = parse_rb("def f\n  return 1 + 2\nend\n");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt with expr' );
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr inside return' );
};

# ============================================================================
# Method calls
# ============================================================================

subtest 'puts("hello")' => sub {
    my $ast = parse_rb('puts("hello")');
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'puts with no parens' => sub {
    my $ast = parse_rb('puts "hello"');
    ok( defined find_node($ast, 'method_call_stmt'), 'method_call_stmt' );
};

subtest 'method call with no args' => sub {
    my $ast = parse_rb("noop()");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter' => sub {
    my $ast = parse_rb("r = 1 + 2 * 3");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes for 1+2*3 (got $count)" );
};

subtest 'equality expression ==' => sub {
    my $ast = parse_rb("r = a == b");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for ==' );
};

subtest 'inequality expression !=' => sub {
    my $ast = parse_rb("r = a != b");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for !=' );
};

subtest 'comparison >' => sub {
    my $ast = parse_rb("r = x > 0");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for >' );
};

subtest 'unary negation' => sub {
    my $ast = parse_rb("r = -x");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_rb("r = (2 + 3)");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'class with method' => sub {
    my $src = <<'END_RB';
class Animal
  def speak
    return "..."
  end
end
END_RB
    my $ast = parse_rb($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'class_def'),  'class_def' );
    ok( defined find_node($ast, 'method_def'), 'method_def' );
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'multiple top-level statements' => sub {
    my $ast = parse_rb("x = 1\ny = 2\nz = x + y");
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'assignment'), 3, '3 assignments' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::RubyParser->parse_ruby('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

done_testing;
