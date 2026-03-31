use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::StarlarkParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::StarlarkParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse Starlark and return the root AST node.
sub parse_starlark {
    my ($src) = @_;
    return CodingAdventures::StarlarkParser->parse_starlark($src);
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
    my $node = CodingAdventures::StarlarkParser::ASTNode->new('assign_stmt', []);
    is( $node->rule_name, 'assign_stmt', 'rule_name' );
    is( $node->is_leaf,   0,             'not a leaf' );
    is( ref($node->children), 'ARRAY',   'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NAME', value => 'x', line => 1, col => 1 };
    my $leaf = CodingAdventures::StarlarkParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token', 'rule_name is token' );
    is( $leaf->is_leaf,       1,       'is_leaf returns 1' );
    is( $leaf->token->{type}, 'NAME',  'token type is NAME' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_starlark("x = 1\n");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_starlark("x = 1\n");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

subtest 'empty program is valid' => sub {
    my $ast = parse_starlark("");
    is( $ast->rule_name, 'program', 'root is program' );
};

# ============================================================================
# Simple assignments
# ============================================================================

subtest 'x = 1' => sub {
    my $ast = parse_starlark("x = 1\n");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'assign_stmt'), 'assign_stmt node' );
};

subtest "x = 'hello'" => sub {
    my $ast = parse_starlark("x = 'hello'\n");
    ok( defined find_node($ast, 'assign_stmt'), 'assign_stmt for string' );
};

subtest 'x = True' => sub {
    my $ast = parse_starlark("x = True\n");
    ok( defined find_node($ast, 'assign_stmt'), 'assign_stmt for True' );
};

subtest 'multiple assignments' => sub {
    my $ast = parse_starlark("x = 1\ny = 2\n");
    my $count = count_nodes($ast, 'assign_stmt');
    ok( $count >= 2, "at least 2 assign_stmt nodes (got $count)" );
};

subtest 'augmented assignment x += 1' => sub {
    my $ast = parse_starlark("x += 1\n");
    ok( defined find_node($ast, 'assign_stmt'), 'assign_stmt for +=' );
    ok( defined find_node($ast, 'augmented_assign_op'), 'augmented_assign_op node' );
};

subtest 'tuple unpacking a, b = 1, 2' => sub {
    my $ast = parse_starlark("a, b = 1, 2\n");
    ok( defined find_node($ast, 'assign_stmt'), 'assign_stmt for tuple unpack' );
};

# ============================================================================
# Function calls
# ============================================================================

