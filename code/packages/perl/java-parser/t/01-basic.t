use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavaParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::JavaParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse Java and return the root AST node.
sub parse_java {
    my ($src, $version) = @_;
    return CodingAdventures::JavaParser->parse_java($src, $version);
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
    my $node = CodingAdventures::JavaParser::ASTNode->new('var_declaration', []);
    is( $node->rule_name, 'var_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                 'not a leaf' );
    is( ref($node->children), 'ARRAY',       'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NAME', value => 'int', line => 1, col => 1 };
    my $leaf = CodingAdventures::JavaParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'NAME',  'token type is NAME' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_java("int x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_java("int x = 5;");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_java("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Variable declarations
# ============================================================================

subtest 'int x = 5' => sub {
    my $ast = parse_java("int x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration node' );
};

subtest 'multiple declarations' => sub {
    my $ast = parse_java("int a = 1;\nint b = 2;\nint c = 3;");
    my $count = count_nodes($ast, 'var_declaration');
    is( $count, 3, '3 var_declaration nodes' );
};

subtest 'var_declaration contains expression' => sub {
    my $ast = parse_java("int x = 42;");
    ok( defined find_node($ast, 'expression'), 'expression node inside declaration' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'assignment x = 10' => sub {
    my $ast = parse_java("x = 10;");
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt node' );
};

# ============================================================================
# Expression statements
# ============================================================================

subtest 'expression statement with literal' => sub {
    my $ast = parse_java("42;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter' => sub {
    my $ast = parse_java("int r = 1 + 2 * 3;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes (got $count)" );
};

subtest 'unary negation' => sub {
    my $ast = parse_java("int r = -5;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_java("int r = (2 + 3);");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# Method calls
# ============================================================================

subtest 'method call f(a, b)' => sub {
    my $ast = parse_java("f(a, b);");
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'method call with no args' => sub {
    my $ast = parse_java("noop();");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if with else' => sub {
    my $ast = parse_java("if (x > 0) { x = 1; } else { x = 0; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
};

subtest 'if without else' => sub {
    my $ast = parse_java("if (x > 0) { x = 1; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt without else' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for loop with typed init' => sub {
    my $ast = parse_java("for (int i = 0; i < 10; i = i + 1) { }");
    ok( defined find_node($ast, 'for_stmt'),      'for_stmt' );
    ok( defined find_node($ast, 'for_init'),       'for_init' );
    ok( defined find_node($ast, 'for_condition'),  'for_condition' );
    ok( defined find_node($ast, 'for_update'),     'for_update' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'multiple top-level statements' => sub {
    my $ast = parse_java("int x = 1;\nint y = 2;\nx = x + y;");
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'var_declaration'), 2, '2 var_declarations' );
    is( count_nodes($ast, 'assignment_stmt'), 1, '1 assignment_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::JavaParser->parse_java('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

subtest 'missing semicolon raises die' => sub {
    ok(
        dies { CodingAdventures::JavaParser->parse_java('int x = 5') },
        'missing semicolon causes die'
    );
};

# ============================================================================
# Version-aware parsing
# ============================================================================

subtest 'parse_java with no version (default)' => sub {
    my $ast = CodingAdventures::JavaParser->parse_java('int x = 5;');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_java with version 1.0' => sub {
    my $ast = CodingAdventures::JavaParser->parse_java('int x = 1;', '1.0');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_java with version 8' => sub {
    my $ast = CodingAdventures::JavaParser->parse_java('int x = 1;', '8');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_java with version 21' => sub {
    my $ast = CodingAdventures::JavaParser->parse_java('int x = 1;', '21');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'new($source, $version) with version 17' => sub {
    my $parser = CodingAdventures::JavaParser->new('int x = 1;', '17');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::JavaParser->parse_java('int x = 1;', '99') },
        'unknown version 99 causes die'
    );
};

subtest 'invalid version string is rejected' => sub {
    ok(
        dies { CodingAdventures::JavaParser->parse_java('int x = 1;', 'java21') },
        'java21 is not a valid Java version'
    );
};

done_testing;
