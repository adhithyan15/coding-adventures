use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CSharpParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::CSharpParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse C# and return the root AST node.
sub parse_cs {
    my ($src, $version) = @_;
    return CodingAdventures::CSharpParser->parse_csharp($src, $version);
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
    my $node = CodingAdventures::CSharpParser::ASTNode->new('var_declaration', []);
    is( $node->rule_name, 'var_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                 'not a leaf' );
    is( ref($node->children), 'ARRAY',       'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NAME', value => 'int', line => 1, col => 1 };
    my $leaf = CodingAdventures::CSharpParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'NAME',  'token type is NAME' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_cs("int x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_cs("int x = 5;");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_cs("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Basic C# class declaration
# ============================================================================
#
# While this parser handles statement-level constructs rather than full class
# declarations, a simple variable declaration inside an implied scope is the
# canonical first test for C# parsing.

subtest 'basic C# variable declaration parses correctly' => sub {
    my $ast = parse_cs("int x = 42;");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration node found' );
};

# ============================================================================
# Variable declarations
# ============================================================================

subtest 'int x = 5' => sub {
    my $ast = parse_cs("int x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration node' );
};

subtest 'multiple declarations' => sub {
    my $ast = parse_cs("int a = 1;\nint b = 2;\nint c = 3;");
    my $count = count_nodes($ast, 'var_declaration');
    is( $count, 3, '3 var_declaration nodes' );
};

subtest 'var_declaration contains expression' => sub {
    my $ast = parse_cs("int x = 42;");
    ok( defined find_node($ast, 'expression'), 'expression node inside declaration' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'assignment x = 10' => sub {
    my $ast = parse_cs("x = 10;");
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt node' );
};

# ============================================================================
# Expression statements
# ============================================================================

subtest 'expression statement with literal' => sub {
    my $ast = parse_cs("42;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter' => sub {
    my $ast = parse_cs("int r = 1 + 2 * 3;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes (got $count)" );
};

subtest 'unary negation' => sub {
    my $ast = parse_cs("int r = -5;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_cs("int r = (2 + 3);");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# C# null-coalescing operator ??
# ============================================================================
#
# The ?? operator is unique to C# (not available in Java). It returns the
# left operand if non-null, otherwise the right operand.
#
# Example:  string name = input ?? "default";
#
# In our expression grammar, ?? binds lower than == and comparison, so:
#   a == b ?? c   parses as   a == (b ?? c)  — but ?? is lower, so
#   a ?? b == c   parses as   a ?? (b == c)
# We test that the null_coalesce node is present in the AST.

subtest 'null-coalescing operator ?? produces null_coalesce node' => sub {
    my $ast = parse_cs("a ?? b;", '2.0');
    ok( defined find_node($ast, 'null_coalesce'), 'null_coalesce node found' );
};

# ============================================================================
# Method calls
# ============================================================================

subtest 'method call f(a, b)' => sub {
    my $ast = parse_cs("f(a, b);");
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'method call with no args' => sub {
    my $ast = parse_cs("noop();");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if with else' => sub {
    my $ast = parse_cs("if (x > 0) { x = 1; } else { x = 0; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
};

subtest 'if without else' => sub {
    my $ast = parse_cs("if (x > 0) { x = 1; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt without else' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for loop with typed init' => sub {
    my $ast = parse_cs("for (int i = 0; i < 10; i = i + 1) { }");
    ok( defined find_node($ast, 'for_stmt'),      'for_stmt' );
    ok( defined find_node($ast, 'for_init'),       'for_init' );
    ok( defined find_node($ast, 'for_condition'),  'for_condition' );
    ok( defined find_node($ast, 'for_update'),     'for_update' );
};

# ============================================================================
# Return statements
# ============================================================================

subtest 'return with expression' => sub {
    my $ast = parse_cs("return x + 1;");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'return without expression' => sub {
    my $ast = parse_cs("return;");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt without value' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'multiple top-level statements' => sub {
    my $ast = parse_cs("int x = 1;\nint y = 2;\nx = x + y;");
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'var_declaration'), 2, '2 var_declarations' );
    is( count_nodes($ast, 'assignment_stmt'), 1, '1 assignment_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::CSharpParser->parse_csharp('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

subtest 'missing semicolon raises die' => sub {
    ok(
        dies { CodingAdventures::CSharpParser->parse_csharp('int x = 5') },
        'missing semicolon causes die'
    );
};

# ============================================================================
# Version-aware parsing — all 12 C# versions must not die
# ============================================================================

my @ALL_VERSIONS = qw(1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0);

subtest 'parse_csharp with no version (default)' => sub {
    my $ast = CodingAdventures::CSharpParser->parse_csharp('int x = 5;');
    is( $ast->rule_name, 'program', 'root is program' );
};

for my $ver (@ALL_VERSIONS) {
    subtest "parse_csharp with version $ver" => sub {
        my $ast = CodingAdventures::CSharpParser->parse_csharp('int x = 1;', $ver);
        is( $ast->rule_name, 'program', "root is program for version $ver" );
    };
}

subtest 'new($source, $version) with version 8.0' => sub {
    my $parser = CodingAdventures::CSharpParser->new('int x = 1;', '8.0');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::CSharpParser->parse_csharp('int x = 1;', '99') },
        'unknown version 99 causes die'
    );
};

subtest 'invalid version string is rejected' => sub {
    ok(
        dies { CodingAdventures::CSharpParser->parse_csharp('int x = 1;', 'csharp12') },
        'csharp12 is not a valid C# version'
    );
};

done_testing;