subtest "print('hello')" => sub {
    my $ast = parse_starlark("print('hello')\n");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'f(x, y)' => sub {
    my $ast = parse_starlark("f(x, y)\n");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'BUILD-style cc_library call' => sub {
    my $ast = parse_starlark('cc_library(name = "foo", srcs = ["foo.cc"])' . "\n");
    is( $ast->rule_name, 'program', 'BUILD rule parses' );
};

subtest 'load statement' => sub {
    my $ast = parse_starlark('load("//rules/python.bzl", "py_library")' . "\n");
    ok( defined find_node($ast, 'load_stmt'), 'load_stmt node' );
};

subtest 'load with alias' => sub {
    my $ast = parse_starlark('load("//rules.bzl", lib = "py_library")' . "\n");
    ok( defined find_node($ast, 'load_stmt'), 'load_stmt with alias' );
};

# ============================================================================
# Function definitions
# ============================================================================

subtest 'def foo(x): return x + 1' => sub {
    my $ast = parse_starlark("def foo(x):\n    return x + 1\n");
    ok( defined find_node($ast, 'def_stmt'),    'def_stmt' );
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'def with no parameters' => sub {
    my $ast = parse_starlark("def noop():\n    pass\n");
    ok( defined find_node($ast, 'def_stmt'),  'def_stmt' );
    ok( defined find_node($ast, 'pass_stmt'), 'pass_stmt' );
};

subtest 'def with default parameter' => sub {
    my $ast = parse_starlark("def greet(name, greeting=\"Hello\"):\n    return greeting\n");
    ok( defined find_node($ast, 'def_stmt'), 'def_stmt with default' );
    ok( defined find_node($ast, 'parameters'), 'parameters node' );
};

subtest 'def with multiple body statements' => sub {
    my $ast = parse_starlark("def add(a, b):\n    c = a + b\n    return c\n");
    ok( defined find_node($ast, 'def_stmt'), 'def_stmt' );
    is( count_nodes($ast, 'assign_stmt'), 1, '1 assign inside def' );
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'return statement' => sub {
    my $ast = parse_starlark("def f(x):\n    return x\n");
    ok( defined find_node($ast, 'return_stmt'), 'return_stmt' );
};

subtest 'pass statement' => sub {
    my $ast = parse_starlark("def todo():\n    pass\n");
    ok( defined find_node($ast, 'pass_stmt'), 'pass_stmt' );
};

# ============================================================================
# List literals
# ============================================================================

subtest '[1, 2, 3]' => sub {
    my $ast = parse_starlark("x = [1, 2, 3]\n");
    ok( defined find_node($ast, 'list_expr'), 'list_expr' );
};

subtest 'empty list []' => sub {
    my $ast = parse_starlark("x = []\n");
    ok( defined find_node($ast, 'list_expr'), 'list_expr for []' );
};

subtest 'list of strings' => sub {
    my $ast = parse_starlark('srcs = ["a.cc", "b.cc"]' . "\n");
    ok( defined find_node($ast, 'list_expr'), 'list_expr for strings' );
};

# ============================================================================
# Dict literals
# ============================================================================

subtest "{'key': 'value'}" => sub {
    my $ast = parse_starlark("d = {'key': 'value'}\n");
    ok( defined find_node($ast, 'dict_expr'), 'dict_expr' );
};

subtest 'empty dict {}' => sub {
    my $ast = parse_starlark("d = {}\n");
    ok( defined find_node($ast, 'dict_expr'), 'dict_expr for {}' );
};

subtest 'multi-entry dict' => sub {
    my $ast = parse_starlark("d = {'a': 1, 'b': 2}\n");
    ok( defined find_node($ast, 'dict_expr'), 'dict_expr for multi-entry' );
    is( count_nodes($ast, 'dict_entry'), 2, '2 dict_entry nodes' );
};

# ============================================================================
# If/else statements
# ============================================================================

subtest 'basic if' => sub {
    my $ast = parse_starlark("if x > 0:\n    y = 1\n");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt' );
};

subtest 'if/else' => sub {
    my $ast = parse_starlark("if x > 0:\n    y = 1\nelse:\n    y = 0\n");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt with else' );
};

subtest 'if/elif/else' => sub {
    my $ast = parse_starlark(
        "if score >= 90:\n    grade = 'A'\nelif score >= 80:\n    grade = 'B'\nelse:\n    grade = 'F'\n"
    );
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt with elif' );
};

subtest 'if with pass body' => sub {
    my $ast = parse_starlark("if True:\n    pass\n");
    ok( defined find_node($ast, 'if_stmt'), 'if_stmt with pass' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for item in items' => sub {
    my $ast = parse_starlark("for item in items:\n    process(item)\n");
    ok( defined find_node($ast, 'for_stmt'),   'for_stmt' );
    ok( defined find_node($ast, 'loop_vars'),  'loop_vars' );
};

subtest 'for with tuple unpacking k, v' => sub {
    my $ast = parse_starlark("for k, v in pairs:\n    print(k)\n");
    ok( defined find_node($ast, 'for_stmt'), 'for_stmt with tuple unpack' );
};

subtest 'for over range call' => sub {
    my $ast = parse_starlark("for i in range(10):\n    total += i\n");
    ok( defined find_node($ast, 'for_stmt'), 'for_stmt over range' );
};

subtest 'break statement' => sub {
    my $ast = parse_starlark("for x in lst:\n    break\n");
    ok( defined find_node($ast, 'break_stmt'), 'break_stmt' );
};

subtest 'continue statement' => sub {
    my $ast = parse_starlark("for x in lst:\n    continue\n");
    ok( defined find_node($ast, 'continue_stmt'), 'continue_stmt' );
};

# ============================================================================
# BUILD file patterns
# ============================================================================

subtest 'cc_library BUILD rule' => sub {
    my $ast = parse_starlark('cc_library(name = "foo", srcs = ["foo.cc"])' . "\n");
    is( $ast->rule_name, 'program', 'BUILD rule parses as program' );
};

subtest 'multi-line BUILD rule' => sub {
    my $src = <<'END';
cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
    deps = [":my_lib"],
)
END
    my $ast = parse_starlark($src);
    is( $ast->rule_name, 'program', 'multi-line BUILD rule' );
};

subtest 'multiple BUILD rules' => sub {
    my $src = "cc_library(name = \"foo\", srcs = [\"foo.cc\"])\ncc_binary(name = \"bar\", deps = [\":foo\"])\n";
    my $ast = parse_starlark($src);
    is( $ast->rule_name, 'program', 'multiple BUILD rules' );
};

subtest 'load + BUILD rule' => sub {
    my $src = "load(\"//rules:defs.bzl\", \"my_rule\")\nmy_rule(name = \"target\")\n";
    my $ast = parse_starlark($src);
    is( $ast->rule_name, 'program', 'load + BUILD rule' );
    ok( defined find_node($ast, 'load_stmt'), 'load_stmt present' );
};

# ============================================================================
# Expressions
# ============================================================================

subtest 'arithmetic a + b * c' => sub {
    my $ast = parse_starlark("x = a + b * c\n");
    is( $ast->rule_name, 'program', 'arithmetic parses' );
};

subtest 'comparison a == b' => sub {
    my $ast = parse_starlark("x = a == b\n");
    is( $ast->rule_name, 'program', 'comparison parses' );
};

subtest 'boolean or: a or b' => sub {
    my $ast = parse_starlark("x = a or b\n");
    ok( defined find_node($ast, 'or_expr'), 'or_expr' );
};

subtest 'boolean and: a and b' => sub {
    my $ast = parse_starlark("x = a and b\n");
    ok( defined find_node($ast, 'and_expr'), 'and_expr' );
};

subtest 'ternary: a if cond else b' => sub {
    my $ast = parse_starlark("x = a if cond else b\n");
    is( $ast->rule_name, 'program', 'ternary parses' );
};

subtest 'lambda: lambda x: x + 1' => sub {
    my $ast = parse_starlark("f = lambda x: x + 1\n");
    ok( defined find_node($ast, 'lambda_expr'), 'lambda_expr' );
};

subtest 'attribute access: obj.attr' => sub {
    my $ast = parse_starlark("x = obj.attr\n");
    is( $ast->rule_name, 'program', 'attribute access parses' );
};

subtest 'string concatenation' => sub {
    my $ast = parse_starlark('greeting = "Hello, " + name' . "\n");
    is( $ast->rule_name, 'program', 'string concat parses' );
};

# ============================================================================
# Complex programs
# ============================================================================

subtest 'function with if and return' => sub {
    my $src = <<'END';
def abs_val(x):
    if x < 0:
        return -x
    else:
        return x
END
    my $ast = parse_starlark($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'def_stmt'),    'def_stmt' );
    ok( defined find_node($ast, 'if_stmt'),     'if_stmt' );
    is( count_nodes($ast, 'return_stmt'), 2, '2 return statements' );
};

subtest 'BUILD file with load and rules' => sub {
    my $src = <<'END';
load("//tools:defs.bzl", "cc_lib")

cc_lib(
    name = "mylib",
    srcs = ["mylib.cc", "mylib.h"],
    deps = ["//external:dep"],
)
END
    my $ast = parse_starlark($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'load_stmt'), 'load_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected token causes die' => sub {
    ok(
        dies { CodingAdventures::StarlarkParser->parse_starlark('@@@ GARBAGE') },
        'garbage input causes die'
    );
};

done_testing;
