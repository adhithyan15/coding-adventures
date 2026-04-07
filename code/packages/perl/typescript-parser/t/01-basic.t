use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TypescriptParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::TypescriptParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse TypeScript and return the root AST node.
sub parse_ts {
    my ($src) = @_;
    return CodingAdventures::TypescriptParser->parse_ts($src);
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
    my $node = CodingAdventures::TypescriptParser::ASTNode->new('var_declaration', []);
    is( $node->rule_name, 'var_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                 'not a leaf' );
    is( ref($node->children), 'ARRAY',       'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'LET', value => 'let', line => 1, col => 1 };
    my $leaf = CodingAdventures::TypescriptParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'LET',   'token type is LET' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_ts("let x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_ts("let x = 5;");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_ts("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Variable declarations
# ============================================================================

subtest 'let x = 5' => sub {
    my $ast = parse_ts("let x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration node' );
};

subtest 'const PI = 3' => sub {
    my $ast = parse_ts("const PI = 3;");
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration for const' );
};

subtest 'var y = "hello"' => sub {
    my $ast = parse_ts('var y = "hello";');
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration for var' );
};

subtest 'multiple declarations' => sub {
    my $ast = parse_ts("let a = 1;\nconst b = 2;\nvar c = 3;");
    my $count = count_nodes($ast, 'var_declaration');
    is( $count, 3, '3 var_declaration nodes' );
};

subtest 'var_declaration contains expression' => sub {
    my $ast = parse_ts("let x = 42;");
    ok( defined find_node($ast, 'expression'), 'expression node inside declaration' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'assignment x = 10' => sub {
    my $ast = parse_ts("x = 10;");
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt node' );
};

subtest 'assignment with string' => sub {
    my $ast = parse_ts('name = "Alice";');
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt' );
};

# ============================================================================
# Expression statements
# ============================================================================

subtest 'expression statement with literal' => sub {
    my $ast = parse_ts("42;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt' );
};

subtest 'expression statement with name' => sub {
    my $ast = parse_ts("x;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt with name' );
};

# ============================================================================
# Function declarations
# ============================================================================

subtest 'function add(a, b) { return a + b; }' => sub {
    my $ast = parse_ts("function add(a, b) { return a + b; }");
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
    ok( defined find_node($ast, 'param_list'),    'param_list' );
    ok( defined find_node($ast, 'block'),         'block' );
    ok( defined find_node($ast, 'return_stmt'),   'return_stmt' );
};

subtest 'function with no parameters' => sub {
    my $ast = parse_ts("function noop() { }");
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if (x > 0) { return x; } else { return 0; }' => sub {
    my $ast = parse_ts(
        "function f(x) { if (x > 0) { return x; } else { return 0; } }"
    );
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    my $block_count = count_nodes($ast, 'block');
    ok( $block_count >= 2, "at least 2 block nodes (got $block_count)" );
};

subtest 'if without else' => sub {
    my $ast = parse_ts("if (x > 0) { x = 1; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt without else' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for (let i = 0; i < 10; i = i + 1) { }' => sub {
    my $ast = parse_ts("for (let i = 0; i < 10; i = i + 1) { }");
    ok( defined find_node($ast, 'for_stmt'),      'for_stmt' );
    ok( defined find_node($ast, 'for_init'),      'for_init' );
    ok( defined find_node($ast, 'for_condition'), 'for_condition' );
    ok( defined find_node($ast, 'for_update'),    'for_update' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter than addition' => sub {
    my $ast = parse_ts("let r = 1 + 2 * 3;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes for 1 + 2 * 3 (got $count)" );
};

subtest 'comparison expression' => sub {
    my $ast = parse_ts("let r = x > 0;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for >' );
};

subtest 'strict equality ===' => sub {
    my $ast = parse_ts("let r = a === b;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for ===' );
};

subtest 'unary negation' => sub {
    my $ast = parse_ts("let r = -x;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'unary logical not' => sub {
    my $ast = parse_ts("let r = !flag;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr for !' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_ts("let r = (2 + 3);");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# Arrow functions
# ============================================================================

subtest 'arrow function (x) => x + 1' => sub {
    my $ast = parse_ts("let f = (x) => x + 1;");
    ok( defined find_node($ast, 'arrow_expr'), 'arrow_expr' );
};

subtest 'arrow function with two params (a, b) => a + b' => sub {
    my $ast = parse_ts("let add = (a, b) => a + b;");
    ok( defined find_node($ast, 'arrow_expr'), 'arrow_expr' );
    ok( defined find_node($ast, 'param_list'), 'param_list' );
};

# ============================================================================
# Function calls
# ============================================================================

subtest 'function call f(a, b)' => sub {
    my $ast = parse_ts("f(a, b);");
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'function call with no args' => sub {
    my $ast = parse_ts("noop();");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'function with if and return' => sub {
    my $src = <<'END_TS';
function abs(x) {
    if (x < 0) {
        return -x;
    } else {
        return x;
    }
}
END_TS
    my $ast = parse_ts($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
    ok( defined find_node($ast, 'if_stmt'),       'if_stmt' );
    ok( defined find_node($ast, 'unary_expr'),    'unary_expr for -x' );
    is( count_nodes($ast, 'return_stmt'), 2, '2 return statements' );
};

subtest 'multiple top-level statements' => sub {
    my $ast = parse_ts("let x = 1;\nlet y = 2;\nx = x + y;");
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'var_declaration'), 2, '2 var_declarations' );
    is( count_nodes($ast, 'assignment_stmt'), 1, '1 assignment_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::TypescriptParser->parse_ts('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

subtest 'missing semicolon raises die' => sub {
    ok(
        dies { CodingAdventures::TypescriptParser->parse_ts('let x = 5') },
        'missing semicolon causes die'
    );
};

# ============================================================================
# Version-aware parsing
# ============================================================================
#
# The optional $version argument threads through to TypescriptLexer and
# selects the correct grammar for that TypeScript release.
# No-version calls continue to work (backward compatible default).

subtest 'parse_ts with no version (backward compatible)' => sub {
    my $ast = CodingAdventures::TypescriptParser->parse_ts('let x = 5;');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'new + parse with no version (backward compatible)' => sub {
    my $parser = CodingAdventures::TypescriptParser->new('let x = 5;');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_ts with ts1.0 produces program node' => sub {
    my $ast = CodingAdventures::TypescriptParser->parse_ts('var x = 1;', 'ts1.0');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_ts with ts2.0 produces program node' => sub {
    my $ast = CodingAdventures::TypescriptParser->parse_ts('let x = 1;', 'ts2.0');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_ts with ts5.0 produces program node' => sub {
    my $ast = CodingAdventures::TypescriptParser->parse_ts('const x = 1;', 'ts5.0');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_ts with ts5.8 produces program node' => sub {
    my $ast = CodingAdventures::TypescriptParser->parse_ts('let x = 1;', 'ts5.8');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'new($source, $version) with ts5.0' => sub {
    my $parser = CodingAdventures::TypescriptParser->new('let x = 1;', 'ts5.0');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::TypescriptParser->parse_ts('let x = 1;', 'ts99.0') },
        'unknown version ts99.0 causes die'
    );
};

done_testing;
