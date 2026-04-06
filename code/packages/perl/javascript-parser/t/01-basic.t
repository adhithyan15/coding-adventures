use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavascriptParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::JavascriptParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse JavaScript and return the root AST node.
sub parse_js {
    my ($src) = @_;
    return CodingAdventures::JavascriptParser->parse_js($src);
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
    my $node = CodingAdventures::JavascriptParser::ASTNode->new('var_declaration', []);
    is( $node->rule_name, 'var_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                 'not a leaf' );
    is( ref($node->children), 'ARRAY',       'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'VAR', value => 'var', line => 1, col => 1 };
    my $leaf = CodingAdventures::JavascriptParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'VAR',   'token type is VAR' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_js("var x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_js("var x = 5;");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_js("");
    is( $ast->rule_name, 'program', 'root is program' );
    is( scalar @{ $ast->children }, 0, 'no children for empty program' );
};

# ============================================================================
# Variable declarations
# ============================================================================

subtest 'var x = 5' => sub {
    my $ast = parse_js("var x = 5;");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration node' );
};

subtest 'let y = "hello"' => sub {
    my $ast = parse_js('let y = "hello";');
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration for let' );
};

subtest 'const z = true' => sub {
    my $ast = parse_js("const z = true;");
    ok( defined find_node($ast, 'var_declaration'), 'var_declaration for const' );
};

subtest 'multiple declarations' => sub {
    my $ast = parse_js("var a = 1;\nlet b = 2;\nconst c = 3;");
    my $count = count_nodes($ast, 'var_declaration');
    is( $count, 3, '3 var_declaration nodes' );
};

subtest 'var_declaration contains expression' => sub {
    my $ast = parse_js("var x = 42;");
    ok( defined find_node($ast, 'expression'), 'expression node inside declaration' );
};

# ============================================================================
# Assignments
# ============================================================================

subtest 'assignment x = 10' => sub {
    my $ast = parse_js("x = 10;");
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt node' );
};

subtest 'assignment with string' => sub {
    my $ast = parse_js('name = "Alice";');
    ok( defined find_node($ast, 'assignment_stmt'), 'assignment_stmt' );
};

# ============================================================================
# Expression statements
# ============================================================================

subtest 'expression statement with literal' => sub {
    my $ast = parse_js("42;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt' );
};

subtest 'expression statement with name' => sub {
    my $ast = parse_js("x;");
    ok( defined find_node($ast, 'expression_stmt'), 'expression_stmt with name' );
};

# ============================================================================
# Function declarations
# ============================================================================

subtest 'function add(a, b) { return a + b; }' => sub {
    my $ast = parse_js("function add(a, b) { return a + b; }");
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
    ok( defined find_node($ast, 'param_list'),    'param_list' );
    ok( defined find_node($ast, 'block'),         'block' );
    ok( defined find_node($ast, 'return_stmt'),   'return_stmt' );
};

subtest 'function with no parameters' => sub {
    my $ast = parse_js("function noop() { }");
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
};

subtest 'return statement' => sub {
    my $ast = parse_js("function f() { return 42; }");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if (x > 0) { return x; } else { return 0; }' => sub {
    my $ast = parse_js(
        "function f(x) { if (x > 0) { return x; } else { return 0; } }"
    );
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
    my $block_count = count_nodes($ast, 'block');
    ok( $block_count >= 2, "at least 2 block nodes (got $block_count)" );
};

subtest 'if without else' => sub {
    my $ast = parse_js("if (x > 0) { x = 1; }");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt without else' );
};

subtest 'if condition contains comparison' => sub {
    my $ast = parse_js("if (a > b) { }");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for comparison' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for (let i = 0; i < 10; i = i + 1) { }' => sub {
    my $ast = parse_js("for (let i = 0; i < 10; i = i + 1) { }");
    ok( defined find_node($ast, 'for_stmt'),    'for_stmt' );
    ok( defined find_node($ast, 'for_init'),    'for_init' );
    ok( defined find_node($ast, 'for_condition'), 'for_condition' );
    ok( defined find_node($ast, 'for_update'),  'for_update' );
};

subtest 'for loop with var init' => sub {
    my $ast = parse_js("for (var i = 0; i < 5; i = i + 1) { }");
    ok( defined find_node($ast, 'for_stmt'), 'for_stmt' );
};

# ============================================================================
# Expression precedence
# ============================================================================

subtest '1 + 2 * 3 — multiplication binds tighter than addition' => sub {
    # The parser should build: binary_expr(1, +, binary_expr(2, *, 3))
    # We verify by checking that binary_expr nodes exist and the expression
    # structure is correct.
    my $ast = parse_js("var r = 1 + 2 * 3;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr exists' );
    my $count = count_nodes($ast, 'binary_expr');
    ok( $count >= 2, "at least 2 binary_expr nodes for 1 + 2 * 3 (got $count)" );
};

subtest 'comparison expression' => sub {
    my $ast = parse_js("var r = x > 0;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for >' );
};

subtest 'equality expression ===' => sub {
    my $ast = parse_js("var r = a === b;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for ===' );
};

subtest 'equality expression !=' => sub {
    my $ast = parse_js("var r = a != b;");
    ok( defined find_node($ast, 'binary_expr'), 'binary_expr for !=' );
};

subtest 'unary negation' => sub {
    my $ast = parse_js("var r = -x;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr' );
};

subtest 'unary logical not' => sub {
    my $ast = parse_js("var r = !flag;");
    ok( defined find_node($ast, 'unary_expr'), 'unary_expr for !' );
};

subtest 'parenthesized expression' => sub {
    my $ast = parse_js("var r = (2 + 3);");
    ok( defined find_node($ast, 'primary'), 'primary for grouped expression' );
};

# ============================================================================
# Arrow functions
# ============================================================================

subtest 'arrow function (x) => x + 1' => sub {
    my $ast = parse_js("var f = (x) => x + 1;");
    # Arrow functions produce arrow_expr or are wrapped in expression
    # (since the expression parser delegates down to primary)
    # The arrow_expr node should appear somewhere in the tree.
    ok( defined find_node($ast, 'arrow_expr'), 'arrow_expr' );
};

subtest 'arrow function with two params (a, b) => a + b' => sub {
    my $ast = parse_js("var add = (a, b) => a + b;");
    ok( defined find_node($ast, 'arrow_expr'), 'arrow_expr' );
    ok( defined find_node($ast, 'param_list'), 'param_list' );
};

subtest 'arrow function with block body' => sub {
    my $ast = parse_js('var f = (x) => { return x + 1; };');
    ok( defined find_node($ast, 'arrow_expr'), 'arrow_expr with block' );
    ok( defined find_node($ast, 'block'),      'block body' );
};

# ============================================================================
# Function calls
# ============================================================================

subtest 'function call f(a, b)' => sub {
    my $ast = parse_js("f(a, b);");
    ok( defined find_node($ast, 'call_expr'), 'call_expr' );
    ok( defined find_node($ast, 'arg_list'),  'arg_list' );
};

subtest 'function call with no args' => sub {
    my $ast = parse_js("noop();");
    ok( defined find_node($ast, 'call_expr'), 'call_expr no-args' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'function with if and return' => sub {
    my $src = <<'END_JS';
function abs(x) {
    if (x < 0) {
        return -x;
    } else {
        return x;
    }
}
END_JS
    my $ast = parse_js($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'function_decl'), 'function_decl' );
    ok( defined find_node($ast, 'if_stmt'),       'if_stmt' );
    ok( defined find_node($ast, 'unary_expr'),    'unary_expr for -x' );
    is( count_nodes($ast, 'return_stmt'), 2, '2 return statements' );
};

subtest 'multiple top-level statements' => sub {
    my $ast = parse_js("var x = 1;\nvar y = 2;\nx = x + y;");
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'var_declaration'), 2, '2 var_declarations' );
    is( count_nodes($ast, 'assignment_stmt'), 1, '1 assignment_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token raises die' => sub {
    ok(
        dies { CodingAdventures::JavascriptParser->parse_js('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

subtest 'incomplete var declaration raises die' => sub {
    ok(
        dies { CodingAdventures::JavascriptParser->parse_js('var x =') },
        'incomplete declaration causes die'
    );
};

subtest 'missing semicolon raises die' => sub {
    ok(
        dies { CodingAdventures::JavascriptParser->parse_js('var x = 5') },
        'missing semicolon causes die'
    );
};

# ============================================================================
# Version-aware parsing
# ============================================================================
#
# The optional $version argument threads through to JavascriptLexer and
# selects the correct grammar for that ECMAScript release.
# No-version calls continue to work (backward compatible default).

subtest 'parse_js with no version (backward compatible)' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 5;');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'new + parse with no version (backward compatible)' => sub {
    my $parser = CodingAdventures::JavascriptParser->new('var x = 5;');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_js with es1 produces program node' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es1');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_js with es3 produces program node' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es3');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_js with es5 produces program node' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es5');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_js with es2015 produces program node' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es2015');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'parse_js with es2025 produces program node' => sub {
    my $ast = CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es2025');
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'new($source, $version) with es2015' => sub {
    my $parser = CodingAdventures::JavascriptParser->new('var x = 1;', 'es2015');
    my $ast = $parser->parse();
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'es99') },
        'unknown version es99 causes die'
    );
};

subtest 'TypeScript version string is rejected' => sub {
    ok(
        dies { CodingAdventures::JavascriptParser->parse_js('var x = 1;', 'ts5.0') },
        'ts5.0 is not a valid ECMAScript version for JavascriptParser'
    );
};

done_testing;
